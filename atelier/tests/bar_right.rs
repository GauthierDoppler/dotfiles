mod common;

use std::path::Path;
use std::process::Command;

use common::{git, git_repo, stdout_of};

fn bar_right(args: &[&str], env: &[(&str, &str)]) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_atelier"));
    command.args(["bar", "right"]).args(args).env_remove("TMUX");
    for (key, value) in env {
        command.env(key, value);
    }
    stdout_of(args, command.output().expect("atelier runs"))
}

fn visible(rendered: &str) -> String {
    let mut text = String::new();
    let mut rest = rendered;
    while let Some(start) = rest.find("#[") {
        text.push_str(&rest[..start]);
        rest = &rest[start..];
        rest = &rest[rest.find(']').expect("style is closed") + 1..];
    }
    text + rest
}

fn repo_counts(dir: &Path) -> String {
    let rendered = bar_right(&[dir.to_str().unwrap(), "90", "root"], &[]);
    let text = visible(&rendered);
    let mut words: Vec<&str> = text.split_whitespace().collect();
    words.pop();
    words.join(" ")
}

fn write(path: &Path, contents: &str) {
    std::fs::write(path, contents).unwrap();
}

fn repo_with_upstream(dir: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let upstream = dir.join("upstream");
    git_repo(&upstream);
    write(&upstream.join("f"), "1\n2\n3\n4\n");
    git(&upstream, &["add", "f"]);
    git(&upstream, &["commit", "-q", "-m", "four lines"]);
    let clone = dir.join("clone");
    git(
        dir,
        &[
            "clone",
            "-q",
            upstream.to_str().unwrap(),
            clone.to_str().unwrap(),
        ],
    );
    (upstream, clone)
}

#[test]
fn counts_lines_changed_against_head_staged_or_not_and_ignores_untracked_files() {
    let dir = tempfile::tempdir().unwrap();
    let (_, clone) = repo_with_upstream(dir.path());
    write(&clone.join("f"), "1\nTWO\n3\n4\nfive\nsix\n");
    write(&clone.join("staged"), "a\n");
    git(&clone, &["add", "staged"]);
    write(&clone.join("untracked"), "x\ny\nz\n");

    assert_eq!(repo_counts(&clone), "+4 −1");
}

#[test]
fn counts_commits_ahead_and_behind_the_upstream() {
    let dir = tempfile::tempdir().unwrap();
    let (upstream, clone) = repo_with_upstream(dir.path());
    for message in ["one", "two"] {
        git(&clone, &["commit", "-q", "--allow-empty", "-m", message]);
    }
    git(
        &upstream,
        &["commit", "-q", "--allow-empty", "-m", "theirs"],
    );
    git(&clone, &["fetch", "-q"]);

    assert_eq!(repo_counts(&clone), "↑2 ↓1");
}

#[test]
fn a_branch_without_upstream_shows_only_its_line_counts() {
    let dir = tempfile::tempdir().unwrap();
    let (_, clone) = repo_with_upstream(dir.path());
    git(&clone, &["checkout", "-q", "-b", "local"]);
    git(&clone, &["commit", "-q", "--allow-empty", "-m", "mine"]);
    write(&clone.join("f"), "1\n2\n3\n");

    assert_eq!(repo_counts(&clone), "−1");
}

#[test]
fn a_detached_head_shows_only_its_line_counts() {
    let dir = tempfile::tempdir().unwrap();
    let (_, clone) = repo_with_upstream(dir.path());
    git(&clone, &["commit", "-q", "--allow-empty", "-m", "mine"]);
    git(&clone, &["checkout", "-q", "--detach"]);
    write(&clone.join("f"), "1\n2\n3\n4\n5\n");

    assert_eq!(repo_counts(&clone), "+1");
}

#[test]
fn an_empty_repo_a_plain_directory_and_a_missing_one_show_no_counts() {
    let dir = tempfile::tempdir().unwrap();
    let empty = dir.path().join("empty");
    std::fs::create_dir(&empty).unwrap();
    git(&empty, &["init", "-q"]);
    write(&empty.join("f"), "a\n");
    git(&empty, &["add", "f"]);
    let plain = dir.path().join("plain");
    std::fs::create_dir(&plain).unwrap();

    assert_eq!(repo_counts(&empty), "");
    assert_eq!(repo_counts(&plain), "");
    assert_eq!(repo_counts(&dir.path().join("gone")), "");
    assert_eq!(repo_counts(Path::new("")), "");
}

#[test]
fn reading_the_counts_never_rewrites_the_index() {
    let dir = tempfile::tempdir().unwrap();
    let (_, clone) = repo_with_upstream(dir.path());
    let stale = std::fs::File::options()
        .write(true)
        .open(clone.join("f"))
        .unwrap();
    stale
        .set_modified(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1 << 30))
        .unwrap();
    let index = clone.join(".git/index");
    let before = std::fs::metadata(&index).unwrap().modified().unwrap();

    repo_counts(&clone);

    assert_eq!(
        std::fs::metadata(&index).unwrap().modified().unwrap(),
        before
    );
}

#[test]
fn the_block_is_as_wide_as_width_reports_whatever_the_locale() {
    let dir = tempfile::tempdir().unwrap();
    let (_, clone) = repo_with_upstream(dir.path());
    write(&clone.join("f"), "1\n2\n");
    git(&clone, &["commit", "-q", "--allow-empty", "-m", "ahead"]);
    let path = clone.to_str().unwrap();

    for client_width in ["80", "90", "100", "120", "200"] {
        let width: usize = bar_right(&["--width", client_width], &[]).parse().unwrap();
        for locale in [
            [("LANG", "C"), ("LC_ALL", "C")],
            [("LANG", "en_US.UTF-8"), ("LC_ALL", "")],
        ] {
            let rendered = bar_right(&[path, client_width, "prefix"], &locale);
            assert_eq!(
                visible(&rendered).chars().count(),
                width,
                "{client_width} columns, {locale:?}: {rendered}"
            );
        }
    }
}
