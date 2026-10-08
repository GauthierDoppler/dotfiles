use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

#[derive(Clone, Copy, Default)]
pub struct RepoCounts {
    pub insertions: u64,
    pub deletions: u64,
    pub ahead: u64,
    pub behind: u64,
}

pub fn repo_counts(dir: &Path) -> RepoCounts {
    if dir.as_os_str().is_empty() || !dir.is_dir() {
        return RepoCounts::default();
    }
    let Some(shortstat) = unlocked(dir, &["diff-index", "--shortstat", "HEAD"]) else {
        return RepoCounts::default();
    };
    let mut counts = RepoCounts::default();
    for part in shortstat.split(',').map(str::trim) {
        let Some((number, label)) = part.split_once(' ') else {
            continue;
        };
        let Ok(number) = number.parse() else {
            continue;
        };
        if label.starts_with("insertion") {
            counts.insertions = number;
        } else if label.starts_with("deletion") {
            counts.deletions = number;
        }
    }
    if let Some(divergence) = unlocked(
        dir,
        &["rev-list", "--left-right", "--count", "@{upstream}...HEAD"],
    ) {
        let mut sides = divergence.split_whitespace().map(str::parse);
        if let (Some(Ok(behind)), Some(Ok(ahead))) = (sides.next(), sides.next()) {
            counts.behind = behind;
            counts.ahead = ahead;
        }
    }
    counts
}

fn unlocked(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("--no-optional-locks")
        .args(args)
        .stderr(Stdio::null())
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}
