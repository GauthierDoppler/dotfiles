mod battery;
mod right;

use std::path::Path;

use clap::Subcommand;

use crate::git;
use crate::session;
use crate::tmux::Tmux;
use crate::Result;

#[derive(Subcommand)]
pub enum Command {
    /// Print the left block of the status bar: the session's project
    Left {
        #[arg(short = 't', long, value_name = "SESSION")]
        target: String,
    },
    /// Print the right block of the status bar, or with --width only its width
    Right {
        /// Print the block's width in cells for this client width, and nothing else
        #[arg(long, value_name = "CLIENT_WIDTH", conflicts_with_all = ["path", "client_width", "key_table"])]
        width: Option<u16>,
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
            Command::Left { target } => {
                println!("{}", session::resolve(tmux, &target)?.project);
                Ok(())
            }
            Command::Right {
                width: Some(client_width),
                ..
            } => {
                println!(
                    "{}",
                    right::width(client_width, battery::Battery::read().is_some())
                );
                Ok(())
            }
            Command::Right {
                width: None,
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
