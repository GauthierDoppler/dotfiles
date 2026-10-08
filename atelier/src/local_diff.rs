use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Stdio};

use clap::Args;
use serde_json::Value;

use crate::setup::{self, Format, STUBS};
use crate::tmux::Tmux;
use crate::Result;

const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[90m";
const YELLOW: &str = "\x1b[33m";
const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";

#[derive(Args)]
pub struct Command {
    /// The dotfiles checkout; defaults to $DOTFILES, the current git root, then ~/dotfiles
    #[arg(long, value_name = "PATH")]
    repo: Option<PathBuf>,
}

crate::flags_only!(Command);

impl Command {
    pub fn run(self, _tmux: &Tmux) -> Result<()> {
        let home = PathBuf::from(env::var_os("HOME").ok_or("HOME is not set")?);
        let flag = self
            .repo
            .or_else(|| env::var_os("DOTFILES").map(PathBuf::from));
        let repo = setup::repo_root(flag, &home)?;
        let report = Report { home: &home };
        let mut found = false;

        for stub in STUBS {
            let local = home.join(stub.dest);
            let shared = repo.join(stub.shared);
            if !local.is_file() {
                continue;
            }
            let extra = match stub.format {
                Format::Shell => difference(shell_lines(&local), shell_lines(&shared))
                    .filter(|line| !loads(line, stub.shared))
                    .collect(),
                Format::Git => difference(git_keys(&local), git_keys(&shared))
                    .filter(|line| !line.starts_with("include.path="))
                    .collect(),
            };
            found |= report.finding(&local, &shared, extra);
        }

        let local = repo.join("dot_claude/settings.local.json");
        let shared = repo.join("dot_claude/settings.json");
        if local.is_file() {
            let extra = difference(json_values(&local), json_values(&shared)).collect();
            found |= report.finding(&local, &shared, extra);
        }

        let legacy = legacy_local_files(&home)?;
        if !legacy.is_empty() {
            found = true;
            report.print(
                "Legacy .local files",
                "nothing sources these — run install.sh to migrate them into the stubs",
                &legacy,
            );
        }

        if !found {
            println!("{GREEN}✓ local config adds nothing beyond the shared dotfiles{RESET}");
        }
        Ok(())
    }
}

struct Report<'a> {
    home: &'a Path,
}

impl Report<'_> {
    fn finding(&self, local: &Path, shared: &Path, extra: Vec<String>) -> bool {
        if extra.is_empty() {
            return false;
        }
        let subtitle = format!("not in {}", self.shorten(shared));
        self.print(&self.shorten(local), &subtitle, &extra);
        true
    }

    fn print(&self, title: &str, subtitle: &str, lines: &[String]) {
        println!("\n{BOLD}{title}{RESET}\n{DIM}{subtitle}{RESET}");
        for line in lines {
            println!("  {YELLOW}{line}{RESET}");
        }
    }

    fn shorten(&self, path: &Path) -> String {
        match path.strip_prefix(self.home) {
            Ok(rest) => Path::new("~").join(rest).display().to_string(),
            Err(_) => path.display().to_string(),
        }
    }
}

fn difference(
    local: BTreeSet<String>,
    shared: BTreeSet<String>,
) -> impl Iterator<Item = String> {
    local.into_iter().filter(move |line| !shared.contains(line))
}

fn loads(line: &str, shared: &str) -> bool {
    line.strip_suffix('"').unwrap_or(line).ends_with(shared)
}

fn shell_lines(path: &Path) -> BTreeSet<String> {
    let content = fs::read(path).unwrap_or_default();
    String::from_utf8_lossy(&content)
        .lines()
        .map(|line| line.split('#').next().unwrap_or_default().trim())
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn git_keys(path: &Path) -> BTreeSet<String> {
    let output = process::Command::new("git")
        .arg("config")
        .arg("--file")
        .arg(path)
        .arg("--list")
        .stderr(Stdio::null())
        .output();
    match output {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_string)
            .collect(),
        _ => BTreeSet::new(),
    }
}

fn json_values(path: &Path) -> BTreeSet<String> {
    let mut values = BTreeSet::new();
    let parsed = fs::read(path)
        .ok()
        .and_then(|content| serde_json::from_slice::<Value>(&content).ok());
    if let Some(value) = parsed {
        flatten(&value, &mut Vec::new(), &mut values);
    }
    values
}

fn flatten(value: &Value, path: &mut Vec<String>, out: &mut BTreeSet<String>) {
    let children: Vec<(String, &Value)> = match value {
        Value::Object(map) => map.iter().map(|(k, v)| (k.clone(), v)).collect(),
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v))
            .collect(),
        Value::String(s) => {
            out.insert(format!("{}={s}", path.join(".")));
            return;
        }
        scalar => {
            out.insert(format!("{}={scalar}", path.join(".")));
            return;
        }
    };
    for (key, child) in children {
        path.push(key);
        flatten(child, path, out);
        path.pop();
    }
}

fn legacy_local_files(home: &Path) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(home)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let legacy = name
            .strip_prefix('.')
            .is_some_and(|rest| rest.ends_with(".local"));
        if legacy && entry.path().exists() {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}
