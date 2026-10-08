use std::path::{Path, PathBuf};

use crate::git;
use crate::tmux::Tmux;
use crate::Result;

pub struct Identity {
    pub project: String,
    pub root: Option<PathBuf>,
}

pub fn resolve(tmux: &Tmux, target: &str) -> Result<Identity> {
    let grove_project = tmux.display(target, "#{@grove_project}")?;
    if !grove_project.is_empty() {
        let grove_root = tmux.display(target, "#{@grove_root}")?;
        return Ok(Identity {
            project: grove_project,
            root: (!grove_root.is_empty()).then(|| PathBuf::from(grove_root)),
        });
    }
    let path = tmux.display(target, "#{session_path}")?;
    if !path.is_empty() {
        if let Some(root) = git::main_worktree(Path::new(&path)) {
            if let Some(project) = root.file_name() {
                return Ok(Identity {
                    project: project.to_string_lossy().into_owned(),
                    root: Some(root),
                });
            }
        }
    }
    let name = tmux.display(target, "#{session_name}")?;
    if name.is_empty() {
        return Err(format!("no session {target}").into());
    }
    Ok(Identity {
        project: name,
        root: None,
    })
}
