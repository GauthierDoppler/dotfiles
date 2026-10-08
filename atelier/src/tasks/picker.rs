use std::io::Write;
use std::process::{Command, Stdio};

use crate::shell;
use crate::tmux::Tmux;
use crate::Result;

use super::catalog::{Catalog, NO_DIRECTORY, NO_TASK};
use super::runner;

pub fn prompt(catalog: &Catalog) -> String {
    format!("  {}  ", catalog.group())
}

pub fn header(catalog: &Catalog) -> &'static str {
    if catalog.groups().is_empty() {
        "j/k move   enter run   i search   ctrl-e edit"
    } else {
        "j/k move   enter run   tab group   i search   ctrl-e edit"
    }
}

pub fn pick(tmux: &Tmux, catalog: &Catalog) -> Result<()> {
    catalog.set_group("all")?;
    let mut atelier = shell::quote(&std::env::current_exe()?.to_string_lossy());
    if let Some(socket) = tmux.socket() {
        atelier.push_str(&format!(" --socket {}", shell::quote(&socket.to_string_lossy())));
    }
    let callback =
        |command: &str| format!("{atelier} tasks {command} -t {}", shell::quote(&catalog.session));
    let dir = shell::quote(&catalog.dir().to_string_lossy());
    let preview = format!(
        "f={{1}}; d={dir}; [ -f \"$d/$f\" ] || exit 0; if command -v bat >/dev/null 2>&1; then bat --color=always --style=plain --line-range=:200 \"$d/$f\"; else cat \"$d/$f\"; fi"
    );
    let esc = format!(
        "esc:transform:[ \"$FZF_INPUT_STATE\" = enabled ] && echo 'disable-search+clear-query+rebind(j,k,q)+transform-prompt({})' || echo abort",
        callback("prompt")
    );
    let tab = format!(
        "tab:transform:{}; echo \"reload({})+transform-prompt({})+transform-header({})+first\"",
        callback("advance"),
        callback("list"),
        callback("prompt"),
        callback("header")
    );
    let edit = format!(
        "ctrl-e:execute(f={{1}}; d={dir}; [ -f \"$d/$f\" ] && ${{EDITOR:-vi}} \"$d/$f\")"
    );

    let mut fzf = Command::new("fzf")
        .args(["--disabled", "--header-first", "--pointer=▸", "--no-multi", "--cycle"])
        .arg(format!("--prompt={}", prompt(catalog)))
        .arg(format!("--header={}", header(catalog)))
        .args(["--bind", "j:down,k:up", "--bind", "q:abort"])
        .args([
            "--bind",
            "i:enable-search+unbind(j,k,q)+change-prompt(  search  )",
        ])
        .args(["--bind", &esc, "--bind", &tab, "--bind", &edit])
        .arg(format!("--preview={preview}"))
        .arg("--preview-window=right,40%,border-left")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    if let Some(mut input) = fzf.stdin.take() {
        for row in catalog.rows() {
            writeln!(input, "{row}")?;
        }
    }
    let output = fzf.wait_with_output()?;
    let selected = String::from_utf8_lossy(&output.stdout);
    let selected = selected.trim();
    if !output.status.success() || [NO_DIRECTORY, NO_TASK, ""].contains(&selected) {
        return Ok(());
    }
    let name = selected.split_whitespace().next().unwrap_or_default();
    runner::place(tmux, catalog, name, true)
}
