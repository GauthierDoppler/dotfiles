use std::fs;
use std::path::{Path, PathBuf};

use super::action::Action;
use super::table::{Source, LINKS};
use crate::Result;

pub(super) fn plan_prune(actions: &mut Vec<Action>, repo: &Path, home: &Path) -> Result<()> {
    let repo_spellings = [repo.to_path_buf(), repo.canonicalize()?];
    let inside_repo = |path: &Path| repo_spellings.iter().any(|repo| path.starts_with(repo));
    let mut dirs: Vec<PathBuf> = LINKS
        .iter()
        .filter_map(|link| match link.source {
            Source::Path(_) => Path::new(link.dest).parent().map(|dir| home.join(dir)),
            Source::EachMarkdownIn(_) => Some(home.join(link.dest)),
        })
        .collect();
    dirs.sort();
    dirs.dedup();
    for dir in dirs {
        if dir.canonicalize().is_ok_and(|real| inside_repo(&real)) {
            continue;
        }
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let mut dangling = Vec::new();
        for entry in entries {
            let path = entry?.path();
            let Ok(target) = fs::read_link(&path) else {
                continue;
            };
            let unlinked = actions
                .iter()
                .any(|action| matches!(action, Action::Unlink { path: planned } if *planned == path));
            if fs::metadata(&path).is_err()
                && inside_repo(&lexically_normal(&dir.join(&target)))
                && !unlinked
            {
                dangling.push((path, target));
            }
        }
        dangling.sort();
        actions.extend(
            dangling
                .into_iter()
                .map(|(path, target)| Action::Prune { path, target }),
        );
    }
    Ok(())
}

fn lexically_normal(path: &Path) -> PathBuf {
    let mut normal = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                normal.pop();
            }
            std::path::Component::CurDir => {}
            other => normal.push(other),
        }
    }
    normal
}
