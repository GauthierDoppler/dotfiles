mod launchd;
mod systemd;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as Process, Stdio};

use clap::{Args, Subcommand, ValueEnum};
use serde::Deserialize;

use crate::tmux::Tmux;
use crate::Result;

const DESCRIPTION: &str = "services.toml";

#[derive(Subcommand)]
pub enum Command {
    /// Write each service's agent or unit, reload what changed, start what is not running
    Install(Target),
    /// Show whether each service is installed, current and running
    List(Target),
    /// Stop each service and remove its agent or unit
    Uninstall(Target),
}

#[derive(Args)]
pub struct Target {
    /// Service manager to write for; defaults to launchd on macOS, systemd elsewhere
    #[arg(long, value_enum)]
    system: Option<System>,
    /// The dotfiles checkout holding services.toml; defaults to the current git root, then ~/dotfiles
    #[arg(long, value_name = "PATH")]
    repo: Option<PathBuf>,
}

#[derive(Clone, Copy, ValueEnum)]
enum System {
    Launchd,
    Systemd,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Description {
    service: Vec<Service>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Service {
    label: String,
    description: String,
    run: Vec<String>,
    log: Option<String>,
    schedule: Option<Schedule>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Schedule {
    days: Vec<Day>,
    at: String,
    every: Option<u32>,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
enum Day {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl Schedule {
    fn time(&self) -> Result<(u8, u8)> {
        let invalid = || format!("schedule.at must be HH:MM, got {:?}", self.at);
        let (hour, minute) = self.at.split_once(':').ok_or_else(invalid)?;
        let hour: u8 = hour.parse().map_err(|_| invalid())?;
        let minute: u8 = minute.parse().map_err(|_| invalid())?;
        if hour > 23 || minute > 59 {
            return Err(invalid().into());
        }
        Ok((hour, minute))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum FileState {
    Current,
    Outdated,
    Missing,
}

impl FileState {
    fn of(path: &Path, content: &str) -> Self {
        match fs::read(path) {
            Ok(existing) if existing == content.as_bytes() => FileState::Current,
            Ok(_) => FileState::Outdated,
            Err(_) => FileState::Missing,
        }
    }

    fn label(self) -> &'static str {
        match self {
            FileState::Current => "up to date",
            FileState::Outdated => "outdated",
            FileState::Missing => "not installed",
        }
    }
}

impl Command {
    pub fn run(self, _tmux: &Tmux) -> Result<()> {
        let (Command::Install(target) | Command::List(target) | Command::Uninstall(target)) = &self;
        let (services, home) = target.load()?;
        let system = target.system();
        match (self, system) {
            (Command::Install(_), System::Launchd) => launchd::install(&services, &home),
            (Command::Install(_), System::Systemd) => systemd::install(&services, &home),
            (Command::List(_), system) => {
                for status in statuses(system, &services, &home)? {
                    println!(
                        "{:<40} {:<14} {}",
                        status.label,
                        status.file.label(),
                        status.state
                    );
                }
                Ok(())
            }
            (Command::Uninstall(_), System::Launchd) => launchd::uninstall(&services, &home),
            (Command::Uninstall(_), System::Systemd) => systemd::uninstall(&services, &home),
        }
    }
}

impl Target {
    fn system(&self) -> System {
        self.system.unwrap_or(if cfg!(target_os = "macos") {
            System::Launchd
        } else {
            System::Systemd
        })
    }

    pub(crate) fn repo(&self) -> Option<PathBuf> {
        self.repo.clone()
    }

    fn load(&self) -> Result<(Vec<Service>, PathBuf)> {
        let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME is not set")?);
        let repo = crate::setup::repo_root(self.repo.clone(), &home)?;
        Ok((load(&repo.join(DESCRIPTION))?, home))
    }
}

struct Status {
    label: String,
    file: FileState,
    state: String,
    running: bool,
}

fn statuses(system: System, services: &[Service], home: &Path) -> Result<Vec<Status>> {
    match system {
        System::Launchd => launchd::statuses(services, home),
        System::Systemd => systemd::statuses(services, home),
    }
}

pub(crate) fn stopped(target: &Target) -> Result<Vec<String>> {
    let (services, home) = target.load()?;
    Ok(statuses(target.system(), &services, &home)?
        .into_iter()
        .filter(|status| !status.running)
        .map(|status| status.label)
        .collect())
}

pub(crate) fn agents_dir_problem(target: &Target) -> Option<(String, String)> {
    match target.system() {
        System::Launchd => {
            let home = PathBuf::from(std::env::var_os("HOME")?);
            launchd::unwritable_agents_dir(&home)
        }
        System::Systemd => None,
    }
}

pub(crate) fn linger(target: &Target) -> Option<Result<(String, bool)>> {
    match target.system() {
        System::Launchd => None,
        System::Systemd => Some(systemd::linger_state()),
    }
}

fn load(path: &Path) -> Result<Vec<Service>> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let description: Description =
        toml::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    for service in &description.service {
        validate(service).map_err(|error| format!("{}: {}: {error}", path.display(), service.label))?;
    }
    Ok(description.service)
}

fn validate(service: &Service) -> Result<()> {
    if service.label.is_empty() || !service.label.chars().all(is_label_char) {
        return Err("label may only hold letters, digits, '.', '-' and '_'".into());
    }
    let program = service.run.first().ok_or("run needs at least the program")?;
    for path in std::iter::once(program).chain(service.log.as_ref()) {
        if path.starts_with('/') || path.is_empty() || !path.chars().all(is_shell_safe) {
            return Err(format!("{path:?} must be a plain path relative to $HOME").into());
        }
    }
    if let Some(schedule) = &service.schedule {
        schedule.time()?;
        if schedule.days.is_empty() {
            return Err("schedule.days is empty".into());
        }
    }
    Ok(())
}

fn is_label_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')
}

fn is_shell_safe(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '/' | ':' | '=' | '@' | '+' | ',')
}

fn succeeds(program: &str, args: &[&str]) -> bool {
    Process::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn stdout_of(program: &str, args: &[&str]) -> Option<String> {
    let output = Process::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn write_if_changed(path: &Path, content: &str) -> Result<bool> {
    if FileState::of(path, content) == FileState::Current {
        return Ok(false);
    }
    fs::write(path, content).map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(true)
}

fn failures(failed: Vec<String>) -> Result<()> {
    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!("failed to start: {}", failed.join(", ")).into())
    }
}
