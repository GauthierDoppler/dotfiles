use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::{symlink, MetadataExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const DESKTOP_LINKS: &[(&str, &str)] = &[
    (".config/delta", "delta"),
    (".config/ghostty", "ghostty"),
    (".config/lazydocker", "lazydocker"),
    (".config/lazygit", "lazygit"),
    (".config/nvim", "nvim"),
    (".config/tmux/oneshot", "tmux/oneshot"),
    (".config/zed", "zed"),
    (".config/git/ignore", "git/ignore"),
    (".tmux.conf", "dot_tmux.conf"),
    (".claude/CLAUDE.md", "dot_claude/CLAUDE.md"),
    (
        ".claude/statusline-custom.sh",
        "dot_claude/statusline-custom.sh",
    ),
    (".claude/hooks", "dot_claude/hooks"),
    (".claude/skills/tmux-tasks", "dot_claude/skills/tmux-tasks"),
    (".claude/skills/grove", "dot_claude/skills/grove"),
    (".claude/skills/notion", "dot_claude/skills/notion"),
    (
        ".pi/agent/extensions/subagents",
        "dot_pi_agent/extensions/subagents",
    ),
    (
        ".pi/agent/agents/implementer.md",
        "dot_pi_agent/agents/implementer.md",
    ),
    (
        ".pi/agent/agents/planner.md",
        "dot_pi_agent/agents/planner.md",
    ),
    (
        ".pi/agent/agents/reviewer.md",
        "dot_pi_agent/agents/reviewer.md",
    ),
    (".pi/agent/agents/scout.md", "dot_pi_agent/agents/scout.md"),
    (".local/bin/local-diff", "scripts/local-diff"),
    (".local/bin/ssh-setup", "scripts/ssh-setup"),
    (".local/bin/tmux-sessions", "scripts/tmux-sessions"),
    (".local/bin/tmux-tasks", "scripts/tmux-tasks"),
    (".local/bin/tmux-task-run", "scripts/tmux-task-run"),
    (".local/bin/md-preview", "scripts/md-preview/md-preview"),
    (".local/bin/cc-tap-service", "scripts/cc-tap-service"),
];

const GUI_APP_CONFIGS: &[&str] = &[".config/ghostty", ".config/zed"];

const STUB_HEADER: &str = "# Loads the shared dotfiles config. Everything BELOW this block is\n\
# machine-local and stays out of the dotfiles repo — put local overrides\n\
# here, and let installers append here too.\n";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crate sits inside the dotfiles repo")
        .to_path_buf()
}

struct Home {
    dir: tempfile::TempDir,
}

impl Home {
    fn new() -> Self {
        Home {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn join(&self, relative: &str) -> PathBuf {
        self.path().join(relative)
    }

    fn setup_in(&self, cwd: &Path, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_atelier"))
            .arg("setup")
            .args(args)
            .current_dir(cwd)
            .env("HOME", self.path())
            .env_remove("TMUX")
            .output()
            .expect("atelier runs")
    }

    fn setup(&self, args: &[&str]) -> String {
        self.setup_from(&repo(), args)
    }

    fn setup_from(&self, repo: &Path, args: &[&str]) -> String {
        let mut full = vec!["--repo", repo.to_str().unwrap()];
        full.extend_from_slice(args);
        let output = self.setup_in(self.path(), &full);
        assert!(
            output.status.success(),
            "atelier setup {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.join(relative)).unwrap()
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn link_target(&self, relative: &str) -> Option<PathBuf> {
        fs::read_link(self.join(relative)).ok()
    }

    fn snapshot(&self) -> BTreeMap<PathBuf, String> {
        let mut entries = BTreeMap::new();
        walk(self.path(), self.path(), &mut entries);
        entries
    }
}

fn walk(root: &Path, dir: &Path, entries: &mut BTreeMap<PathBuf, String>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let meta = fs::symlink_metadata(&path).unwrap();
        let relative = path.strip_prefix(root).unwrap().to_path_buf();
        let state = if meta.file_type().is_symlink() {
            format!(
                "link {} ino {}",
                fs::read_link(&path).unwrap().display(),
                meta.ino()
            )
        } else if meta.is_dir() {
            walk(root, &path, entries);
            format!("dir ino {}", meta.ino())
        } else {
            format!(
                "file {:?} ino {} mtime {}.{}",
                fs::read_to_string(&path).unwrap(),
                meta.ino(),
                meta.mtime(),
                meta.mtime_nsec()
            )
        };
        entries.insert(relative, state);
    }
}

#[test]
fn the_desktop_profile_links_every_shared_config_into_home() {
    let home = Home::new();
    home.setup(&["--profile", "desktop"]);

    for (dest, src) in DESKTOP_LINKS {
        assert_eq!(
            home.link_target(dest),
            Some(repo().join(src)),
            "{dest} links to {src}"
        );
    }
}

#[test]
fn the_remote_profile_skips_gui_app_configs() {
    let home = Home::new();
    home.setup(&["--profile", "remote"]);

    for dest in GUI_APP_CONFIGS {
        assert!(!home.join(dest).exists(), "{dest} is not linked on remote");
    }
    assert_eq!(home.link_target(".config/nvim"), Some(repo().join("nvim")));
}

#[test]
fn shell_and_git_configs_are_stubs_that_load_the_repo_through_home() {
    let home = Home::new();
    symlink(repo(), home.join("dotfiles")).unwrap();
    home.setup_from(&home.join("dotfiles"), &[]);

    assert_eq!(
        home.read(".zshrc"),
        format!("{STUB_HEADER}source \"$HOME/dotfiles/dot_zshrc\"\n\n")
    );
    assert_eq!(
        home.read(".zprofile"),
        format!("{STUB_HEADER}source \"$HOME/dotfiles/dot_zprofile\"\n\n")
    );
    assert_eq!(
        home.read(".gitconfig"),
        format!("{STUB_HEADER}[include]\n\tpath = ~/dotfiles/dot_gitconfig\n\n")
    );
    assert_eq!(home.link_target(".zshrc"), None);
}

#[test]
fn existing_files_and_folders_are_backed_up_before_linking() {
    let home = Home::new();
    home.write(".tmux.conf", "set -g mouse on\n");
    home.write(".config/nvim/init.lua", "-- mine\n");
    home.setup(&[]);

    assert_eq!(home.read(".tmux.conf.bak"), "set -g mouse on\n");
    assert_eq!(home.read(".config/nvim.bak/init.lua"), "-- mine\n");
    assert_eq!(
        home.link_target(".tmux.conf"),
        Some(repo().join("dot_tmux.conf"))
    );
}

#[test]
fn an_earlier_backup_is_never_overwritten() {
    let home = Home::new();
    home.write(".tmux.conf.bak", "first\n");
    home.write(".tmux.conf", "second\n");
    home.setup(&[]);

    assert_eq!(home.read(".tmux.conf.bak"), "first\n");
    assert_eq!(home.read(".tmux.conf.bak.1"), "second\n");
}

#[test]
fn a_symlink_pointing_elsewhere_is_replaced_without_backup() {
    let home = Home::new();
    fs::create_dir_all(home.join(".config")).unwrap();
    symlink("/somewhere/else", home.join(".config/lazygit")).unwrap();
    home.setup(&[]);

    assert_eq!(
        home.link_target(".config/lazygit"),
        Some(repo().join("lazygit"))
    );
    assert!(fs::symlink_metadata(home.join(".config/lazygit.bak")).is_err());
}

#[test]
fn local_content_below_an_existing_stub_is_left_alone() {
    let home = Home::new();
    let zshrc = format!(
        "{STUB_HEADER}source \"{}/dot_zshrc\"\n\nexport PATH=\"$HOME/.cargo/bin:$PATH\"\n",
        repo().display()
    );
    home.write(".zshrc", &zshrc);
    home.setup(&[]);

    assert_eq!(home.read(".zshrc"), zshrc);
    assert!(!home.join(".zshrc.bak").exists());
}

#[test]
fn a_hand_written_config_is_backed_up_and_replaced_by_a_stub() {
    let home = Home::new();
    home.write(".gitconfig", "[user]\n\tname = me\n");
    home.setup(&[]);

    assert_eq!(home.read(".gitconfig.bak"), "[user]\n\tname = me\n");
    assert_eq!(
        home.read(".gitconfig"),
        format!(
            "{STUB_HEADER}[include]\n\tpath = {}/dot_gitconfig\n\n",
            repo().display()
        )
    );
}

#[test]
fn the_old_symlinked_zshrc_becomes_a_stub_without_backup() {
    let home = Home::new();
    symlink(repo().join("dot_zshrc"), home.join(".zshrc")).unwrap();
    home.setup(&[]);

    assert_eq!(home.link_target(".zshrc"), None);
    assert!(home.read(".zshrc").starts_with(STUB_HEADER));
    assert!(fs::symlink_metadata(home.join(".zshrc.bak")).is_err());
}

#[test]
fn legacy_local_files_are_folded_into_their_stub_and_kept() {
    let home = Home::new();
    home.write(".zshrc.local", "alias k=kubectl\n");
    home.write(".gitconfig.local", "[user]\n\temail = me@example.com\n");
    home.setup(&[]);

    assert_eq!(
        home.read(".zshrc"),
        format!(
            "{STUB_HEADER}source \"{}/dot_zshrc\"\n\n\n# ─── migrated from .zshrc.local ───\nalias k=kubectl\n",
            repo().display()
        )
    );
    assert!(home.read(".gitconfig").ends_with(
        "\n# ─── migrated from .gitconfig.local ───\n[user]\n\temail = me@example.com\n"
    ));
    assert!(!home.join(".zshrc.local").exists());
    assert_eq!(home.read(".zshrc.local.migrated"), "alias k=kubectl\n");
    assert_eq!(
        home.read(".gitconfig.local.migrated"),
        "[user]\n\temail = me@example.com\n"
    );
}

#[test]
fn running_setup_twice_changes_nothing_the_second_time() {
    let home = Home::new();
    home.write(".tmux.conf", "old\n");
    home.write(".zshrc.local", "alias k=kubectl\n");
    home.setup(&["--profile", "desktop"]);
    let after_first = home.snapshot();

    let output = home.setup(&["--profile", "desktop"]);

    assert_eq!(home.snapshot(), after_first);
    assert_eq!(output, "setup: up to date\n");
}

#[test]
fn a_dry_run_prints_the_plan_and_touches_nothing() {
    let home = Home::new();
    home.write(".tmux.conf", "old\n");
    home.write(".zshrc.local", "alias k=kubectl\n");
    let before = home.snapshot();

    let output = home.setup(&["--dry-run"]);

    assert_eq!(home.snapshot(), before);
    let h = home.path().display();
    let r = repo().display().to_string();
    for line in [
        format!("backup: {h}/.tmux.conf -> {h}/.tmux.conf.bak"),
        format!("linked: {h}/.tmux.conf -> {r}/dot_tmux.conf"),
        format!("stubbed: {h}/.zshrc"),
        format!("migrated: {h}/.zshrc.local -> {h}/.zshrc (kept as {h}/.zshrc.local.migrated)"),
    ] {
        assert!(output.lines().any(|l| l == line), "{line:?} in {output}");
    }
}

#[test]
fn without_repo_the_current_git_root_is_used() {
    let home = Home::new();
    let output = home.setup_in(&repo().join("atelier/src"), &[]);

    assert!(output.status.success());
    assert_eq!(
        home.link_target(".config/nvim")
            .map(|t| t.ends_with("nvim")),
        Some(true)
    );
}

#[test]
fn a_repo_that_is_not_the_dotfiles_is_refused() {
    let home = Home::new();
    let elsewhere = tempfile::tempdir().unwrap();
    let output = home.setup_in(home.path(), &["--repo", elsewhere.path().to_str().unwrap()]);

    assert!(!output.status.success());
    assert_eq!(home.snapshot(), BTreeMap::new());
}

#[test]
fn a_repo_outside_home_is_loaded_by_its_absolute_path() {
    let home = Home::new();
    home.setup(&[]);

    assert_eq!(
        home.read(".zshrc"),
        format!("{STUB_HEADER}source \"{}/dot_zshrc\"\n\n", repo().display())
    );
}
