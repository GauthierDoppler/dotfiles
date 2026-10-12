use std::fs;
use std::path::{Path, PathBuf};

use super::action::Action;
use super::backup::{backup, free_path};
use super::prune::plan_prune;
use super::table::{Source, Stub, LINKS, STUBS, STUB_HEADER};
use super::Profile;
use crate::Result;

pub(super) fn plan(repo: &Path, home: &Path, profile: Profile) -> Result<Vec<Action>> {
    let mut actions = Vec::new();
    for (source, dest) in links(repo, home, profile)? {
        plan_link(&mut actions, source, dest)?;
    }
    for stub in STUBS {
        plan_stub(&mut actions, stub, repo, home)?;
    }
    plan_prune(&mut actions, repo, home)?;
    Ok(actions)
}

fn links(repo: &Path, home: &Path, profile: Profile) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut pairs = Vec::new();
    for link in LINKS {
        if link.desktop_only && profile != Profile::Desktop {
            continue;
        }
        match link.source {
            Source::Path(source) => pairs.push((repo.join(source), home.join(link.dest))),
            Source::EachMarkdownIn(dir) => {
                let mut names = Vec::new();
                for entry in fs::read_dir(repo.join(dir))? {
                    let name = entry?.file_name();
                    if Path::new(&name).extension().is_some_and(|ext| ext == "md") {
                        names.push(name);
                    }
                }
                names.sort();
                for name in names {
                    pairs.push((repo.join(dir).join(&name), home.join(link.dest).join(&name)));
                }
            }
        }
    }
    Ok(pairs)
}

fn plan_link(actions: &mut Vec<Action>, source: PathBuf, dest: PathBuf) -> Result<()> {
    match fs::symlink_metadata(&dest) {
        Ok(meta) if meta.file_type().is_symlink() => {
            if fs::read_link(&dest)? == source {
                return Ok(());
            }
            actions.push(Action::Unlink { path: dest.clone() });
        }
        Ok(_) => actions.push(backup(&dest)),
        Err(_) => {}
    }
    actions.push(Action::Symlink { source, dest });
    Ok(())
}

fn plan_stub(actions: &mut Vec<Action>, stub: &Stub, repo: &Path, home: &Path) -> Result<()> {
    let repo = match repo.strip_prefix(home) {
        Ok(relative) => format!("{}/{}", stub.home, relative.display()),
        Err(_) => repo.display().to_string(),
    };
    let load = stub
        .load
        .replace("{shared}", &format!("{repo}/{}", stub.shared));
    let dest = home.join(stub.dest);

    let stubbed = match fs::symlink_metadata(&dest) {
        Ok(meta) if meta.file_type().is_symlink() => {
            actions.push(Action::Unlink { path: dest.clone() });
            false
        }
        Ok(meta) if meta.is_file() && is_stubbed(&dest, &load)? => true,
        Ok(_) => {
            actions.push(backup(&dest));
            false
        }
        Err(_) => false,
    };
    if !stubbed {
        actions.push(Action::WriteStub {
            path: dest.clone(),
            content: format!("{STUB_HEADER}{load}\n\n"),
        });
    }

    if let Some(legacy) = stub.legacy.map(|legacy| home.join(legacy)) {
        if legacy.is_file() {
            actions.push(Action::Migrate {
                kept: free_path(&legacy, ".migrated"),
                legacy,
                dest,
            });
        }
    }
    Ok(())
}

fn is_stubbed(path: &Path, load: &str) -> Result<bool> {
    let last = load.lines().last().unwrap_or(load);
    let content = fs::read(path)?;
    Ok(String::from_utf8_lossy(&content)
        .lines()
        .any(|line| line == last))
}
