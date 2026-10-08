mod battery;
mod left;
mod right;

use std::path::Path;

use clap::Subcommand;

use crate::git;
use crate::session;
use crate::tmux::Tmux;
use crate::Result;

#[derive(Subcommand)]
pub enum Command {
    /// Print the left block of the status bar: the project and root/wt pills
    Left {
        #[arg(short = 't', long, value_name = "SESSION")]
        target: String,
        #[arg(default_value_t = 200)]
        client_width: u16,
    },
    /// Print the right block of the status bar
    Right {
        /// The session's directory, for the repo counts
        #[arg(default_value = "")]
        path: String,
        #[arg(default_value_t = 200)]
        client_width: u16,
        #[arg(default_value = "root")]
        key_table: String,
    },
}

impl Command {
    pub fn run(self, tmux: &Tmux) -> Result<()> {
        match self {
            Command::Left {
                target,
                client_width,
            } => {
                let identity = session::resolve(tmux, &target)?;
                let block = left::Left {
                    project: &identity.project,
                    checkout: identity.checkout,
                };
                let right_width = right::width(client_width, battery::Battery::read().is_some());
                println!("{}", block.render(client_width, right_width));
                Ok(())
            }
            Command::Right {
                path,
                client_width,
                key_table,
            } => {
                let block = right::Right {
                    key_table: &key_table,
                    repo: git::repo_counts(Path::new(&path)),
                    battery: battery::Battery::read(),
                    now: chrono::Local::now().naive_local(),
                };
                println!("{}", block.render(client_width));
                Ok(())
            }
        }
    }
}
