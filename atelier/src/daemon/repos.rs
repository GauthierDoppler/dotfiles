use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher, WatcherKind};

use crate::git::{self, RepoCounts};
use crate::session::{self, Field, Identity};

const SETTLE: Duration = Duration::from_millis(200);

pub type Fields = [String; Field::ALL.len()];

#[derive(PartialEq)]
struct Layout {
    worktree: PathBuf,
    git_dir: PathBuf,
    common: PathBuf,
}

impl Layout {
    fn find(path: &Path) -> Option<Layout> {
        let path = std::fs::canonicalize(path).ok()?;
        for dir in path.ancestors() {
            let dot_git = dir.join(".git");
            let git_dir = if dot_git.is_dir() {
                dot_git
            } else if dot_git.is_file() {
                let pointer = std::fs::read_to_string(&dot_git).ok()?;
                dir.join(pointer.strip_prefix("gitdir:")?.trim())
            } else {
                continue;
            };
            let git_dir = std::fs::canonicalize(git_dir).ok()?;
            let common = match std::fs::read_to_string(git_dir.join("commondir")) {
                Ok(relative) => std::fs::canonicalize(git_dir.join(relative.trim())).ok()?,
                Err(_) => git_dir.clone(),
            };
            return Some(Layout {
                worktree: dir.to_path_buf(),
                git_dir,
                common,
            });
        }
        None
    }

    fn change(&self, path: &Path) -> Option<Change> {
        let parent = path.parent()?;
        let name = path.file_name()?;
        if parent == self.git_dir && name == "index" {
            return Some(Change::Index);
        }
        let head = (parent == self.git_dir || parent == self.common)
            && (name == "HEAD" || name == "packed-refs");
        (head || path.starts_with(self.common.join("refs"))).then_some(Change::Refs)
    }
}

#[derive(PartialEq)]
enum Change {
    Index,
    Refs,
    Worktree,
}

struct Repo {
    layout: Layout,
    tracked: HashSet<PathBuf>,
    counts: Option<RepoCounts>,
    due: Option<Instant>,
    retrack: bool,
}

impl Repo {
    fn new(layout: Layout) -> Repo {
        let mut repo = Repo {
            layout,
            tracked: HashSet::new(),
            counts: None,
            due: None,
            retrack: false,
        };
        repo.track();
        repo
    }

    fn track(&mut self) {
        self.tracked = git::tracked_files(&self.layout.worktree)
            .unwrap_or_default()
            .into_iter()
            .collect();
    }

    fn change(&self, path: &Path) -> Option<Change> {
        if path.extension().is_some_and(|extension| extension == "lock") {
            return None;
        }
        self.layout
            .change(path)
            .or_else(|| self.tracked.contains(path).then_some(Change::Worktree))
    }

    fn watches(&self, recursive_worktree: bool) -> Vec<(PathBuf, bool)> {
        let layout = &self.layout;
        let mut watches = vec![
            (layout.git_dir.clone(), false),
            (layout.common.clone(), false),
            (layout.common.join("refs"), true),
        ];
        if recursive_worktree {
            watches.push((layout.worktree.clone(), true));
        } else {
            let dirs: HashSet<&Path> = self.tracked.iter().filter_map(|f| f.parent()).collect();
            watches.extend(dirs.into_iter().map(|dir| (dir.to_path_buf(), false)));
            watches.push((layout.worktree.clone(), false));
        }
        watches
    }
}

pub struct Repos {
    watcher: Option<RecommendedWatcher>,
    repos: HashMap<PathBuf, Repo>,
    located: HashMap<String, Option<PathBuf>>,
    watching: HashMap<PathBuf, bool>,
    unwatchable: HashSet<PathBuf>,
    identities: HashMap<(Fields, Option<PathBuf>), Option<Identity>>,
}

impl Repos {
    pub fn new(on_change: impl Fn(notify::Result<Event>) + Send + 'static) -> Repos {
        let watcher = notify::recommended_watcher(on_change)
        .inspect_err(|error| eprintln!("atelier: no repo watcher: {error}"))
        .ok();
        Repos {
            watcher,
            repos: HashMap::new(),
            located: HashMap::new(),
            watching: HashMap::new(),
            unwatchable: HashSet::new(),
            identities: HashMap::new(),
        }
    }

    pub fn follow(&mut self, sessions: &[Fields]) {
        self.located.clear();
        for fields in sessions {
            let path = &fields[Field::Path as usize];
            if self.located.contains_key(path) {
                continue;
            }
            let layout = Layout::find(Path::new(path));
            let key = layout.as_ref().map(|layout| layout.git_dir.clone());
            if let Some(layout) = layout {
                if self.repos.get(&layout.git_dir).map(|repo| &repo.layout) != Some(&layout) {
                    self.repos.insert(layout.git_dir.clone(), Repo::new(layout));
                }
            }
            self.located.insert(path.clone(), key);
        }
        let used: HashSet<&PathBuf> = self.located.values().flatten().collect();
        self.repos.retain(|git_dir, _| used.contains(git_dir));
        self.identities.retain(|(fields, _), _| sessions.contains(fields));
        self.rewatch();
    }

    pub fn identity(&mut self, fields: &Fields) -> Option<Identity> {
        let key = (fields.clone(), self.repo_of(&fields[Field::Path as usize]));
        self.identities
            .entry(key)
            .or_insert_with(|| {
                session::identify(|field| Ok(fields[field as usize].clone()))
                    .ok()
                    .flatten()
            })
            .clone()
    }

    pub fn counts(&mut self, path: &str) -> RepoCounts {
        let Some(repo) = self
            .repo_of(path)
            .and_then(|git_dir| self.repos.get_mut(&git_dir))
        else {
            return RepoCounts::default();
        };
        *repo
            .counts
            .get_or_insert_with(|| git::repo_counts(&repo.layout.worktree))
    }

    pub fn note(&mut self, event: notify::Result<Event>) {
        let event = match event {
            Ok(event) => event,
            Err(error) => {
                eprintln!("atelier: repo watcher: {error}");
                return;
            }
        };
        let due = Instant::now() + SETTLE;
        if event.need_rescan() {
            for repo in self.repos.values_mut() {
                repo.due = Some(due);
                repo.retrack = true;
            }
        }
        match event.kind {
            EventKind::Access(_) => return,
            EventKind::Remove(_) => {
                for path in &event.paths {
                    self.watching.remove(path);
                }
            }
            _ => {}
        }
        for path in &event.paths {
            for repo in self.repos.values_mut() {
                if let Some(change) = repo.change(path) {
                    repo.due = Some(due);
                    repo.retrack |= change == Change::Index;
                }
            }
        }
    }

    pub fn due(&self) -> Option<Instant> {
        self.repos.values().filter_map(|repo| repo.due).min()
    }

    pub fn settle(&mut self) -> bool {
        let now = Instant::now();
        let mut settled = false;
        for repo in self.repos.values_mut() {
            if repo.due.is_some_and(|due| due <= now) {
                repo.due = None;
                repo.counts = None;
                settled = true;
                if std::mem::take(&mut repo.retrack) {
                    repo.track();
                }
            }
        }
        if settled {
            self.rewatch();
        }
        settled
    }

    fn repo_of(&self, path: &str) -> Option<PathBuf> {
        self.located.get(path).cloned().flatten()
    }

    fn rewatch(&mut self) {
        let Some(watcher) = self.watcher.as_mut() else {
            return;
        };
        let recursive_worktree = RecommendedWatcher::kind() == WatcherKind::Fsevent;
        let mut wanted: HashMap<PathBuf, bool> = HashMap::new();
        for repo in self.repos.values() {
            for (path, recursive) in repo.watches(recursive_worktree) {
                *wanted.entry(path).or_default() |= recursive;
            }
        }
        self.watching.retain(|path, recursive| {
            let keep = wanted.get(path) == Some(recursive);
            if !keep {
                let _ = watcher.unwatch(path);
            }
            keep
        });
        self.unwatchable.retain(|path| wanted.contains_key(path));
        for (path, recursive) in wanted {
            if self.watching.contains_key(&path) {
                continue;
            }
            let mode = if recursive {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };
            match watcher.watch(&path, mode) {
                Ok(()) => {
                    self.unwatchable.remove(&path);
                    self.watching.insert(path, recursive);
                }
                Err(error) => {
                    if !self.unwatchable.contains(&path) {
                        eprintln!("atelier: cannot watch {}: {error}", path.display());
                        self.unwatchable.insert(path);
                    }
                }
            }
        }
    }
}
