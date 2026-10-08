use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
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

pub fn is_linked_worktree(dir: &Path) -> Option<bool> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "rev-parse",
            "--path-format=absolute",
            "--git-dir",
            "--git-common-dir",
        ])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let listing = String::from_utf8(output.stdout).ok()?;
    let mut lines = listing.lines();
    Some(lines.next()? != lines.next()?)
}

#[derive(Clone, Copy, Default)]
pub struct RepoCounts {
    pub insertions: u64,
    pub deletions: u64,
    pub ahead: u64,
    pub behind: u64,
}

pub fn repo_counts(dir: &Path) -> RepoCounts {
    if !dir.is_dir() {
        return RepoCounts::default();
    }
    let Some(shortstat) = read_only_git(dir, &["diff-index", "--shortstat", "HEAD"]) else {
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
    if let Some(divergence) = read_only_git(
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

pub fn tracked_files(worktree: &Path) -> Option<Vec<PathBuf>> {
    let listing = read_only_git_bytes(worktree, &["ls-files", "-z"])?;
    Some(
        listing
            .split(|byte| *byte == 0)
            .filter(|name| !name.is_empty())
            .map(|name| worktree.join(OsStr::from_bytes(name)))
            .collect(),
    )
}

fn read_only_git(dir: &Path, args: &[&str]) -> Option<String> {
    read_only_git_bytes(dir, args).map(|stdout| String::from_utf8_lossy(&stdout).into_owned())
}

fn read_only_git_bytes(dir: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("--no-optional-locks")
        .args(args)
        .stderr(Stdio::null())
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}
