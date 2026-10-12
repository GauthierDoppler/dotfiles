mod dotfiles;
mod fnv;
mod fzf;
mod git;
mod opener;
mod process;
mod session;
mod shell;
mod tmux;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::tmux::Tmux;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

macro_rules! flags_only {
    ($args:ty) => {
        impl clap::Subcommand for $args {
            fn augment_subcommands(command: clap::Command) -> clap::Command {
                <Self as clap::Args>::augment_args(command)
                    .subcommand_required(false)
                    .arg_required_else_help(false)
            }

            fn augment_subcommands_for_update(command: clap::Command) -> clap::Command {
                <Self as clap::Args>::augment_args_for_update(command)
                    .subcommand_required(false)
                    .arg_required_else_help(false)
            }

            fn has_subcommand(_name: &str) -> bool {
                false
            }
        }
    };
}
pub(crate) use flags_only;

macro_rules! features {
    (
        $($(#[$doc:meta])* $variant:ident => $module:ident,)*
        $(; top level: $($tvariant:ident => $tmodule:ident,)*)?
    ) => {
        $(mod $module;)*
        $($(mod $tmodule;)*)?

        #[derive(Subcommand)]
        enum Feature {
            $($(#[$doc])* #[command(subcommand)] $variant($module::Command),)*
            $($(#[command(flatten)] $tvariant($tmodule::Command),)*)?
        }

        impl Feature {
            fn run(self, tmux: &Tmux) -> Result<()> {
                match self {
                    $(Feature::$variant(command) => command.run(tmux),)*
                    $($(Feature::$tvariant(command) => command.run(tmux),)*)?
                }
            }
        }
    };
}

features! {
    /// The tmux status bar
    Bar => bar,
    /// Regenerate ~/.claude/settings.json from the shared base, local overrides and app drift
    ClaudeSettingsSync => claude_settings,
    /// Check what this machine needs and print the fix for each failure
    Doctor => doctor,
    /// Entry points for other tools' hooks
    Hook => hook,
    /// List what this machine's config adds beyond the shared dotfiles
    LocalDiff => local_diff,
    /// Link the dotfiles into $HOME and write the shell and git stubs
    Setup => setup,
    /// Markdown preview: `atelier preview <file.md>` or `atelier preview serve`
    Preview => preview,
    /// Pick a URL or file path off a pane
    Pick => pick,
    /// Background services from services.toml, as launchd agents or systemd user units
    Service => service,
    /// The session picker
    Sessions => sessions,
    /// The per-project task picker and runner over `.tmux/`
    Tasks => tasks,
    ; top level:
    Daemon => daemon,
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
