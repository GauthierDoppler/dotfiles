use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use clap::Args;
use serde_json::{Map, Value};

use crate::tmux::Tmux;
use crate::Result;

#[derive(Args)]
pub struct Command {
    /// Print the drift that would be captured and the resulting files, and write nothing
    #[arg(long)]
    dry_run: bool,
    /// The dotfiles checkout; defaults to the current git root, then ~/dotfiles
    #[arg(long, value_name = "PATH")]
    repo: Option<PathBuf>,
}

impl clap::Subcommand for Command {
    fn augment_subcommands(command: clap::Command) -> clap::Command {
        <Self as Args>::augment_args(command)
            .subcommand_required(false)
            .arg_required_else_help(false)
    }

    fn augment_subcommands_for_update(command: clap::Command) -> clap::Command {
        <Self as Args>::augment_args_for_update(command)
            .subcommand_required(false)
            .arg_required_else_help(false)
    }

    fn has_subcommand(_name: &str) -> bool {
        false
    }
}

impl Command {
    pub fn run(self, _tmux: &Tmux) -> Result<()> {
        let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME is not set")?);
        let repo = crate::setup::repo_root(self.repo, &home)?;
        let base_path = repo.join("dot_claude/settings.json");
        let local_path = repo.join("dot_claude/settings.local.json");
        let snapshot_path = repo.join("dot_claude/settings.generated.json");
        let live_path = home.join(".claude/settings.json");

        let base = read_object(&base_path, "")?
            .ok_or_else(|| format!("missing base {}", base_path.display()))?;
        let local = read_object(&local_path, "")?.unwrap_or_default();
        let snapshot = read_object(&snapshot_path, "").ok().flatten();
        let live = read_object(&live_path, " — refusing to touch it")?;

        let drift = match &live {
            Some(live) => difference(live, snapshot.as_ref().unwrap_or(&base)),
            None => Map::new(),
        };
        let new_local = merge(&local, &drift);
        let merged = merge(&base, &new_local);
        let rendered = render(&merged)?;

        if self.dry_run {
            if snapshot.is_none() {
                println!("(no snapshot yet — diffing against the base)");
            }
            println!("── drift captured from {} ──", live_path.display());
            print!("{}", render(&drift)?);
            println!("── resulting {} ──", local_path.display());
            print!("{}", render(&new_local)?);
            println!("── resulting {} ──", live_path.display());
            print!("{rendered}");
            return Ok(());
        }

        if new_local != local {
            fs::write(&local_path, render(&new_local)?)?;
            println!("updated: {}", local_path.display());
        }
        match &live {
            Some(live) if *live == merged => println!("up to date: {}", live_path.display()),
            Some(_) => {
                let mut backup = live_path.clone().into_os_string();
                backup.push(".bak");
                fs::copy(&live_path, backup)?;
                fs::write(&live_path, &rendered)?;
                println!("regenerated: {}", live_path.display());
            }
            None => {
                if let Some(parent) = live_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&live_path, &rendered)?;
                println!("regenerated: {}", live_path.display());
            }
        }
        if fs::read_to_string(&snapshot_path).ok().as_ref() != Some(&rendered) {
            fs::write(&snapshot_path, &rendered)?;
        }
        Ok(())
    }
}

fn read_object(path: &Path, refusal: &str) -> Result<Option<Map<String, Value>>> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{}: {error}", path.display()).into()),
    };
    match serde_json::from_str(&content) {
        Ok(Value::Object(object)) => Ok(Some(object)),
        Ok(_) => Err(format!("{} is not a JSON object{refusal}", path.display()).into()),
        Err(_) => Err(format!("{} is not valid JSON{refusal}", path.display()).into()),
    }
}

fn difference(live: &Map<String, Value>, reference: &Map<String, Value>) -> Map<String, Value> {
    let mut drift = Map::new();
    for (key, value) in live {
        match (reference.get(key), value) {
            (Some(old), _) if old == value => {}
            (Some(Value::Object(old)), Value::Object(new)) => {
                let sub = difference(new, old);
                if !sub.is_empty() {
                    drift.insert(key.clone(), Value::Object(sub));
                }
            }
            _ => {
                drift.insert(key.clone(), value.clone());
            }
        }
    }
    drift
}

fn merge(left: &Map<String, Value>, right: &Map<String, Value>) -> Map<String, Value> {
    let mut merged = left.clone();
    for (key, value) in right {
        let value = match (merged.get(key), value) {
            (Some(Value::Object(old)), Value::Object(new)) => Value::Object(merge(old, new)),
            _ => value.clone(),
        };
        merged.insert(key.clone(), value);
    }
    merged
}

fn render(object: &Map<String, Value>) -> Result<String> {
    Ok(serde_json::to_string_pretty(object)? + "\n")
}
