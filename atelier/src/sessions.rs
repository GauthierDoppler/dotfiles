mod picker;
mod preview;
mod rows;
mod scope;
mod windows;

use clap::Subcommand;

use crate::tmux::Tmux;
use crate::Result;

use picker::Picker;

#[derive(Subcommand)]
pub enum Command {
    /// Open the session picker in fzf; meant for `display-popup -E`
    Pick {
        /// The session the picker is opened from; defaults to the current one
        #[arg(short = 't', long, value_name = "SESSION")]
        target: Option<String>,
        /// The client to switch; defaults to the current one
        #[arg(short = 'c', long, value_name = "CLIENT")]
        client: Option<String>,
    },
    /// Print the picker's rows, one per other session: its id, a tab, its label
    Rows {
        #[arg(short = 't', long, value_name = "SESSION")]
        target: String,
    },
    /// Toggle between this project's sessions and all of them; prints fzf actions
    Toggle {
        #[arg(short = 't', long, value_name = "SESSION")]
        target: String,
    },
    /// Print the picker's key help for the current scope
    Header {
        #[arg(short = 't', long, value_name = "SESSION")]
        target: String,
    },
    /// Leave search, or close the picker when not searching; prints fzf actions
    Escape {
        #[arg(short = 't', long, value_name = "SESSION")]
        target: String,
    },
    /// Switch a client to a session; an empty session does nothing
    Switch {
        #[arg(short = 'c', long, value_name = "CLIENT")]
        client: Option<String>,
        session: String,
    },
    /// Print a session's window list and the tail of its current window
    Preview { session: String },
    /// Move a session to its next window; the picker's `l`
    Next { session: String },
    /// Move a session to its previous window; the picker's `h`
    Prev { session: String },
    /// Kill a session and reload the picker's rows; prints fzf actions
    Kill {
        #[arg(short = 't', long, value_name = "SESSION")]
        target: String,
        session: String,
    },
}

impl Command {
    pub fn run(self, tmux: &Tmux) -> Result<()> {
        match self {
            Command::Pick { target, client } => picker::pick(tmux, target, client),
            Command::Rows { target } => {
                for row in Picker::open(tmux, &target)?.rows()? {
                    println!("{row}");
                }
                Ok(())
            }
            Command::Toggle { target } => {
                print_actions(&Picker::open(tmux, &target)?.toggle()?);
                Ok(())
            }
            Command::Header { target } => {
                println!("{}", Picker::open(tmux, &target)?.header()?);
                Ok(())
            }
            Command::Escape { target } => {
                let input_state = std::env::var("FZF_INPUT_STATE").unwrap_or_default();
                print_actions(&Picker::open(tmux, &target)?.escape(&input_state)?);
                Ok(())
            }
            Command::Switch { client, session } => picker::switch(tmux, client.as_deref(), &session),
            Command::Preview { session } => {
                let lines = std::env::var("FZF_PREVIEW_LINES")
                    .ok()
                    .and_then(|lines| lines.parse().ok())
                    .unwrap_or(24);
                print!("{}", preview::preview(tmux, session.trim(), lines)?);
                Ok(())
            }
            Command::Next { session } => windows::cycle(tmux, "next-window", session.trim()),
            Command::Prev { session } => windows::cycle(tmux, "previous-window", session.trim()),
            Command::Kill { target, session } => {
                print_actions(&Picker::open(tmux, &target)?.kill(session.trim())?);
                Ok(())
            }
        }
    }
}

fn print_actions(actions: &str) {
    if !actions.is_empty() {
        println!("{actions}");
    }
}
