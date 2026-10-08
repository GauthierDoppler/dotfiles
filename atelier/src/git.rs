use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn read_only_git(dir: &Path, args: &[&str]) -> Option<String> {
    let mut read_only = vec!["--no-optional-locks"];
    read_only.extend_from_slice(args);
    git(dir, &read_only)
}

pub fn main_worktree(dir: &Path) -> Option<PathBuf> {
    let listing = git(dir, &["worktree", "list", "--porcelain", "-z"])?;
    let first = listing.split('\0').next()?;
    first.strip_prefix("worktree ").map(PathBuf::from)
}

pub fn is_linked_worktree(dir: &Path) -> Option<bool> {
    let listing = git(
        dir,
        &[
            "rev-parse",
            "--path-format=absolute",
            "--git-dir",
            "--git-common-dir",
        ],
    )?;
    let mut lines = listing.lines();
    Some(lines.next()? != lines.next()?)
}

pub fn toplevel(dir: &Path) -> Option<PathBuf> {
    let top = git(dir, &["rev-parse", "--show-toplevel"])?;
    let top = top.trim_end_matches('\n');
    (!top.is_empty()).then(|| PathBuf::from(top))
}

pub fn files(dir: &Path) -> Option<Vec<String>> {
    let listed = git(dir, &["ls-files", "-z", "-co", "--exclude-standard"])?;
    Some(
        listed
            .split('\0')
            .filter(|file| !file.is_empty())
            .map(String::from)
            .collect(),
    )
}

pub fn config_entries(file: &Path) -> Option<Vec<String>> {
    let file = file.to_str()?;
    let listed = git(Path::new("."), &["config", "--file", file, "--list"])?;
    Some(listed.lines().map(String::from).collect())
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
