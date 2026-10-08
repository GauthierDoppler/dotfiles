use std::process::Stdio;

use crate::fzf;
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
    let atelier = fzf::atelier(tmux)?;
    let callback =
        |command: &str| format!("{atelier} tasks {command} -t {}", shell::quote(&catalog.session));
    let dir = shell::quote(&catalog.dir().to_string_lossy());
    let preview = format!(
        "f={{1}}; d={dir}; [ -f \"$d/$f\" ] || exit 0; if command -v bat >/dev/null 2>&1; then bat --color=always --style=plain --line-range=:200 \"$d/$f\"; else cat \"$d/$f\"; fi"
    );
    let tab = format!(
        "--bind=tab:transform:{}; echo \"reload({})+transform-prompt({})+transform-header({})+first\"",
        callback("advance"),
        callback("list"),
        callback("prompt"),
        callback("header")
    );
    let edit = format!(
        "--bind=ctrl-e:execute(f={{1}}; d={dir}; [ -f \"$d/$f\" ] && ${{EDITOR:-vi}} \"$d/$f\")"
    );

    let mut picker = fzf::picker(&prompt(catalog), header(catalog), fzf::MODAL_KEYS);
    picker
        .arg(fzf::leave_search(
            fzf::MODAL_KEYS,
            &format!("transform-prompt({})", callback("prompt")),
        ))
        .args([tab, edit])
        .arg(format!("--preview={preview}"))
        .arg("--preview-window=right,40%,border-left")
        .stdout(Stdio::piped());
    let output = fzf::spawn(&mut picker, catalog.rows())?.wait_with_output()?;
    let selected = String::from_utf8_lossy(&output.stdout);
    let selected = selected.trim();
    if !output.status.success() || [NO_DIRECTORY, NO_TASK, ""].contains(&selected) {
        return Ok(());
    }
    let name = selected.split_whitespace().next().unwrap_or_default();
    runner::place(tmux, catalog, name, true)
}
