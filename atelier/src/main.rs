mod git;
mod session;
mod tmux;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::tmux::Tmux;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

macro_rules! features {
    ($($(#[$doc:meta])* $variant:ident => $module:ident,)*) => {
        $(mod $module;)*

        #[derive(Subcommand)]
        enum Feature {
            $($(#[$doc])* #[command(subcommand)] $variant($module::Command),)*
        }

        impl Feature {
            fn run(self, tmux: &Tmux) -> Result<()> {
                match self {
                    $(Feature::$variant(command) => command.run(tmux),)*
                }
            }
        }
    };
}

features! {
    /// The tmux status bar
    Bar => bar,
    /// The session picker
    Sessions => sessions,
}

#[derive(Parser)]
#[command(version, about = "Orchestrates the tmux workstation")]
struct Cli {
    /// tmux server socket; defaults to the one in $TMUX
    #[arg(short = 'S', long, global = true, value_name = "PATH")]
    socket: Option<PathBuf>,
    #[command(subcommand)]
    feature: Feature,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.feature.run(&Tmux::new(cli.socket)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("atelier: {error}");
            ExitCode::FAILURE
        }
    }
}
