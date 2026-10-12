use std::fs;
use std::path::{Path, PathBuf};

use super::action::Action;

pub(super) fn backup(path: &Path) -> Action {
    Action::Backup {
        path: path.to_path_buf(),
        to: free_path(path, ".bak"),
    }
}

pub(super) fn free_path(path: &Path, suffix: &str) -> PathBuf {
    let mut candidate = suffixed(path, suffix);
    let mut n = 1;
    while fs::symlink_metadata(&candidate).is_ok() {
        candidate = suffixed(path, &format!("{suffix}.{n}"));
        n += 1;
    }
    candidate
}

fn suffixed(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}
