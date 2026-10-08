use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::tmux::Tmux;
use crate::Result;

pub const NO_DIRECTORY: &str = "(no .tmux/ directory in this project)";
pub const NO_TASK: &str = "(no executable task in .tmux/)";
const ALL: &str = "all";

pub struct Catalog {
    pub session: String,
    pub root: PathBuf,
}

impl Catalog {
    pub fn of_session(tmux: &Tmux, target: Option<&str>) -> Result<Self> {
        let session = tmux.resolve(target, "#{session_id}")?;
        let path = tmux.display(&session, "#{session_path}")?;
        let start = if path.is_empty() {
            std::env::current_dir()?
        } else {
            PathBuf::from(path)
        };
        let root = start
            .ancestors()
            .find(|dir| dir.join(".tmux").is_dir())
            .unwrap_or(&start)
            .to_path_buf();
        Ok(Catalog { session, root })
    }

    pub fn dir(&self) -> PathBuf {
        self.root.join(".tmux")
    }

    pub fn script(&self, name: &str) -> PathBuf {
        self.dir().join(name)
    }

    pub fn names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for (entry, relative) in entries(&self.dir(), "") {
            if entry.symlink_metadata().is_ok_and(|meta| meta.is_dir()) {
                names.extend(
                    entries(&entry, &relative)
                        .into_iter()
                        .filter(|(nested, _)| is_executable_file(nested))
                        .map(|(_, nested_relative)| nested_relative),
                );
            } else if is_executable_file(&entry) {
                names.push(relative);
            }
        }
        names.retain(|name| !name.starts_with('.'));
        names.sort();
        names
    }

    pub fn groups(&self) -> Vec<String> {
        let mut groups: Vec<String> = self
            .names()
            .iter()
            .filter_map(|name| name.split_once('/'))
            .map(|(group, _)| group.to_string())
            .collect();
        groups.dedup();
        groups
    }

    pub fn group(&self) -> String {
        fs::read_to_string(self.state("group"))
            .ok()
            .filter(|group| !group.is_empty())
            .unwrap_or_else(|| ALL.to_string())
    }

    pub fn set_group(&self, group: &str) -> Result<()> {
        Ok(fs::write(self.state("group"), group)?)
    }

    pub fn advance_group(&self) -> Result<()> {
        let current = self.group();
        let mut ring = vec![ALL.to_string()];
        ring.extend(self.groups());
        let next = ring
            .iter()
            .position(|group| *group == current)
            .and_then(|index| ring.get(index + 1))
            .map_or(ALL, String::as_str);
        self.set_group(next)
    }

    pub fn touch(&self, name: &str) -> Result<()> {
        let mut recent = vec![name.to_string()];
        recent.extend(self.recent().into_iter().filter(|seen| seen != name));
        let path = self.state("recent");
        let partial = path.with_extension(format!("tmp.{}", std::process::id()));
        fs::write(&partial, recent.join("\n") + "\n")?;
        Ok(fs::rename(partial, path)?)
    }

    pub fn rows(&self) -> Vec<String> {
        let mut names = self.names();
        if names.is_empty() {
            let notice = if self.dir().is_dir() {
                NO_TASK
            } else {
                NO_DIRECTORY
            };
            return vec![notice.to_string()];
        }
        let group = self.group();
        if group != ALL {
            let prefix = format!("{group}/");
            names.retain(|name| name.starts_with(&prefix));
        }
        let mut ordered: Vec<String> = self
            .recent()
            .into_iter()
            .filter(|name| names.contains(name))
            .collect();
        names.retain(|name| !ordered.contains(name));
        ordered.append(&mut names);
        let width = ordered
            .iter()
            .map(|name| name.chars().count())
            .max()
            .unwrap_or(0);
        ordered
            .iter()
            .map(|name| {
                let row = format!("{name:<width$}  {}", header(&self.script(name), "task"));
                row.trim_end().to_string()
            })
            .collect()
    }

    fn recent(&self) -> Vec<String> {
        fs::read_to_string(self.state("recent"))
            .map(|recent| recent.lines().map(str::to_string).collect())
            .unwrap_or_default()
    }

    fn state(&self, kind: &str) -> PathBuf {
        let hash = self
            .root
            .as_os_str()
            .as_encoded_bytes()
            .iter()
            .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
                (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
            });
        std::env::temp_dir().join(format!("atelier-tasks.{hash:016x}.{kind}"))
    }
}

pub fn header(script: &Path, key: &str) -> String {
    let Ok(file) = fs::File::open(script) else {
        return String::new();
    };
    BufReader::new(file)
        .split(b'\n')
        .take(20)
        .map_while(std::result::Result::ok)
        .find_map(|line| header_value(&String::from_utf8_lossy(&line), key))
        .unwrap_or_default()
}

fn header_value(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix('#')?.trim_start_matches(' ');
    let value = rest.strip_prefix(key)?.strip_prefix(':')?;
    Some(value.trim_start_matches(' ').to_string())
}

fn entries(dir: &Path, prefix: &str) -> Vec<(PathBuf, String)> {
    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };
    read.flatten()
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let relative = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            (entry.path(), relative)
        })
        .collect()
}

fn is_executable_file(path: &Path) -> bool {
    path.symlink_metadata()
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o100 != 0)
}
