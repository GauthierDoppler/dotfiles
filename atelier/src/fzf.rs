use std::fmt::Display;
use std::io::Write;
use std::process::{Child, Command, Stdio};

use crate::shell;
use crate::tmux::Tmux;
use crate::Result;

const MODAL_KEYS: [&str; 4] = ["j", "k", "q", "i"];

const COLORS: &str = "--color=fg:#c6d0f5,fg+:#c6d0f5,bg:-1,bg+:#51576d,hl:#8caaee,hl+:#8caaee,border:#626880,header:#a5adce,info:#838ba7,prompt:#8caaee,pointer:#8caaee";

pub fn atelier_argv(tmux: &Tmux) -> Result<Vec<String>> {
    let mut argv = vec![std::env::current_exe()?.to_string_lossy().into_owned()];
    if let Some(socket) = tmux.socket() {
        argv.push("--socket".into());
        argv.push(socket.to_string_lossy().into_owned());
    }
    Ok(argv)
}

pub fn atelier(tmux: &Tmux) -> Result<String> {
    Ok(atelier_argv(tmux)?
        .iter()
        .map(|word| shell::quote(word))
        .collect::<Vec<_>>()
        .join(" "))
}

pub struct Picker<'a> {
    pub prompt: &'a str,
    pub header: &'a str,
    pub modal_keys: &'a [&'a str],
    pub prompt_command: Option<&'a str>,
}

impl Picker<'_> {
    pub fn command(&self) -> Command {
        let keys = MODAL_KEYS
            .iter()
            .chain(self.modal_keys)
            .copied()
            .collect::<Vec<_>>()
            .join(",");
        let restore_prompt = match self.prompt_command {
            Some(command) => format!("transform-prompt({command})"),
            None => format!("change-prompt({})", self.prompt),
        };
        let leave_search = format!("disable-search+rebind({keys})+{restore_prompt}");
        let mut fzf = crate::process::command("fzf");
        fzf.args([
            "--no-multi",
            "--disabled",
            "--cycle",
            "--height=100%",
            "--layout=reverse",
            "--info=inline",
            "--header-first",
            "--pointer=▸",
            COLORS,
        ])
        .arg(format!("--prompt={}", self.prompt))
        .arg(format!("--header={}", self.header))
        .args(["--bind=j:down,k:up", "--bind=q:abort"])
        .arg(format!(
            "--bind=i:enable-search+unbind({keys})+change-prompt(  search  )"
        ))
        // fzf's unbind removes a key's behaviour rather than restoring its
        // default: unbinding esc during search would leave no way out. And a
        // query cleared from inside a transform is never searched again once
        // search is disabled, leaving the list filtered: clear it before.
        .arg(format!(
            "--bind=esc:clear-query+search()+transform:[ \"$FZF_INPUT_STATE\" = enabled ] && echo {} || echo abort",
            shell::quote(&leave_search)
        ))
        .stdin(Stdio::piped());
        fzf
    }
}

pub fn header(moves: &[&str], acts: &[&str]) -> String {
    let moves = std::iter::once("j/k move")
        .chain(moves.iter().copied())
        .chain(["i search"]);
    let acts = acts.iter().copied().chain(["esc close"]);
    format!(
        "{}\n{}",
        moves.collect::<Vec<_>>().join("   "),
        acts.collect::<Vec<_>>().join("   ")
    )
}

pub fn spawn<T: Display>(fzf: &mut Command, rows: impl IntoIterator<Item = T>) -> Result<Child> {
    let mut child = fzf.spawn().map_err(|error| format!("fzf: {error}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        for row in rows {
            writeln!(stdin, "{row}")?;
        }
    }
    Ok(child)
}
