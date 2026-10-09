use std::path::{Path, PathBuf};

use crate::git;
use crate::Result;

pub fn home() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".into())
}

pub fn repo(flag: Option<PathBuf>, home: &Path) -> Result<PathBuf> {
    let chosen = flag.or_else(|| {
        std::env::var_os("DOTFILES")
            .filter(|dir| !dir.is_empty())
            .map(PathBuf::from)
    });
    if let Some(repo) = chosen {
        let repo = std::path::absolute(repo)?;
        return if is_dotfiles(&repo) {
            Ok(repo)
        } else {
            Err(format!("{} is not a dotfiles checkout", repo.display()).into())
        };
    }
    let candidates = git::toplevel(Path::new("."))
        .into_iter()
        .chain([home.join("dotfiles")]);
    for repo in candidates {
        if is_dotfiles(&repo) {
            return Ok(repo);
        }
    }
    Err("no dotfiles checkout found: pass --repo".into())
}

fn is_dotfiles(dir: &Path) -> bool {
    dir.join("dot_zshrc").is_file() && dir.join("atelier/Cargo.toml").is_file()
}
