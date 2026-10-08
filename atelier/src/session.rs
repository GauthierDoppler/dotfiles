use std::path::{Path, PathBuf};

use crate::git;
use crate::tmux::Tmux;
use crate::Result;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Checkout {
    Root,
    Worktree,
}

pub struct Identity {
    pub project: String,
    pub root: Option<PathBuf>,
    pub checkout: Option<Checkout>,
}

pub fn resolve(tmux: &Tmux, target: &str) -> Result<Identity> {
    let grove_project = tmux.display(target, "#{@grove_project}")?;
    if !grove_project.is_empty() {
        let grove_root = tmux.display(target, "#{@grove_root}")?;
        let grove_worktree = tmux.display(target, "#{@grove_worktree}")?;
        return Ok(Identity {
            project: grove_project,
            root: (!grove_root.is_empty()).then(|| PathBuf::from(grove_root)),
            checkout: Some(if grove_worktree.is_empty() {
                Checkout::Root
            } else {
                Checkout::Worktree
            }),
        });
    }
    let path = tmux.display(target, "#{session_path}")?;
    if !path.is_empty() {
        let path = Path::new(&path);
        if let Some(root) = git::main_worktree(path) {
            if let Some(project) = root.file_name() {
                return Ok(Identity {
                    project: project.to_string_lossy().into_owned(),
                    checkout: git::is_linked_worktree(path).map(|linked| {
                        if linked {
                            Checkout::Worktree
                        } else {
                            Checkout::Root
                        }
                    }),
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
        checkout: None,
    })
}
