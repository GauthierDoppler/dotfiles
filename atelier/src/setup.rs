mod action;
mod backup;
mod plan;
mod prune;
mod table;

use std::path::{Path, PathBuf};

use clap::{Args, ValueEnum};

use crate::dotfiles;
use crate::tmux::Tmux;
use crate::Result;
use plan::plan;

pub(crate) use table::{Format, STUBS};

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Profile {
    Desktop,
    Remote,
}

#[derive(Args)]
pub struct Command {
    /// Which machine this is; defaults to desktop on macOS, remote elsewhere
    #[arg(long, value_enum)]
    profile: Option<Profile>,
    /// Print what would change and touch nothing
    #[arg(long)]
    dry_run: bool,
    /// The dotfiles checkout; defaults to $DOTFILES, the current git root, then ~/dotfiles
    #[arg(long, value_name = "PATH")]
    repo: Option<PathBuf>,
}

crate::flags_only!(Command);

impl Command {
    pub fn run(self, _tmux: &Tmux) -> Result<()> {
        let home = dotfiles::home()?;
        let repo = dotfiles::repo(self.repo, &home)?;
        let actions = plan(&repo, &home, self.profile.unwrap_or_else(Profile::of_this_machine))?;
        for action in &actions {
            println!("{}", action.describe());
            if !self.dry_run {
                action.apply()?;
            }
        }
        match (actions.is_empty(), self.dry_run) {
            (true, _) => println!("setup: up to date"),
            (false, true) => println!("setup: dry run, nothing written"),
            (false, false) => {}
        }
        Ok(())
    }
}

impl Profile {
    fn of_this_machine() -> Self {
        if cfg!(target_os = "macos") {
            Profile::Desktop
        } else {
            Profile::Remote
        }
    }
}

pub(crate) fn pending(repo: &Path, home: &Path) -> Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = plan(repo, home, Profile::of_this_machine())?
        .iter()
        .map(|action| action.path().to_path_buf())
        .collect();
    paths.dedup();
    Ok(paths)
}
