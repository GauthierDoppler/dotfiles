use std::fmt::Write as _;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use super::{failures, stdout_of, succeeds, write_if_changed, Day, FileState, Schedule, Service};
use crate::Result;

struct Unit {
    name: String,
    content: String,
}

struct Rendered {
    files: Vec<Unit>,
    start: String,
}

pub(super) fn install(services: &[Service], home: &Path) -> Result<()> {
    let dir = units_dir(home);
    fs::create_dir_all(&dir).map_err(|error| format!("cannot create {}: {error}", dir.display()))?;
    let mut plan = Vec::new();
    for service in services {
        let rendered = render(service)?;
        let mut changed = false;
        for unit in &rendered.files {
            changed |= write_if_changed(&dir.join(&unit.name), &unit.content)?;
        }
        plan.push((service, rendered.start, changed));
    }
    if plan.iter().any(|(_, _, changed)| *changed) {
        systemctl(&["daemon-reload"]);
    }
    let mut failed = Vec::new();
    for (service, start, changed) in plan {
        let started = if changed {
            systemctl(&["enable", &start]) && systemctl(&["restart", &start])
        } else {
            systemctl(&["enable", "--now", &start])
        };
        if !started {
            eprintln!("atelier: systemctl could not start {start}");
            failed.push(service.label.clone());
        } else if changed {
            println!("loaded: {}", service.label);
        } else {
            println!("unit ok: {}", service.label);
        }
    }
    linger()?;
    failures(failed)
}

pub(super) fn list(services: &[Service], home: &Path) -> Result<()> {
    let dir = units_dir(home);
    for service in services {
        let rendered = render(service)?;
        let state = rendered
            .files
            .iter()
            .map(|unit| FileState::of(&dir.join(&unit.name), &unit.content))
            .max_by_key(|state| match state {
                FileState::Current => 0,
                FileState::Outdated => 1,
                FileState::Missing => 2,
            })
            .unwrap_or(FileState::Missing);
        let active = stdout_of("systemctl", &["--user", "is-active", &rendered.start])
            .filter(|active| !active.is_empty())
            .unwrap_or_else(|| "unknown".into());
        println!("{:<40} {:<14} {active}", service.label, state.label());
    }
    Ok(())
}

pub(super) fn uninstall(services: &[Service], home: &Path) -> Result<()> {
    let dir = units_dir(home);
    let mut removed_any = false;
    for service in services {
        let rendered = render(service)?;
        if !rendered.files.iter().any(|unit| dir.join(&unit.name).exists()) {
            continue;
        }
        systemctl(&["disable", "--now", &rendered.start]);
        for unit in &rendered.files {
            let path = dir.join(&unit.name);
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(format!("cannot remove {}: {error}", path.display()).into())
                }
            }
        }
        removed_any = true;
        println!("removed: {}", service.label);
    }
    if removed_any {
        systemctl(&["daemon-reload"]);
    }
    Ok(())
}

fn units_dir(home: &Path) -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"))
        .join("systemd/user")
}

fn systemctl(args: &[&str]) -> bool {
    let args: Vec<&str> = std::iter::once("--user").chain(args.iter().copied()).collect();
    succeeds("systemctl", &args)
}

fn linger() -> Result<()> {
    let user = match std::env::var("USER") {
        Ok(user) if !user.is_empty() => user,
        _ => stdout_of("id", &["-un"])
            .filter(|user| !user.is_empty())
            .ok_or("cannot tell which user to enable linger for")?,
    };
    let enabled = stdout_of(
        "loginctl",
        &["show-user", &user, "--property=Linger", "--value"],
    );
    if enabled.as_deref() == Some("yes") {
        return Ok(());
    }
    if succeeds("loginctl", &["enable-linger", &user]) {
        println!("linger: enabled for {user}");
        Ok(())
    } else {
        Err(format!(
            "loginctl enable-linger {user} failed -- services stop at logout until it succeeds"
        )
        .into())
    }
}

fn render(service: &Service) -> Result<Rendered> {
    let description = service.description.replace('%', "%%");
    let mut unit = format!("[Unit]\nDescription={description}\n\n[Service]\n");
    if service.schedule.is_some() {
        unit.push_str("Type=oneshot\n");
    }
    let mut exec = format!("%h/{}", service.run[0]);
    for arg in &service.run[1..] {
        exec.push(' ');
        exec.push_str(&quote(arg));
    }
    let _ = writeln!(unit, "ExecStart={exec}");
    let service_file = format!("{}.service", service.label);
    let Some(schedule) = &service.schedule else {
        unit.push_str("Restart=always\nRestartSec=10\n\n[Install]\nWantedBy=default.target\n");
        return Ok(Rendered {
            files: vec![Unit {
                name: service_file.clone(),
                content: unit,
            }],
            start: service_file,
        });
    };
    let timer_file = format!("{}.timer", service.label);
    Ok(Rendered {
        files: vec![
            Unit {
                name: service_file,
                content: unit,
            },
            Unit {
                name: timer_file.clone(),
                content: timer(&description, schedule)?,
            },
        ],
        start: timer_file,
    })
}

fn timer(description: &str, schedule: &Schedule) -> Result<String> {
    let (hour, minute) = schedule.time()?;
    let mut timer = format!(
        "[Unit]\nDescription={description}\n\n[Timer]\nOnActiveSec=0\nOnCalendar={} {hour:02}:{minute:02}\nPersistent=true\n",
        calendar_days(&schedule.days)
    );
    if let Some(every) = schedule.every {
        let _ = writeln!(timer, "OnUnitActiveSec={every}s");
    }
    timer.push_str("\n[Install]\nWantedBy=timers.target\n");
    Ok(timer)
}

fn calendar_days(days: &[Day]) -> String {
    let mut days = days.to_vec();
    days.sort();
    days.dedup();
    let mut runs: Vec<(Day, Day)> = Vec::new();
    for day in days {
        match runs.last_mut() {
            Some((_, end)) if *end as u8 + 1 == day as u8 => *end = day,
            _ => runs.push((day, day)),
        }
    }
    runs.iter()
        .map(|&(start, end)| match end as u8 - start as u8 {
            0 => name(start).to_owned(),
            1 => format!("{},{}", name(start), name(end)),
            _ => format!("{}..{}", name(start), name(end)),
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn name(day: Day) -> &'static str {
    match day {
        Day::Mon => "Mon",
        Day::Tue => "Tue",
        Day::Wed => "Wed",
        Day::Thu => "Thu",
        Day::Fri => "Fri",
        Day::Sat => "Sat",
        Day::Sun => "Sun",
    }
}

fn quote(arg: &str) -> String {
    let escaped = arg.replace('%', "%%").replace('$', "$$");
    if !arg.is_empty() && arg.chars().all(super::is_shell_safe) {
        escaped
    } else {
        format!("\"{}\"", escaped.replace('\\', r"\\").replace('"', "\\\""))
    }
}
