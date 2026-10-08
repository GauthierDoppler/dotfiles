use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;

use clap::{Args, ValueEnum};

use crate::tmux::Tmux;
use crate::Result;

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Profile {
    Desktop,
    Remote,
}

#[derive(Args)]
pub struct Command {
    /// Which machine this is; defaults to desktop on macOS, remote elsewhere
    #[arg(long, value_enum)]
    profile: Option<Profile>,
    /// Print what would change and touch nothing
    #[arg(long)]
    dry_run: bool,
    /// The dotfiles checkout; defaults to the current git root, then ~/dotfiles
    #[arg(long, value_name = "PATH")]
    repo: Option<PathBuf>,
}

crate::flags_only!(Command);

enum Source {
    Path(&'static str),
    EachMarkdownIn(&'static str),
}

struct Link {
    source: Source,
    dest: &'static str,
    desktop_only: bool,
}

const fn link(source: &'static str, dest: &'static str) -> Link {
    Link {
        source: Source::Path(source),
        dest,
        desktop_only: false,
    }
}

const fn desktop(source: &'static str, dest: &'static str) -> Link {
    Link {
        source: Source::Path(source),
        dest,
        desktop_only: true,
    }
}

const fn each_markdown_in(dir: &'static str, dest: &'static str) -> Link {
    Link {
        source: Source::EachMarkdownIn(dir),
        dest,
        desktop_only: false,
    }
}

const LINKS: &[Link] = &[
    link("delta", ".config/delta"),
    desktop("ghostty", ".config/ghostty"),
    link("lazydocker", ".config/lazydocker"),
    link("lazygit", ".config/lazygit"),
    link("nvim", ".config/nvim"),
    link("tmux/oneshot", ".config/tmux/oneshot"),
    desktop("zed", ".config/zed"),
    link("git/ignore", ".config/git/ignore"),
    link("dot_tmux.conf", ".tmux.conf"),
    link("dot_claude/CLAUDE.md", ".claude/CLAUDE.md"),
    link(
        "dot_claude/statusline-custom.sh",
        ".claude/statusline-custom.sh",
    ),
    link("dot_claude/hooks", ".claude/hooks"),
    link("dot_claude/skills/tmux-tasks", ".claude/skills/tmux-tasks"),
    link("dot_claude/skills/grove", ".claude/skills/grove"),
    link("dot_claude/skills/notion", ".claude/skills/notion"),
    link(
        "dot_pi_agent/extensions/subagents",
        ".pi/agent/extensions/subagents",
    ),
    each_markdown_in("dot_pi_agent/agents", ".pi/agent/agents"),
    link("scripts/ssh-setup", ".local/bin/ssh-setup"),
    link("scripts/cc-tap-service", ".local/bin/cc-tap-service"),
];

pub(crate) enum Format {
    Shell,
    Git,
}

pub(crate) struct Stub {
    pub(crate) dest: &'static str,
    pub(crate) shared: &'static str,
    pub(crate) format: Format,
    home: &'static str,
    load: &'static str,
    legacy: Option<&'static str>,
}

pub(crate) const STUBS: &[Stub] = &[
    Stub {
        dest: ".zshrc",
        shared: "dot_zshrc",
        format: Format::Shell,
        home: "$HOME",
        load: "source \"{shared}\"",
        legacy: Some(".zshrc.local"),
    },
    Stub {
        dest: ".zprofile",
        shared: "dot_zprofile",
        format: Format::Shell,
        home: "$HOME",
        load: "source \"{shared}\"",
        legacy: None,
    },
    Stub {
        dest: ".gitconfig",
        shared: "dot_gitconfig",
        format: Format::Git,
        home: "~",
        load: "[include]\n\tpath = {shared}",
        legacy: Some(".gitconfig.local"),
    },
];

const STUB_HEADER: &str = "\
# Loads the shared dotfiles config. Everything BELOW this block is
# machine-local and stays out of the dotfiles repo — put local overrides
# here, and let installers append here too.
";

enum Action {
    Backup {
        path: PathBuf,
        to: PathBuf,
    },
    Unlink {
        path: PathBuf,
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

impl Command {
    pub fn run(self, _tmux: &Tmux) -> Result<()> {
        let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME is not set")?);
        let repo = repo_root(self.repo, &home)?;
        let profile = self.profile.unwrap_or(if cfg!(target_os = "macos") {
            Profile::Desktop
        } else {
            Profile::Remote
        });

        let mut actions = Vec::new();
        for (source, dest) in links(&repo, &home, profile)? {
            plan_link(&mut actions, source, dest)?;
        }
        for stub in STUBS {
            plan_stub(&mut actions, stub, &repo, &home)?;
        }

        for action in &actions {
            println!("{}", action.describe());
            if !self.dry_run {
                action.apply()?;
            }
        }
        match (actions.is_empty(), self.dry_run) {
            (true, _) => println!("setup: up to date"),
            (false, true) => println!("setup: dry run, nothing written"),
            (false, false) => {}
        }
        Ok(())
    }
}

pub(crate) fn repo_root(flag: Option<PathBuf>, home: &Path) -> Result<PathBuf> {
    if let Some(repo) = flag {
        let repo = std::path::absolute(repo)?;
        return if is_dotfiles(&repo) {
            Ok(repo)
        } else {
            Err(format!("{} is not a dotfiles checkout", repo.display()).into())
        };
    }
    let candidates = git_toplevel().into_iter().chain([home.join("dotfiles")]);
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

fn git_toplevel() -> Option<PathBuf> {
    let output = process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8(output.stdout).ok()?;
    Some(PathBuf::from(path.trim_end_matches('\n')))
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

fn backup(path: &Path) -> Action {
    Action::Backup {
        path: path.to_path_buf(),
        to: free_path(path, ".bak"),
    }
}

fn free_path(path: &Path, suffix: &str) -> PathBuf {
    let mut candidate = suffixed(path, suffix);
    let mut n = 1;
    while fs::symlink_metadata(&candidate).is_ok() {
        candidate = suffixed(path, &format!("{suffix}.{n}"));
        n += 1;
    }
    candidate
}

fn suffixed(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

impl Action {
    fn describe(&self) -> String {
        match self {
            Action::Backup { path, to } => {
                format!("backup: {} -> {}", path.display(), to.display())
            }
            Action::Unlink { path } => format!("unlink: {}", path.display()),
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

    fn apply(&self) -> Result<()> {
        match self {
            Action::Backup { path, to } => fs::rename(path, to)?,
            Action::Unlink { path } => fs::remove_file(path)?,
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
