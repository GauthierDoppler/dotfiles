use std::path::{Path, PathBuf};

use crate::service::Target;
use crate::tmux::Tmux;
use crate::Result;

pub(super) type Check = fn(&Context) -> Outcome;

pub(super) struct Context<'a> {
    pub(super) tmux: &'a Tmux,
    pub(super) server: bool,
    pub(super) home: PathBuf,
    pub(super) target: &'a Target,
}

impl Context<'_> {
    pub(super) fn repo(&self) -> Result<PathBuf> {
        crate::dotfiles::repo(self.target.repo(), &self.home)
    }

    pub(super) fn tilde(&self, path: &Path) -> String {
        match path.strip_prefix(&self.home) {
            Ok(relative) => format!("~/{}", relative.display()),
            Err(_) => path.display().to_string(),
        }
    }
}

pub(super) enum Outcome {
    Ok(String),
    Fail { problem: String, fix: String },
    Look(String),
    Skip(String),
}

pub(super) fn fail(problem: impl Into<String>, fix: impl Into<String>) -> Outcome {
    Outcome::Fail {
        problem: problem.into(),
        fix: fix.into(),
    }
}

pub(super) fn no_server() -> Outcome {
    Outcome::Skip("no tmux server to ask".into())
}
