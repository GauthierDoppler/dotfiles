use std::fmt::Write as _;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{failures, write_if_changed, Day, FileState, Service, Status};
use crate::process::{self, succeeds};
use crate::Result;

const HEADER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
"#;

pub(super) fn install(services: &[Service], home: &Path) -> Result<()> {
    let dir = writable_agents_dir(home)?;
    let domain = domain();
    let mut failed = Vec::new();
    for service in services {
        let path = plist_path(&dir, service);
        let plist = render(service)?;
        let target = format!("{domain}/{}", service.label);
        if FileState::of(&path, &plist) == FileState::Current && loaded(&target) {
            succeeds("launchctl", &["kickstart", &target]);
            println!("agent ok: {}", service.label);
            continue;
        }
        // bootout returns before the service is gone, and a bootstrap issued in
        // that window fails with "5: Input/output error".
        if succeeds("launchctl", &["bootout", &target]) {
            for _ in 0..50 {
                if !loaded(&target) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        write_if_changed(&path, &plist)?;
        if succeeds("launchctl", &["bootstrap", &domain, &path.to_string_lossy()]) {
            // A fresh bootstrap can sit at "pended nondemand spawn = speculative"
            // for minutes; kickstart without -k starts it and leaves a running one alone.
            succeeds("launchctl", &["kickstart", &target]);
            println!("loaded: {}", service.label);
        } else {
            eprintln!("atelier: launchctl bootstrap failed for {}", service.label);
            failed.push(service.label.clone());
        }
    }
    failures(failed)
}

pub(super) fn statuses(services: &[Service], home: &Path) -> Result<Vec<Status>> {
    let dir = agents_dir(home);
    let domain = domain();
    let mut statuses = Vec::new();
    for service in services {
        let state = match state(&format!("{domain}/{}", service.label)) {
            None => "not loaded".to_owned(),
            Some(_) if service.schedule.is_some() => "loaded".to_owned(),
            Some(state) => state,
        };
        statuses.push(Status {
            label: service.label.clone(),
            file: FileState::of(&plist_path(&dir, service), &render(service)?),
            running: matches!(state.as_str(), "loaded" | "running"),
            state,
        });
    }
    Ok(statuses)
}

pub(super) fn uninstall(services: &[Service], home: &Path) -> Result<()> {
    let dir = agents_dir(home);
    let domain = domain();
    for service in services {
        let path = plist_path(&dir, service);
        let unloaded = succeeds("launchctl", &["bootout", &format!("{domain}/{}", service.label)]);
        let removed = match fs::remove_file(&path) {
            Ok(()) => true,
            Err(error) if error.kind() == ErrorKind::NotFound => false,
            Err(error) => return Err(format!("cannot remove {}: {error}", path.display()).into()),
        };
        if unloaded || removed {
            println!("removed: {}", service.label);
        }
    }
    Ok(())
}

pub(super) fn installed(label: &str, home: &Path) -> bool {
    agents_dir(home).join(format!("{label}.plist")).is_file()
}

pub(super) fn kickstart(label: &str, kill: bool) -> bool {
    let target = format!("{}/{label}", domain());
    if kill {
        succeeds("launchctl", &["kickstart", "-k", &target])
    } else {
        succeeds("launchctl", &["kickstart", &target])
    }
}

fn agents_dir(home: &Path) -> PathBuf {
    home.join("Library/LaunchAgents")
}

fn writable_agents_dir(home: &Path) -> Result<PathBuf> {
    let dir = agents_dir(home);
    if let Err(error) = fs::create_dir_all(&dir).and_then(|()| probe(&dir)) {
        let (problem, fix) = unwritable(&dir, &error);
        return Err(format!("{problem} -- fix it with: {fix}").into());
    }
    Ok(dir)
}

pub(super) fn unwritable_agents_dir(home: &Path) -> Option<(String, String)> {
    let dir = agents_dir(home);
    if !dir.exists() {
        return None;
    }
    probe(&dir).err().map(|error| unwritable(&dir, &error))
}

fn probe(dir: &Path) -> std::io::Result<()> {
    let probe = dir.join(".atelier-write-probe");
    fs::write(&probe, "").and_then(|()| fs::remove_file(&probe))
}

fn unwritable(dir: &Path, error: &std::io::Error) -> (String, String) {
    let user = std::env::var("USER").unwrap_or_else(|_| "$USER".into());
    (
        format!("{} is not writable ({error})", dir.display()),
        format!("sudo chown {user}:staff {}", dir.display()),
    )
}

fn plist_path(dir: &Path, service: &Service) -> PathBuf {
    dir.join(format!("{}.plist", service.label))
}

fn domain() -> String {
    format!("gui/{}", process::uid())
}

fn loaded(target: &str) -> bool {
    succeeds("launchctl", &["print", target])
}

fn state(target: &str) -> Option<String> {
    let output = process::output("launchctl", &["print", target])?;
    if !output.status.success() {
        return None;
    }
    let printed = String::from_utf8_lossy(&output.stdout);
    let state = printed
        .lines()
        .filter_map(|line| line.strip_prefix('\t'))
        .find_map(|line| line.strip_prefix("state = "))
        .unwrap_or("unknown");
    Some(state.trim().to_owned())
}

fn render(service: &Service) -> Result<String> {
    let mut plist = String::from(HEADER);
    let _ = writeln!(plist, "  <key>Label</key><string>{}</string>", service.label);
    plist.push_str("  <key>ProgramArguments</key>\n  <array>\n");
    for arg in ["/bin/sh", "-c", &script(service)] {
        let _ = writeln!(plist, "    <string>{}</string>", xml_escape(arg));
    }
    plist.push_str("  </array>\n  <key>RunAtLoad</key><true/>\n");
    match &service.schedule {
        None => plist.push_str(
            "  <key>KeepAlive</key><true/>\n  <key>ThrottleInterval</key><integer>10</integer>\n",
        ),
        Some(schedule) => {
            let (hour, minute) = schedule.time()?;
            plist.push_str("  <key>StartCalendarInterval</key>\n  <array>\n");
            for day in &schedule.days {
                let _ = writeln!(
                    plist,
                    "    <dict><key>Weekday</key><integer>{}</integer><key>Hour</key><integer>{hour}</integer><key>Minute</key><integer>{minute}</integer></dict>",
                    weekday(*day)
                );
            }
            plist.push_str("  </array>\n");
            if let Some(every) = schedule.every {
                let _ = writeln!(plist, "  <key>StartInterval</key><integer>{every}</integer>");
            }
        }
    }
    plist.push_str("</dict>\n</plist>\n");
    Ok(plist)
}

fn script(service: &Service) -> String {
    let mut script = format!("exec \"$HOME/{}\"", service.run[0]);
    for arg in &service.run[1..] {
        script.push(' ');
        script.push_str(&shell_quote(arg));
    }
    if let Some(log) = &service.log {
        let _ = write!(script, " >>\"$HOME/Library/Logs/{log}\" 2>&1");
    }
    script
}

fn shell_quote(arg: &str) -> String {
    if !arg.is_empty() && arg.chars().all(super::is_shell_safe) {
        arg.to_owned()
    } else {
        crate::shell::quote(arg)
    }
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;")
}

fn weekday(day: Day) -> u8 {
    match day {
        Day::Sun => 0,
        Day::Mon => 1,
        Day::Tue => 2,
        Day::Wed => 3,
        Day::Thu => 4,
        Day::Fri => 5,
        Day::Sat => 6,
    }
}
