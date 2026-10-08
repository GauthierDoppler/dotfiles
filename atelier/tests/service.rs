mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

const LABELS: &[&str] = &[
    "com.theodo.cc-tap.dashboard",
    "com.theodo.cc-tap.proxy",
    "com.theodo.cc-tap.update",
    "com.github.gauthierdoppler.md-preview",
];

const FAKE_LAUNCHCTL: &str = r#"#!/bin/sh
echo "launchctl $*" >>"$FAKE_LOG"
state="$FAKE_STATE/launchd"
mkdir -p "$state"
case "$1" in
  print)
    [ -e "$state/${2##*/}" ] || exit 113
    running=running
    [ -s "$state/${2##*/}" ] && read -r running <"$state/${2##*/}"
    printf '%s = {\n\tstate = %s\n\tjob state = exited\n}\n' "$2" "$running" ;;
  bootstrap) touch "$state/$(basename "$3" .plist)" ;;
  bootout) [ -e "$state/${2##*/}" ] && rm "$state/${2##*/}" ;;
  kickstart) [ -e "$state/${2##*/}" ] ;;
esac
"#;

const FAKE_SYSTEMCTL: &str = r#"#!/bin/sh
echo "systemctl $*" >>"$FAKE_LOG"
case "$2" in
  is-active) echo active ;;
esac
"#;

const FAKE_LOGINCTL: &str = r#"#!/bin/sh
echo "loginctl $*" >>"$FAKE_LOG"
case "$1" in
  show-user) cat "$FAKE_STATE/linger" 2>/dev/null || echo no ;;
esac
"#;

struct Machine {
    home: tempfile::TempDir,
    fakes: tempfile::TempDir,
}

impl Machine {
    fn new() -> Self {
        let machine = Machine {
            home: tempfile::tempdir().unwrap(),
            fakes: tempfile::tempdir().unwrap(),
        };
        fs::create_dir(machine.fakes.path().join("bin")).unwrap();
        fs::create_dir(machine.fakes.path().join("state")).unwrap();
        for (name, script) in [
            ("launchctl", FAKE_LAUNCHCTL),
            ("systemctl", FAKE_SYSTEMCTL),
            ("loginctl", FAKE_LOGINCTL),
        ] {
            let path = machine.fakes.path().join("bin").join(name);
            fs::write(&path, script).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        machine
    }

    fn home(&self, relative: &str) -> PathBuf {
        self.home.path().join(relative)
    }

    fn agents(&self) -> PathBuf {
        self.home("Library/LaunchAgents")
    }

    fn units(&self) -> PathBuf {
        self.home(".config/systemd/user")
    }

    fn command(&self, args: &[&str]) -> Command {
        let path = format!(
            "{}:{}",
            self.fakes.path().join("bin").display(),
            std::env::var("PATH").unwrap()
        );
        let mut command = Command::new(env!("CARGO_BIN_EXE_atelier"));
        command
            .arg("service")
            .args(args)
            .arg("--repo")
            .arg(common::repo())
            .env("HOME", self.home.path())
            .env_remove("XDG_CONFIG_HOME")
            .env("USER", "alice")
            .env("PATH", path)
            .env("FAKE_LOG", self.fakes.path().join("log"))
            .env("FAKE_STATE", self.fakes.path().join("state"));
        command
    }

    fn service(&self, args: &[&str]) -> Output {
        self.command(args).output().expect("atelier runs")
    }

    fn ok(&self, args: &[&str]) -> String {
        let output = self.service(args);
        assert!(
            output.status.success(),
            "atelier service {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    fn calls(&self) -> Vec<String> {
        let log = self.fakes.path().join("log");
        let calls = fs::read_to_string(&log)
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect();
        let _ = fs::remove_file(log);
        calls
    }

    fn set_linger(&self, value: &str) {
        fs::write(self.fakes.path().join("state/linger"), value).unwrap();
    }
}

fn uid() -> String {
    let output = Command::new("id").arg("-u").output().unwrap();
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

#[test]
fn launchd_agents_are_generated_from_the_description() {
    let machine = Machine::new();
    machine.ok(&["install", "--system", "launchd"]);

    for label in LABELS {
        let plist = fs::read_to_string(machine.agents().join(format!("{label}.plist")))
            .unwrap_or_else(|_| panic!("{label}.plist written"));
        insta::assert_snapshot!(format!("launchd__{label}"), plist);
    }
}

#[test]
fn launchd_install_bootstraps_every_agent_then_only_kickstarts_on_a_rerun() {
    let machine = Machine::new();
    let uid = uid();
    let dashboard = format!("gui/{uid}/com.theodo.cc-tap.dashboard");
    let plist = machine.agents().join("com.theodo.cc-tap.dashboard.plist");

    machine.ok(&["install", "--system", "launchd"]);
    let first = machine.calls();
    assert!(first.contains(&format!(
        "launchctl bootstrap gui/{uid} {}",
        plist.display()
    )));
    assert!(first.contains(&format!("launchctl kickstart {dashboard}")));
    let written = fs::metadata(&plist).unwrap().modified().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(20));
    let output = machine.ok(&["install", "--system", "launchd"]);
    let second = machine.calls();
    assert!(!second
        .iter()
        .any(|call| call.contains("bootstrap") || call.contains("bootout")));
    assert!(second.contains(&format!("launchctl kickstart {dashboard}")));
    assert_eq!(fs::metadata(&plist).unwrap().modified().unwrap(), written);
    assert!(output.contains("agent ok: com.theodo.cc-tap.dashboard"));
}

#[test]
fn launchd_install_reloads_only_the_agent_whose_plist_changed() {
    let machine = Machine::new();
    let uid = uid();
    machine.ok(&["install", "--system", "launchd"]);
    machine.calls();
    let plist = machine.agents().join("com.theodo.cc-tap.proxy.plist");
    fs::write(&plist, "stale").unwrap();

    machine.ok(&["install", "--system", "launchd"]);
    let reloads: Vec<String> = machine
        .calls()
        .into_iter()
        .filter(|call| call.contains("bootstrap") || call.contains("bootout"))
        .collect();
    assert_eq!(
        reloads,
        [
            format!("launchctl bootout gui/{uid}/com.theodo.cc-tap.proxy"),
            format!("launchctl bootstrap gui/{uid} {}", plist.display()),
        ]
    );
    assert!(fs::read_to_string(&plist)
        .unwrap()
        .contains("cc-tap-service\" proxy"));
}

#[test]
fn launchd_install_loads_an_unchanged_agent_that_is_not_loaded() {
    let machine = Machine::new();
    let uid = uid();
    machine.ok(&["install", "--system", "launchd"]);
    fs::remove_dir_all(machine.fakes.path().join("state/launchd")).unwrap();
    machine.calls();

    machine.ok(&["install", "--system", "launchd"]);
    let plist = machine
        .agents()
        .join("com.github.gauthierdoppler.md-preview.plist");
    assert!(machine.calls().contains(&format!(
        "launchctl bootstrap gui/{uid} {}",
        plist.display()
    )));
}

#[test]
fn an_unwritable_launch_agents_folder_is_reported_with_the_fix() {
    if uid() == "0" {
        eprintln!("running as root, permissions are not enforced: skipping");
        return;
    }
    let machine = Machine::new();
    fs::create_dir_all(machine.agents()).unwrap();
    fs::set_permissions(machine.agents(), fs::Permissions::from_mode(0o555)).unwrap();

    let output = machine.service(&["install", "--system", "launchd"]);
    fs::set_permissions(machine.agents(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "sudo chown alice:staff {}",
            machine.agents().display()
        )),
        "{stderr}"
    );
}

#[test]
fn launchd_uninstall_boots_out_and_removes_every_agent() {
    let machine = Machine::new();
    let uid = uid();
    machine.ok(&["install", "--system", "launchd"]);
    machine.calls();

    machine.ok(&["uninstall", "--system", "launchd"]);
    let calls = machine.calls();
    for label in LABELS {
        assert!(!machine.agents().join(format!("{label}.plist")).exists());
        assert!(calls.contains(&format!("launchctl bootout gui/{uid}/{label}")));
    }
}

#[test]
fn launchd_list_shows_each_agent_file_and_state() {
    let machine = Machine::new();
    machine.ok(&["install", "--system", "launchd"]);
    fs::write(
        machine.agents().join("com.theodo.cc-tap.proxy.plist"),
        "stale",
    )
    .unwrap();
    fs::remove_file(
        machine
            .fakes
            .path()
            .join("state/launchd/com.theodo.cc-tap.update"),
    )
    .unwrap();
    fs::write(
        machine
            .fakes
            .path()
            .join("state/launchd/com.theodo.cc-tap.proxy"),
        "spawn scheduled\n",
    )
    .unwrap();

    assert_eq!(
        rows(&machine.ok(&["list", "--system", "launchd"])),
        [
            ["com.theodo.cc-tap.dashboard", "up to date", "running"],
            ["com.theodo.cc-tap.proxy", "outdated", "spawn scheduled"],
            ["com.theodo.cc-tap.update", "up to date", "not loaded"],
            [
                "com.github.gauthierdoppler.md-preview",
                "up to date",
                "running"
            ],
        ]
    );
}

const UNITS: &[&str] = &[
    "com.theodo.cc-tap.dashboard.service",
    "com.theodo.cc-tap.proxy.service",
    "com.theodo.cc-tap.update.service",
    "com.theodo.cc-tap.update.timer",
    "com.github.gauthierdoppler.md-preview.service",
];

#[test]
fn systemd_units_and_timers_are_generated_from_the_description() {
    let machine = Machine::new();
    machine.ok(&["install", "--system", "systemd"]);

    for unit in UNITS {
        let content = fs::read_to_string(machine.units().join(unit))
            .unwrap_or_else(|_| panic!("{unit} written"));
        insta::assert_snapshot!(format!("systemd__{unit}"), content);
    }
}

#[test]
fn systemd_install_reloads_and_restarts_what_changed_and_starts_the_rest() {
    let machine = Machine::new();
    machine.set_linger("yes");
    machine.ok(&["install", "--system", "systemd"]);
    let first = machine.calls();
    assert!(first.contains(&"systemctl --user daemon-reload".to_owned()));
    for unit in [
        "com.theodo.cc-tap.dashboard.service",
        "com.theodo.cc-tap.update.timer",
    ] {
        assert!(first.contains(&format!("systemctl --user enable {unit}")));
        assert!(first.contains(&format!("systemctl --user restart {unit}")));
    }
    assert!(!first.iter().any(|call| call.contains("update.service")));

    fs::write(
        machine.units().join("com.theodo.cc-tap.update.service"),
        "stale",
    )
    .unwrap();
    let written = fs::metadata(machine.units().join("com.theodo.cc-tap.proxy.service"))
        .unwrap()
        .modified()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    machine.ok(&["install", "--system", "systemd"]);
    let second = machine.calls();
    assert_eq!(
        second
            .iter()
            .filter(|call| call.starts_with("systemctl"))
            .collect::<Vec<_>>(),
        [
            "systemctl --user daemon-reload",
            "systemctl --user enable --now com.theodo.cc-tap.dashboard.service",
            "systemctl --user enable --now com.theodo.cc-tap.proxy.service",
            "systemctl --user enable com.theodo.cc-tap.update.timer",
            "systemctl --user restart com.theodo.cc-tap.update.timer",
            "systemctl --user enable --now com.github.gauthierdoppler.md-preview.service",
        ]
    );
    assert_eq!(
        fs::metadata(machine.units().join("com.theodo.cc-tap.proxy.service"))
            .unwrap()
            .modified()
            .unwrap(),
        written
    );
}

#[test]
fn systemd_install_enables_linger_only_when_it_is_off() {
    let machine = Machine::new();
    machine.set_linger("no");
    machine.ok(&["install", "--system", "systemd"]);
    assert!(machine
        .calls()
        .contains(&"loginctl enable-linger alice".to_owned()));

    machine.set_linger("yes");
    machine.ok(&["install", "--system", "systemd"]);
    assert!(!machine
        .calls()
        .iter()
        .any(|call| call.contains("enable-linger")));
}

#[test]
fn systemd_install_honours_xdg_config_home() {
    let machine = Machine::new();
    let output = machine
        .command(&["install", "--system", "systemd"])
        .env("XDG_CONFIG_HOME", machine.home("xdg"))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(machine
        .home("xdg/systemd/user/com.theodo.cc-tap.update.timer")
        .is_file());
}

#[test]
fn systemd_uninstall_disables_and_removes_every_unit() {
    let machine = Machine::new();
    machine.ok(&["install", "--system", "systemd"]);
    machine.calls();

    machine.ok(&["uninstall", "--system", "systemd"]);
    let calls = machine.calls();
    for unit in UNITS {
        assert!(!machine.units().join(unit).exists(), "{unit} left behind");
    }
    assert!(
        calls.contains(&"systemctl --user disable --now com.theodo.cc-tap.update.timer".to_owned())
    );
    assert!(calls.contains(
        &"systemctl --user disable --now com.theodo.cc-tap.dashboard.service".to_owned()
    ));
    assert_eq!(calls.last().unwrap(), "systemctl --user daemon-reload");
}

#[test]
fn systemd_list_shows_each_unit_file_and_activity() {
    let machine = Machine::new();
    machine.ok(&["install", "--system", "systemd"]);
    fs::remove_file(machine.units().join("com.theodo.cc-tap.update.timer")).unwrap();

    assert_eq!(
        rows(&machine.ok(&["list", "--system", "systemd"])),
        [
            ["com.theodo.cc-tap.dashboard", "up to date", "active"],
            ["com.theodo.cc-tap.proxy", "up to date", "active"],
            ["com.theodo.cc-tap.update", "not installed", "active"],
            [
                "com.github.gauthierdoppler.md-preview",
                "up to date",
                "active"
            ],
        ]
    );
}

fn rows(listing: &str) -> Vec<Vec<String>> {
    listing
        .lines()
        .map(|line| {
            line.split("  ")
                .map(str::trim)
                .filter(|field| !field.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .collect()
}
