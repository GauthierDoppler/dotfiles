use std::path::{Path, PathBuf};

use crate::git;
use crate::tmux::Tmux;
use crate::Result;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Checkout {
    Root,
    Worktree,
}

#[derive(Clone)]
pub struct Identity {
    pub project: String,
    pub root: Option<PathBuf>,
    pub checkout: Option<Checkout>,
}

#[derive(Clone, Copy)]
pub enum Field {
    GroveProject,
    GroveRoot,
    GroveWorktree,
    Path,
    Name,
}

impl Field {
    pub const ALL: [Field; 5] = [
        Field::GroveProject,
        Field::GroveRoot,
        Field::GroveWorktree,
        Field::Path,
        Field::Name,
    ];

    pub fn variable(self) -> &'static str {
        match self {
            Field::GroveProject => "@grove_project",
            Field::GroveRoot => "@grove_root",
            Field::GroveWorktree => "@grove_worktree",
            Field::Path => "session_path",
            Field::Name => "session_name",
        }
    }
}

pub fn resolve(tmux: &Tmux, target: &str) -> Result<Identity> {
    let identity = identify(|field| tmux.display(target, &format!("#{{{}}}", field.variable())))?;
    identity.ok_or_else(|| format!("no session {target}").into())
}

pub fn identify(mut read: impl FnMut(Field) -> Result<String>) -> Result<Option<Identity>> {
    let grove_project = read(Field::GroveProject)?;
    if !grove_project.is_empty() {
        let grove_root = read(Field::GroveRoot)?;
        let grove_worktree = read(Field::GroveWorktree)?;
        return Ok(Some(Identity {
            project: grove_project,
            root: (!grove_root.is_empty()).then(|| PathBuf::from(grove_root)),
            checkout: Some(if grove_worktree.is_empty() {
                Checkout::Root
            } else {
                Checkout::Worktree
            }),
        }));
    }
    let path = read(Field::Path)?;
    if !path.is_empty() {
        let path = Path::new(&path);
        if let Some(root) = git::main_worktree(path) {
            if let Some(project) = root.file_name() {
                return Ok(Some(Identity {
                    project: project.to_string_lossy().into_owned(),
                    checkout: git::is_linked_worktree(path).map(|linked| {
                        if linked {
                            Checkout::Worktree
                        } else {
                            Checkout::Root
                        }
                    }),
                    root: Some(root),
                }));
            }
        }
    }
    let name = read(Field::Name)?;
    Ok((!name.is_empty()).then_some(Identity {
        project: name,
        root: None,
        checkout: None,
    }))
}
