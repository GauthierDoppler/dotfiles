use std::fmt::Write as _;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{failures, stdout_of, succeeds, write_if_changed, Day, FileState, Service};
use crate::Result;

const HEADER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
"#;

pub(super) fn install(services: &[Service], home: &Path) -> Result<()> {
    let dir = writable_agents_dir(home)?;
    let domain = domain()?;
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

pub(super) fn list(services: &[Service], home: &Path) -> Result<()> {
    let dir = agents_dir(home);
    let domain = domain()?;
    for service in services {
        let state = FileState::of(&plist_path(&dir, service), &render(service)?);
        let running = if loaded(&format!("{domain}/{}", service.label)) {
            "loaded"
        } else {
            "not loaded"
        };
        println!("{:<40} {:<14} {running}", service.label, state.label());
    }
    Ok(())
}

pub(super) fn uninstall(services: &[Service], home: &Path) -> Result<()> {
    let dir = agents_dir(home);
    let domain = domain()?;
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

fn agents_dir(home: &Path) -> PathBuf {
    home.join("Library/LaunchAgents")
}

fn writable_agents_dir(home: &Path) -> Result<PathBuf> {
    let dir = agents_dir(home);
    let probe = dir.join(".atelier-write-probe");
    let writable = fs::create_dir_all(&dir)
        .and_then(|()| fs::write(&probe, ""))
        .and_then(|()| fs::remove_file(&probe));
    match writable {
        Ok(()) => Ok(dir),
        Err(error) => {
            let user = std::env::var("USER").unwrap_or_else(|_| "$USER".into());
            Err(format!(
                "{} is not writable ({error}) -- fix it with: sudo chown {user}:staff {}",
                dir.display(),
                dir.display()
            )
            .into())
        }
    }
}

fn plist_path(dir: &Path, service: &Service) -> PathBuf {
    dir.join(format!("{}.plist", service.label))
}

fn domain() -> Result<String> {
    let uid = stdout_of("id", &["-u"]).filter(|uid| !uid.is_empty());
    Ok(format!("gui/{}", uid.ok_or("cannot read the user id")?))
}

fn loaded(target: &str) -> bool {
    succeeds("launchctl", &["print", target])
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
