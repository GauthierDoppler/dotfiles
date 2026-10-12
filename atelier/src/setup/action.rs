use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::Result;

pub(super) enum Action {
    Backup {
        path: PathBuf,
        to: PathBuf,
    },
    Unlink {
        path: PathBuf,
    },
    Prune {
        path: PathBuf,
        target: PathBuf,
    },
    Symlink {
        source: PathBuf,
        dest: PathBuf,
    },
    WriteStub {
        path: PathBuf,
        content: String,
    },
    Migrate {
        legacy: PathBuf,
        dest: PathBuf,
        kept: PathBuf,
    },
}

impl Action {
    pub(super) fn path(&self) -> &Path {
        match self {
            Action::Backup { path, .. }
            | Action::Unlink { path }
            | Action::Prune { path, .. }
            | Action::Symlink { dest: path, .. }
            | Action::WriteStub { path, .. }
            | Action::Migrate { dest: path, .. } => path,
        }
    }

    pub(super) fn describe(&self) -> String {
        match self {
            Action::Backup { path, to } => {
                format!("backup: {} -> {}", path.display(), to.display())
            }
            Action::Unlink { path } => format!("unlink: {}", path.display()),
            Action::Prune { path, target } => {
                format!("pruned: {} -> {}", path.display(), target.display())
            }
            Action::Symlink { source, dest } => {
                format!("linked: {} -> {}", dest.display(), source.display())
            }
            Action::WriteStub { path, .. } => format!("stubbed: {}", path.display()),
            Action::Migrate { legacy, dest, kept } => format!(
                "migrated: {} -> {} (kept as {})",
                legacy.display(),
                dest.display(),
                kept.display()
            ),
        }
    }

    pub(super) fn apply(&self) -> Result<()> {
        match self {
            Action::Backup { path, to } => fs::rename(path, to)?,
            Action::Unlink { path } | Action::Prune { path, .. } => fs::remove_file(path)?,
            Action::Symlink { source, dest } => {
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                std::os::unix::fs::symlink(source, dest)?;
            }
            Action::WriteStub { path, content } => fs::write(path, content)?,
            Action::Migrate { legacy, dest, kept } => {
                let name = legacy.file_name().unwrap_or_default().to_string_lossy();
                let mut block = format!("\n# ─── migrated from {name} ───\n").into_bytes();
                block.extend(fs::read(legacy)?);
                fs::OpenOptions::new()
                    .append(true)
                    .open(dest)?
                    .write_all(&block)?;
                fs::rename(legacy, kept)?;
            }
        }
        Ok(())
    }
}
