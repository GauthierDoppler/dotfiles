use std::path::{Path, PathBuf};

use crate::tmux::Tmux;
use crate::Result;

pub(super) fn absolute(tmux: &Tmux, pane: &str, token: &str) -> Result<PathBuf> {
    Ok(expand(
        Path::new(&tmux.display(pane, "#{pane_current_path}")?),
        token,
    ))
}

pub(super) fn expand(cwd: &Path, path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return Path::new(&home).join(rest);
        }
    }
    cwd.join(path)
}
