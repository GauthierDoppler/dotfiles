use std::path::{Path, PathBuf};
use std::process::Command;

pub fn main_worktree(dir: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["worktree", "list", "--porcelain", "-z"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let listing = String::from_utf8(output.stdout).ok()?;
    let first = listing.split('\0').next()?;
    first.strip_prefix("worktree ").map(PathBuf::from)
}
