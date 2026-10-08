use std::path::{Path, PathBuf};
use std::process::{Command as Process, Stdio};
use std::time::SystemTime;

use crate::fzf;
use crate::tmux::Tmux;
use crate::Result;

use super::is_markdown;

pub const PLACEHOLDER: &str = "(no markdown file under this session)";
const WALK_DEPTH: usize = 6;

pub fn session_root(tmux: &Tmux, target: Option<&str>) -> Result<PathBuf> {
    let path = match target {
        Some(target) => tmux.display(target, "#{session_path}")?,
        None => tmux.run(&["display-message", "-p", "#{session_path}"])?,
    };
    if path.is_empty() {
        return Err(format!("no session {}", target.unwrap_or("attached")).into());
    }
    Ok(PathBuf::from(path))
}

pub fn rows(root: &Path) -> Vec<String> {
    let mut found: Vec<(SystemTime, String)> = candidates(root)
        .into_iter()
        .filter(|relative| is_markdown(Path::new(relative)))
        .filter_map(|relative| {
            let metadata = std::fs::metadata(root.join(&relative)).ok()?;
            metadata
                .is_file()
                .then(|| (metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH), relative))
        })
        .collect();
    if found.is_empty() {
        return vec![PLACEHOLDER.to_string()];
    }
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    found.into_iter().map(|(_, relative)| relative).collect()
}

fn candidates(root: &Path) -> Vec<String> {
    if !root.is_dir() {
        return Vec::new();
    }
    if let Some(listed) = git_files(root) {
        return listed;
    }
    let mut files = Vec::new();
    walk(root, Path::new(""), 0, &mut files);
    files
}

fn git_files(root: &Path) -> Option<Vec<String>> {
    let output = Process::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z", "-co", "--exclude-standard"])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .split('\0')
            .filter(|file| !file.is_empty())
            .map(String::from)
            .collect(),
    )
}

fn walk(root: &Path, relative: &Path, depth: usize, files: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(root.join(relative)) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = relative.join(&name);
        if kind.is_dir() {
            if depth + 1 < WALK_DEPTH {
                walk(root, &path, depth + 1, files);
            }
        } else {
            files.push(path.to_string_lossy().into_owned());
        }
    }
}

pub fn pick(tmux: &Tmux, target: Option<&str>) -> Result<()> {
    let root = session_root(tmux, target)?;
    let rows = rows(&root);
    let atelier = fzf::atelier(tmux)?;
    let mut picker = fzf::picker(
        "  markdown  ",
        "j/k move   i search\nenter preview   esc close",
        fzf::MODAL_KEYS,
    );
    picker
        .arg(fzf::leave_search(
            fzf::MODAL_KEYS,
            "change-prompt(  markdown  )",
        ))
        .arg(format!("--bind=enter:become({atelier} preview {{}})"))
        .current_dir(if root.is_dir() { &root } else { Path::new("/") });
    fzf::spawn(&mut picker, rows)?.wait()?;
    Ok(())
}

