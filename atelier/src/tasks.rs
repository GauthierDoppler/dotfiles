mod catalog;
mod picker;
mod runner;

use std::path::PathBuf;

use clap::{Args, Subcommand};

use crate::tmux::Tmux;
use crate::Result;

use catalog::Catalog;

#[derive(Args)]
pub struct Target {
    /// Session whose `.tmux/` to use; defaults to the current one
    #[arg(short = 't', long, value_name = "SESSION")]
    target: Option<String>,
}

impl Target {
    fn catalog(&self, tmux: &Tmux) -> Result<Catalog> {
        Catalog::of_session(tmux, self.target.as_deref())
    }
}

#[derive(Subcommand)]
pub enum Command {
    /// Open the fzf picker over the session's tasks and run the one chosen
    Pick(Target),
    /// Run a task in the placement its `# tmux:` header asks for
    Run {
        #[command(flatten)]
        target: Target,
        /// Task name, relative to `.tmux/`, e.g. `android/build`
        name: String,
    },
    /// Print the picker rows: tasks of the current group, most recently run first
    List(Target),
    /// Move the picker to the next group: all, then each subfolder of `.tmux/`
    Advance(Target),
    /// Print the picker prompt, which names the current group
    Prompt(Target),
    /// Print the picker header: the keys that apply to this project
    Header(Target),
    #[command(hide = true)]
    Exec {
        #[arg(long)]
        mark: bool,
        root: PathBuf,
        name: String,
    },
}

impl Command {
    pub fn run(self, tmux: &Tmux) -> Result<()> {
        match self {
            Command::Pick(target) => picker::pick(tmux, &target.catalog(tmux)?)?,
            Command::Run { target, name } => {
                runner::place(tmux, &target.catalog(tmux)?, &name, false)?
            }
            Command::List(target) => {
                for row in target.catalog(tmux)?.rows() {
                    println!("{row}");
                }
            }
            Command::Advance(target) => target.catalog(tmux)?.advance_group()?,
            Command::Prompt(target) => println!("{}", picker::prompt(&target.catalog(tmux)?)),
            Command::Header(target) => println!("{}", picker::header(&target.catalog(tmux)?)),
            Command::Exec { mark, root, name } => runner::execute(tmux, &root, &name, mark)?,
        }
        Ok(())
    }
}
