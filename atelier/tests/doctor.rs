mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use common::TmuxServer;

const FAKE_INFOCMP: &str = r#"#!/bin/sh
for name; do :; done
[ -e "$FAKE_STATE/terminfo/$name" ] || { echo "infocmp: couldn't open terminfo file for $name" >&2; exit 1; }
"#;

const FAKE_FZF: &str = r#"#!/bin/sh
version=0.65.2
[ -e "$FAKE_STATE/fzf-version" ] && read -r version <"$FAKE_STATE/fzf-version"
echo "$version (brew)"
"#;

const FAKE_SYSTEMCTL: &str = r#"#!/bin/sh
for unit; do :; done
case "$2" in
  is-active)
    if [ -e "$FAKE_STATE/inactive/$unit" ]; then echo inactive; exit 3; fi
    echo active ;;
esac
"#;

const FAKE_LOGINCTL: &str = r#"#!/bin/sh
case "$1" in
  show-user)
    linger=yes
    [ -e "$FAKE_STATE/linger" ] && read -r linger <"$FAKE_STATE/linger"
    echo "$linger" ;;
esac
"#;

const FAKE_LAUNCHCTL: &str = r#"#!/bin/sh
case "$1" in
  print)
    [ -e "$FAKE_STATE/inactive/${2##*/}" ] && exit 113
    state=running
    [ -e "$FAKE_STATE/launchd/${2##*/}" ] && read -r state <"$FAKE_STATE/launchd/${2##*/}"
    printf '%s = {\n\tstate = %s\n\tjob state = exited\n}\n' "$2" "$state" ;;
esac
"#;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crate sits inside the dotfiles repo")
        .to_path_buf()
}

fn real(program: &str) -> PathBuf {
    let output = Command::new("sh")
        .args(["-c", &format!("command -v {program}")])
        .output()
        .unwrap();
    PathBuf::from(String::from_utf8(output.stdout).unwrap().trim())
}

struct Machine {
    tmux: TmuxServer,
    home: tempfile::TempDir,
    fakes: tempfile::TempDir,
}

impl Machine {
    fn healthy() -> Self {
        let machine = Machine {
            tmux: TmuxServer::start(),
            home: tempfile::tempdir().unwrap(),
            fakes: tempfile::tempdir().unwrap(),
        };
        for dir in ["bin", "state/terminfo", "state/inactive", "state/launchd"] {
            fs::create_dir_all(machine.fakes.path().join(dir)).unwrap();
        }
        for program in ["tmux", "uname"] {
            std::os::unix::fs::symlink(real(program), machine.bin(program)).unwrap();
        }
        for (name, script) in [
            ("infocmp", FAKE_INFOCMP),
            ("fzf", FAKE_FZF),
            ("systemctl", FAKE_SYSTEMCTL),
            ("loginctl", FAKE_LOGINCTL),
            ("launchctl", FAKE_LAUNCHCTL),
        ] {
            machine.fake(name, script);
        }
        machine.state("terminfo/xterm-ghostty", "");

        let installed = machine.home.path().join(".local/bin/atelier");
        fs::create_dir_all(installed.parent().unwrap()).unwrap();
        fs::copy(env!("CARGO_BIN_EXE_atelier"), &installed).unwrap();
        fs::create_dir_all(
            machine
                .home
                .path()
                .join("Library/Keyboard Layouts/FR-AZERTY-num.bundle"),
        )
        .unwrap();
        let setup = Command::new(env!("CARGO_BIN_EXE_atelier"))
            .args(["setup", "--repo"])
            .arg(repo())
            .env("HOME", machine.home.path())
            .output()
            .unwrap();
        assert!(setup.status.success(), "setup failed: {setup:?}");

        machine
            .tmux
            .tmux(&["set-option", "-s", "extended-keys", "on"]);
        machine.tmux.new_session("work", machine.home.path());
        machine.tmux.tmux(&[
            "run-shell",
            "-b",
            &format!("{} daemon --ensure", env!("CARGO_BIN_EXE_atelier")),
        ]);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !machine
            .tmux
            .atelier_stdout(&["status"])
            .starts_with("daemon: running")
        {
            assert!(Instant::now() < deadline, "the daemon never answered");
            std::thread::sleep(Duration::from_millis(50));
        }
        machine
    }

    fn bin(&self, name: &str) -> PathBuf {
        self.fakes.path().join("bin").join(name)
    }

    fn fake(&self, name: &str, script: &str) {
        let path = self.bin(name);
        let _ = fs::remove_file(&path);
        fs::write(&path, script).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn state(&self, relative: &str, content: &str) {
        fs::write(self.fakes.path().join("state").join(relative), content).unwrap();
    }

    fn doctor_with(&self, mut command: Command, args: &[&str]) -> Output {
        command
            .arg("doctor")
            .args(args)
            .arg("--repo")
            .arg(repo())
            .env("HOME", self.home.path())
            .env_remove("XDG_CONFIG_HOME")
            .env("USER", "alice")
            .env("TERM", "xterm-ghostty")
            .env("PATH", self.fakes.path().join("bin"))
            .env("FAKE_STATE", self.fakes.path().join("state"));
        command.output().expect("atelier runs")
    }

    fn doctor(&self, args: &[&str]) -> Output {
        self.doctor_with(self.tmux.atelier_command(&[]), args)
    }
}

fn report(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn check<'a>(report: &'a str, name: &str) -> &'a str {
    report
        .lines()
        .find(|line| line.get(6..).is_some_and(|rest| rest.starts_with(name)))
        .unwrap_or_else(|| panic!("no {name:?} check in:\n{report}"))
}

fn fix<'a>(report: &'a str, name: &str) -> &'a str {
    let line = check(report, name);
    let mut lines = report.lines().skip_while(|l| *l != line).skip(1);
    lines
        .next()
        .and_then(|next| next.trim_start().strip_prefix("fix: "))
        .unwrap_or_else(|| panic!("no fix under {name:?} in:\n{report}"))
}

fn assert_fails(machine: &Machine, args: &[&str], name: &str) -> String {
    let output = machine.doctor(args);
    let report = report(&output);
    assert!(!output.status.success(), "doctor passed:\n{report}");
    assert!(
        check(&report, name).starts_with("FAIL"),
        "{name} did not fail:\n{report}"
    );
    report
}

#[test]
fn a_healthy_machine_passes_every_check() {
    let machine = Machine::healthy();
    let output = machine.doctor(&[]);
    let report = report(&output);
    assert!(
        output.status.success(),
        "{report}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!report.contains("FAIL"), "{report}");
    for name in [
        "terminfo",
        "tmux version",
        "extended keys",
        "terminal features",
        "fzf",
        "atelier installed",
        "setup",
        "daemon",
        "services",
        "linger",
    ] {
        assert!(check(&report, name).starts_with("ok"), "{name}:\n{report}");
    }
}

#[test]
fn the_nerd_font_check_shows_the_glyphs_and_leaves_the_verdict_to_the_reader() {
    let machine = Machine::healthy();
    let report = report(&machine.doctor(&[]));
    let line = check(&report, "nerd font");
    assert!(line.starts_with("look"), "{report}");
    assert!(
        line.contains('\u{e0b6}') && line.contains('\u{e0b4}'),
        "{line}"
    );
}

#[test]
fn missing_terminfo_prints_the_command_that_copies_it_over() {
    let machine = Machine::healthy();
    fs::remove_file(machine.fakes.path().join("state/terminfo/xterm-ghostty")).unwrap();
    let report = assert_fails(&machine, &[], "terminfo");
    assert!(check(&report, "terminfo").contains("xterm-ghostty"));
    let host = String::from_utf8(Command::new("uname").arg("-n").output().unwrap().stdout).unwrap();
    assert_eq!(
        fix(&report, "terminfo"),
        format!("infocmp -x xterm-ghostty | ssh {} tic -x -", host.trim())
    );
}

#[test]
fn a_tmux_older_than_3_3_fails() {
    let machine = Machine::healthy();
    machine.fake(
        "tmux",
        &format!(
            "#!/bin/sh\n[ \"$1\" = -V ] && {{ echo 'tmux 3.2a'; exit; }}\nexec {} \"$@\"\n",
            real("tmux").display()
        ),
    );
    let report = assert_fails(&machine, &[], "tmux version");
    assert!(check(&report, "tmux version").contains("3.2a"), "{report}");
    assert!(check(&report, "tmux version").contains("3.3"), "{report}");
}

#[test]
fn a_development_tmux_build_passes() {
    let machine = Machine::healthy();
    machine.fake(
        "tmux",
        &format!(
            "#!/bin/sh\n[ \"$1\" = -V ] && {{ echo 'tmux next-3.6'; exit; }}\nexec {} \"$@\"\n",
            real("tmux").display()
        ),
    );
    let report = report(&machine.doctor(&[]));
    assert!(check(&report, "tmux version").starts_with("ok"), "{report}");
}

#[test]
fn extended_keys_off_fails_with_the_option_to_set() {
    let machine = Machine::healthy();
    machine
        .tmux
        .tmux(&["set-option", "-s", "extended-keys", "off"]);
    let report = assert_fails(&machine, &[], "extended keys");
    assert!(fix(&report, "extended keys").contains("set -s extended-keys on"));
}

#[test]
fn terminal_features_grown_by_reloads_fail() {
    let machine = Machine::healthy();
    for _ in 0..2 {
        machine
            .tmux
            .tmux(&["set-option", "-as", "terminal-features", ",xterm-kitty:RGB"]);
    }
    let report = assert_fails(&machine, &[], "terminal features");
    assert!(check(&report, "terminal features").contains("xterm-kitty:RGB"));
    assert!(fix(&report, "terminal features").contains("set -gu terminal-features"));
}

#[test]
fn an_fzf_older_than_0_45_fails() {
    let machine = Machine::healthy();
    machine.state("fzf-version", "0.44.1\n");
    let report = assert_fails(&machine, &[], "fzf");
    assert!(check(&report, "fzf").contains("0.44.1"), "{report}");
}

#[test]
fn a_missing_fzf_fails() {
    let machine = Machine::healthy();
    fs::remove_file(machine.bin("fzf")).unwrap();
    let report = assert_fails(&machine, &[], "fzf");
    assert!(check(&report, "fzf").contains("not found"), "{report}");
}

#[test]
fn an_atelier_missing_from_local_bin_fails() {
    let machine = Machine::healthy();
    fs::remove_file(machine.home.path().join(".local/bin/atelier")).unwrap();
    let report = assert_fails(&machine, &[], "atelier installed");
    assert!(fix(&report, "atelier installed").contains("cargo install"));
}

#[test]
fn setup_drift_fails_with_what_setup_would_change() {
    let machine = Machine::healthy();
    fs::remove_file(machine.home.path().join(".tmux.conf")).unwrap();
    let report = assert_fails(&machine, &[], "setup");
    assert!(check(&report, "setup").contains(".tmux.conf"), "{report}");
    assert!(fix(&report, "setup").starts_with("atelier setup"));
}

#[test]
fn a_stopped_systemd_service_fails_naming_it() {
    let machine = Machine::healthy();
    machine.state("inactive/com.theodo.cc-tap.proxy.service", "");
    let report = assert_fails(&machine, &["--system", "systemd"], "services");
    let line = check(&report, "services");
    assert!(line.contains("com.theodo.cc-tap.proxy"), "{report}");
    assert!(!line.contains("dashboard"), "{report}");
    assert!(fix(&report, "services").contains("atelier service install"));
}

#[test]
fn a_stopped_launchd_agent_fails_naming_it() {
    let machine = Machine::healthy();
    machine.state("inactive/com.theodo.cc-tap.dashboard", "");
    let report = assert_fails(&machine, &["--system", "launchd"], "services");
    assert!(check(&report, "services").contains("com.theodo.cc-tap.dashboard"));
    assert!(check(&report, "linger").starts_with("skip"), "{report}");
}

#[test]
fn a_crash_looping_launchd_agent_fails_although_it_stays_loaded() {
    let machine = Machine::healthy();
    machine.state("launchd/com.theodo.cc-tap.proxy", "spawn scheduled\n");
    machine.state("launchd/com.theodo.cc-tap.update", "not running\n");
    let report = assert_fails(&machine, &["--system", "launchd"], "services");
    let line = check(&report, "services");
    assert!(line.contains("com.theodo.cc-tap.proxy"), "{report}");
    assert!(!line.contains("update"), "{report}");
}

#[test]
fn a_tmux_outside_path_is_still_found() {
    let machine = Machine::healthy();
    fs::remove_file(machine.bin("tmux")).unwrap();
    let report = report(&machine.doctor(&[]));
    assert!(check(&report, "tmux version").starts_with("ok"), "{report}");
}

#[test]
fn linger_off_fails_with_the_loginctl_command() {
    let machine = Machine::healthy();
    machine.state("linger", "no\n");
    let report = assert_fails(&machine, &["--system", "systemd"], "linger");
    assert_eq!(fix(&report, "linger"), "loginctl enable-linger alice");
}

#[test]
fn no_daemon_for_this_tmux_server_fails() {
    let machine = Machine::healthy();
    let other = TmuxServer::start();
    other.new_session("idle", machine.home.path());
    let output = machine.doctor_with(other.atelier_command(&[]), &[]);
    let report = report(&output);
    assert!(!output.status.success(), "{report}");
    assert!(check(&report, "daemon").starts_with("FAIL"), "{report}");
    assert!(fix(&report, "daemon").contains("atelier daemon --ensure"));
}

#[test]
fn without_a_tmux_server_its_checks_are_skipped_not_failed() {
    let machine = Machine::healthy();
    let mut command = Command::new(env!("CARGO_BIN_EXE_atelier"));
    command
        .arg("--socket")
        .arg(machine.fakes.path().join("no-such-socket"))
        .env_remove("TMUX")
        .env("XDG_RUNTIME_DIR", machine.fakes.path());
    let output = machine.doctor_with(command, &[]);
    let report = report(&output);
    assert!(output.status.success(), "{report}");
    for name in ["extended keys", "terminal features", "daemon"] {
        assert!(
            check(&report, name).starts_with("skip"),
            "{name}:\n{report}"
        );
    }
}

#[cfg(not(target_os = "macos"))]
#[test]
fn the_keyboard_layout_is_only_checked_on_macos() {
    let machine = Machine::healthy();
    let report = report(&machine.doctor(&[]));
    assert!(
        check(&report, "keyboard layout").starts_with("skip"),
        "{report}"
    );
}
