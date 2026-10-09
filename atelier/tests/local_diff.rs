#![allow(clippy::disallowed_methods)]

mod common;

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use common::BoundedOutput;

const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[90m";
const YELLOW: &str = "\x1b[33m";
const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";

struct Machine {
    home: tempfile::TempDir,
}

impl Machine {
    fn new() -> Self {
        let machine = Machine {
            home: tempfile::tempdir().unwrap(),
        };
        machine.write("dotfiles/atelier/Cargo.toml", "");
        machine.write("dotfiles/dot_zshrc", "");
        machine
    }

    fn home(&self) -> &Path {
        self.home.path()
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.home().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_atelier"))
            .arg("local-diff")
            .args(args)
            .current_dir(self.home())
            .env("HOME", self.home())
            .env_remove("DOTFILES")
            .env_remove("TMUX")
            .bounded_output()
            .expect("atelier runs")
    }

    fn local_diff(&self) -> String {
        let output = self.run(&[]);
        assert!(
            output.status.success(),
            "atelier local-diff failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }
}

fn finding(title: &str, subtitle: &str, lines: &[&str]) -> String {
    let mut out = format!("\n{BOLD}{title}{RESET}\n{DIM}{subtitle}{RESET}\n");
    for line in lines {
        out.push_str(&format!("  {YELLOW}{line}{RESET}\n"));
    }
    out
}

fn nothing() -> String {
    format!("{GREEN}✓ local config adds nothing beyond the shared dotfiles{RESET}\n")
}

#[test]
fn a_machine_with_nothing_local_says_so() {
    let machine = Machine::new();

    assert_eq!(machine.local_diff(), nothing());
}

#[test]
fn shell_lines_below_the_load_line_that_the_shared_config_lacks_are_listed() {
    let machine = Machine::new();
    machine.write(
        "dotfiles/dot_zshrc",
        "# shared\nalias c=\"clear\"\nexport EDITOR=nvim\n",
    );
    machine.write(
        ".zshrc",
        "# Loads the shared dotfiles config.\n\
         source \"$HOME/dotfiles/dot_zshrc\"\n\
         \n\
         export EDITOR=nvim   # also shared\n\
         \talias c=\"clear\"\n\
         project grove ~/Developer/perso/grove-ai \"Grove\"\n\
         export PATH=\"$HOME/.cargo/bin:$PATH\"\n\
         project grove ~/Developer/perso/grove-ai \"Grove\"\n",
    );

    assert_eq!(
        machine.local_diff(),
        finding(
            "~/.zshrc",
            "not in ~/dotfiles/dot_zshrc",
            &[
                "export PATH=\"$HOME/.cargo/bin:$PATH\"",
                "project grove ~/Developer/perso/grove-ai \"Grove\"",
            ],
        )
    );
}

#[test]
fn a_load_line_in_any_spelling_is_not_drift() {
    let machine = Machine::new();
    machine.write("dotfiles/dot_zprofile", "export LANG=en_US.UTF-8\n");
    machine.write(
        ".zprofile",
        "source ~/dotfiles/dot_zprofile\nsource \"/elsewhere/dot_zprofile\"\n",
    );

    assert_eq!(machine.local_diff(), nothing());
}

#[test]
fn git_keys_the_shared_config_lacks_are_listed_whatever_their_formatting() {
    let machine = Machine::new();
    machine.write("dotfiles/dot_gitconfig", "[core]\n\tpager = delta\n");
    machine.write(
        ".gitconfig",
        "[include]\n\tpath = ~/dotfiles/dot_gitconfig\n\n\
         [user]\n  email = me@example.com\n\
         [Core]\n    Pager = delta\n",
    );

    assert_eq!(
        machine.local_diff(),
        finding(
            "~/.gitconfig",
            "not in ~/dotfiles/dot_gitconfig",
            &["user.email=me@example.com"],
        )
    );
}

#[test]
fn claude_settings_overrides_are_listed_by_key_path_and_value() {
    let machine = Machine::new();
    machine.write(
        "dotfiles/dot_claude/settings.json",
        r#"{"model": "opus", "permissions": {"allow": ["Bash(git:*)"]}}"#,
    );
    machine.write(
        "dotfiles/dot_claude/settings.local.json",
        r#"{"model": "sonnet", "verbose": false, "permissions": {"allow": ["Bash(git:*)", "Read"]}}"#,
    );

    assert_eq!(
        machine.local_diff(),
        finding(
            "~/dotfiles/dot_claude/settings.local.json",
            "not in ~/dotfiles/dot_claude/settings.json",
            &["model=sonnet", "permissions.allow.1=Read", "verbose=false"],
        )
    );
}

#[test]
fn legacy_local_files_are_listed_as_unsourced() {
    let machine = Machine::new();
    machine.write(".zshrc.local", "alias k=kubectl\n");
    machine.write(".npmrc.local", "");

    assert_eq!(
        machine.local_diff(),
        finding(
            "Legacy .local files",
            "nothing sources these — run install.sh to migrate them into the stubs",
            &[".npmrc.local", ".zshrc.local"],
        )
    );
}

#[test]
fn findings_come_in_a_fixed_order() {
    let machine = Machine::new();
    machine.write("dotfiles/dot_gitconfig", "");
    machine.write("dotfiles/dot_zprofile", "");
    machine.write(".gitconfig", "[user]\n\tname = me\n");
    machine.write(".zprofile", "export JAVA_HOME=/opt/jdk\n");
    machine.write(".zshrc", "alias k=kubectl\n");

    let titles: Vec<String> = machine
        .local_diff()
        .lines()
        .filter_map(|line| line.strip_prefix(BOLD))
        .map(|line| line.trim_end_matches(RESET).to_string())
        .collect();
    assert_eq!(titles, ["~/.zshrc", "~/.zprofile", "~/.gitconfig"]);
}

#[test]
fn the_dotfiles_checkout_can_be_given_explicitly() {
    let machine = Machine::new();
    let elsewhere = tempfile::tempdir().unwrap();
    let repo = elsewhere.path().join("dots");
    fs::create_dir_all(repo.join("atelier")).unwrap();
    fs::write(repo.join("atelier/Cargo.toml"), "").unwrap();
    fs::write(repo.join("dot_zshrc"), "alias k=kubectl\n").unwrap();
    machine.write(".zshrc", "alias k=kubectl\nalias g=git\n");

    let output = machine.run(&["--repo", repo.to_str().unwrap()]);

    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        finding(
            "~/.zshrc",
            &format!("not in {}/dot_zshrc", repo.display()),
            &["alias g=git"],
        )
    );
}
