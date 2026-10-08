use std::fmt::Display;
use std::io::Write;
use std::process::{Child, Command, Stdio};

use crate::shell;
use crate::tmux::Tmux;
use crate::Result;

pub const MODAL_KEYS: &str = "j,k,q";

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

pub fn picker(prompt: &str, header: &str, modal_keys: &str) -> Command {
    let mut fzf = Command::new("fzf");
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
    .arg(format!("--prompt={prompt}"))
    .arg(format!("--header={header}"))
    .args(["--bind=j:down,k:up", "--bind=q:abort"])
    .arg(format!(
        "--bind=i:enable-search+unbind({modal_keys})+change-prompt(  search  )"
    ))
    .stdin(Stdio::piped());
    fzf
}

// fzf's unbind removes a key's behaviour rather than restoring its default:
// unbinding esc during search would leave no way out.
pub fn leave_search(modal_keys: &str, restore_prompt: &str) -> String {
    let actions = format!("disable-search+clear-query+rebind({modal_keys})+{restore_prompt}");
    format!(
        "--bind=esc:transform:[ \"$FZF_INPUT_STATE\" = enabled ] && echo {} || echo abort",
        shell::quote(&actions)
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
