use std::path::Path;

use crate::git;
use crate::tmux::Tmux;
use crate::Result;

pub struct Identity {
    pub project: String,
}

pub fn resolve(tmux: &Tmux, target: &str) -> Result<Identity> {
    let grove_project = tmux.display(target, "#{@grove_project}")?;
    if !grove_project.is_empty() {
        return Ok(Identity {
            project: grove_project,
        });
    }
    let path = tmux.display(target, "#{session_path}")?;
    if !path.is_empty() {
        if let Some(project) = git::main_worktree(Path::new(&path))
            .as_deref()
            .and_then(Path::file_name)
        {
            return Ok(Identity {
                project: project.to_string_lossy().into_owned(),
            });
        }
    }
    let name = tmux.display(target, "#{session_name}")?;
    if name.is_empty() {
        return Err(format!("no session {target}").into());
    }
    Ok(Identity { project: name })
}
