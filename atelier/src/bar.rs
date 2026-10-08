use clap::Subcommand;

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
}

impl Command {
    pub fn run(self, tmux: &Tmux) -> Result<()> {
        match self {
            Command::Left { target } => {
                println!("{}", session::resolve(tmux, &target)?.project);
                Ok(())
            }
        }
    }
}
