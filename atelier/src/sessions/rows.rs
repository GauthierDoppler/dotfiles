use std::path::Path;

use crate::session;
use crate::tmux::Tmux;
use crate::Result;

const NO_OTHER_SESSION: &str = "(no other session)";
const NO_OTHER_IN_PROJECT: &str = "(no other session in this project)";

pub(super) fn rows(tmux: &Tmux, current: &str, project: Option<&Path>) -> Result<Vec<String>> {
    let clients = tmux.run(&[
        "list-clients",
        "-F",
        "#{client_control_mode} #{session_id}",
    ])?;
    let watched: Vec<&str> = clients
        .lines()
        .filter_map(|line| line.strip_prefix("0 "))
        .collect();
    let mut listed = Vec::new();
    for id in tmux.run(&["list-sessions", "-F", "#{session_id}"])?.lines() {
        if id != current && in_project(tmux, id, project)? {
            listed.push(Listed::read(tmux, id, watched.contains(&id))?);
        }
    }
    if listed.is_empty() {
        let notice = if project.is_some() {
            NO_OTHER_IN_PROJECT
        } else {
            NO_OTHER_SESSION
        };
        return Ok(vec![format!("\t{notice}")]);
    }
    listed.sort_by(|a, b| {
        b.last_attached
            .cmp(&a.last_attached)
            .then_with(|| a.name.cmp(&b.name))
    });
    let width = listed
        .iter()
        .map(|session| session.name.chars().count())
        .max()
        .unwrap_or(0);
    Ok(listed.iter().map(|session| session.row(width)).collect())
}

fn in_project(tmux: &Tmux, id: &str, project: Option<&Path>) -> Result<bool> {
    Ok(match project {
        Some(root) => session::resolve(tmux, id)?.root.as_deref() == Some(root),
        None => true,
    })
}

struct Listed {
    id: String,
    name: String,
    windows: String,
    attached: bool,
    last_attached: u64,
}

impl Listed {
    fn read(tmux: &Tmux, id: &str, attached: bool) -> Result<Self> {
        let line = tmux.display(
            id,
            "#{session_last_attached} #{session_windows} #{session_name}",
        )?;
        let mut fields = line.splitn(3, ' ');
        let mut next = || fields.next().unwrap_or_default().to_string();
        let last_attached = next().parse().unwrap_or(0);
        let windows = next();
        Ok(Listed {
            id: id.to_string(),
            name: next(),
            windows,
            attached,
            last_attached,
        })
    }

    fn row(&self, width: usize) -> String {
        let attached = if self.attached { "  · attached" } else { "" };
        format!(
            "{}\t{:<width$}  {:>3} win{attached}",
            self.id, self.name, self.windows
        )
    }
}
