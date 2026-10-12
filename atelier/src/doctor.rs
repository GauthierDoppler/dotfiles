mod check;
mod machine;
mod services;
mod terminal;
mod tmux;
mod tools;
mod version;

use clap::Args;

use crate::service::Target;
use crate::tmux::Tmux;
use crate::Result;

use check::{Check, Context, Outcome};

#[derive(Args)]
pub struct Command {
    #[command(flatten)]
    target: Target,
}

crate::flags_only!(Command);

const CHECKS: &[(&str, Check)] = &[
    ("terminfo", terminal::terminfo),
    ("nerd font", terminal::nerd_font),
    ("tmux version", tmux::tmux_version),
    ("extended keys", tmux::extended_keys),
    ("terminal features", tmux::terminal_features),
    ("fzf", tools::fzf),
    ("atelier installed", tools::installed),
    ("setup", machine::setup),
    ("keyboard layout", machine::keyboard_layout),
    ("daemon", services::daemon),
    ("services", services::services),
    ("linger", services::linger),
];

impl Command {
    pub fn run(self, tmux: &Tmux) -> Result<()> {
        let context = Context {
            tmux,
            server: tmux.run(&["display-message", "-p", "#{pid}"]).is_ok(),
            home: crate::dotfiles::home()?,
            target: &self.target,
        };
        let mut failed = 0;
        for (name, check) in CHECKS {
            match check(&context) {
                Outcome::Ok(detail) => println!("ok    {name:<18} {detail}"),
                Outcome::Skip(why) => println!("skip  {name:<18} {why}"),
                Outcome::Look(what) => println!("look  {name:<18} {what}"),
                Outcome::Fail { problem, fix } => {
                    failed += 1;
                    println!("FAIL  {name:<18} {problem}");
                    println!("      fix: {fix}");
                }
            }
        }
        match failed {
            0 => Ok(()),
            1 => Err("1 check failed".into()),
            n => Err(format!("{n} checks failed").into()),
        }
    }
}
