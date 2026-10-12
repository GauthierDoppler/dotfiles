use std::path::Path;

use crate::opener;
use crate::tmux::Tmux;
use crate::Result;

use super::paths::absolute;
use super::resolve_pane;
use super::tokens::is_url;

pub(super) fn open(tmux: &Tmux, pane: &str, token: &str) -> Result<()> {
    if is_url(token) {
        return opener::open(token);
    }
    let path = absolute(tmux, pane, token)?;
    let session = tmux.display(pane, "#{session_id}")?;
    let panes = tmux.run(&[
        "list-panes",
        "-s",
        "-t",
        &session,
        "-F",
        "#{pane_id} #{pane_current_command}",
    ])?;
    let nvim = panes
        .lines()
        .filter_map(|line| line.split_once(' '))
        .find(|(_, command)| *command == "nvim")
        .map(|(id, _)| id);
    let path_str = path.to_string_lossy();
    if let Some(nvim) = nvim {
        tmux.run(&["send-keys", "-t", nvim, "Escape"])?;
        tmux.run(&[
            "send-keys",
            "-t",
            nvim,
            "-l",
            &format!(":e {}", vim_escape(&path_str)),
        ])?;
        tmux.run(&["send-keys", "-t", nvim, "Enter"])?;
        tmux.run(&["select-window", "-t", nvim])?;
        tmux.run(&["select-pane", "-t", nvim])?;
        return Ok(());
    }
    if path.is_dir() {
        return opener::open(&path);
    }
    let root = tmux.display(pane, "#{session_path}")?;
    let root = if Path::new(&root).is_dir() {
        root
    } else {
        path.parent()
            .map_or_else(String::new, |dir| dir.to_string_lossy().into_owned())
    };
    tmux.run(&[
        "new-window",
        "-t",
        &format!("{session}:"),
        "-c",
        &root,
        "-n",
        "nvim",
        "--",
        "nvim",
        &path_str,
    ])?;
    Ok(())
}

pub(super) fn system(tmux: &Tmux, target: &Option<String>, token: &str) -> Result<()> {
    if is_url(token) {
        return opener::open(token);
    }
    let pane = resolve_pane(tmux, target)?;
    opener::open(&absolute(tmux, &pane, token)?)
}

pub(super) fn preview(tmux: &Tmux, target: &Option<String>, token: &str) -> Result<()> {
    if is_url(token) || !crate::preview::is_markdown(Path::new(token)) {
        return Ok(());
    }
    let pane = resolve_pane(tmux, target)?;
    crate::preview::open(&absolute(tmux, &pane, token)?)
}

pub(super) fn copy(tmux: &Tmux, token: &str) -> Result<()> {
    tmux.run(&["set-buffer", "-w", "--", token])?;
    Ok(())
}

fn vim_escape(path: &str) -> String {
    let mut escaped = String::with_capacity(path.len());
    for c in path.chars() {
        if " \t*?[{`$\\%#'\"|!<".contains(c) {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}
