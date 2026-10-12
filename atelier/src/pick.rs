mod actions;
mod paths;
mod picker;
mod rows;
mod tokens;

use clap::Subcommand;

use crate::tmux::Tmux;
use crate::Result;

use rows::PLACEHOLDER;

#[derive(Subcommand)]
pub enum Command {
    /// Print the URLs and existing paths on a pane, newest first
    List {
        #[arg(short = 't', long, value_name = "PANE")]
        target: Option<String>,
    },
    /// Open a token: a URL in the browser, a file in this session's nvim
    Open {
        #[arg(short = 't', long, value_name = "PANE")]
        target: Option<String>,
        token: String,
    },
    /// Hand a token to the OS opener
    System {
        #[arg(short = 't', long, value_name = "PANE")]
        target: Option<String>,
        token: String,
    },
    /// Open a markdown path in the preview; anything else does nothing
    Preview {
        #[arg(short = 't', long, value_name = "PANE")]
        target: Option<String>,
        token: String,
    },
    /// Copy a token to the clipboard through tmux (OSC 52)
    Copy { token: String },
    /// Run the fzf picker over a pane, for `display-popup -E`
    Popup {
        #[arg(short = 't', long, value_name = "PANE")]
        target: Option<String>,
    },
}


impl Command {
    pub fn run(self, tmux: &Tmux) -> Result<()> {
        match self {
            Command::List { target } => {
                for row in rows::rows(tmux, &resolve_pane(tmux, &target)?)? {
                    println!("{row}");
                }
                Ok(())
            }
            Command::Popup { target } => picker::popup(tmux, &resolve_pane(tmux, &target)?),
            Command::Open { token, .. }
            | Command::System { token, .. }
            | Command::Preview { token, .. }
            | Command::Copy { token }
                if token == PLACEHOLDER =>
            {
                Ok(())
            }
            Command::Open { target, token } => {
                actions::open(tmux, &resolve_pane(tmux, &target)?, &token)
            }
            Command::System { target, token } => actions::system(tmux, &target, &token),
            Command::Preview { target, token } => actions::preview(tmux, &target, &token),
            Command::Copy { token } => actions::copy(tmux, &token),
        }
    }
}

fn resolve_pane(tmux: &Tmux, target: &Option<String>) -> Result<String> {
    tmux.resolve(target.as_deref(), "#{pane_id}")
}
