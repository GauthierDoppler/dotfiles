mod common;

use common::TmuxServer;

fn visible(block: &str) -> String {
    let mut out = String::new();
    let mut rest = block;
    while let Some(start) = rest.find("#[") {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        rest = &rest[rest.find(']').expect("closed style") + 1..];
    }
    out.push_str(rest);
    out
}

fn pills(block: &str) -> String {
    visible(block).trim_end_matches(' ').to_string()
}

fn bare(project: &str) -> String {
    format!("\u{e0b6} {project} \u{e0b4}")
}

fn checkout(project: &str, kind: &str) -> String {
    format!("\u{e0b6} {project} \u{e0b4} \u{e0b6} {kind} \u{e0b4}")
}

fn left(tmux: &TmuxServer, session: &str) -> String {
    tmux.atelier_stdout(&["bar", "left", "-t", session, "200"])
}

#[test]
fn a_session_outside_any_repo_is_a_single_pill_named_after_itself() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let session = tmux.new_session("scratch", dir.path());

    assert_eq!(pills(&left(&tmux, &session)), bare("scratch"));
}

#[test]
fn a_session_in_a_git_repo_is_named_after_the_repo_at_its_root() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("bp-api");
    common::git_repo(&repo);
    let deep = repo.join("src/handlers");
    std::fs::create_dir_all(&deep).unwrap();
    let tmux = TmuxServer::start();
    let session = tmux.new_session("hand-named", &deep);

    assert_eq!(pills(&left(&tmux, &session)), checkout("bp-api", "root"));
}

#[test]
fn a_linked_worktree_is_named_after_the_main_checkout_and_marked_wt() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("bp-api");
    common::git_repo(&repo);
    let linked = dir.path().join("bp-api-feature");
    common::git(
        &repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "feature",
            linked.to_str().unwrap(),
        ],
    );
    let tmux = TmuxServer::start();
    let session = tmux.new_session("feature", &linked);

    assert_eq!(pills(&left(&tmux, &session)), checkout("bp-api", "wt"));
}

#[test]
fn a_path_containing_a_space_keeps_the_space() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("my project");
    common::git_repo(&repo);
    let tmux = TmuxServer::start();
    let session = tmux.new_session("spaced", &repo);

    assert_eq!(
        pills(&left(&tmux, &session)),
        checkout("my project", "root")
    );
}

#[test]
fn a_grove_tagged_session_is_named_after_its_grove_project() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("checkout-dir");
    common::git_repo(&repo);
    let tmux = TmuxServer::start();
    let session = tmux.new_session("grove_bp_api_main_f2d1", &repo);
    tmux.set_session_option(&session, "@grove_project", "bp-api");
    tmux.set_session_option(&session, "@grove_root", repo.to_str().unwrap());
    tmux.set_session_option(&session, "@grove_worktree", "");

    assert_eq!(pills(&left(&tmux, &session)), checkout("bp-api", "root"));
}

#[test]
fn a_grove_worktree_session_is_marked_wt_from_its_tag_alone() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let session = tmux.new_session("grove_bp_api_feature_f2d1", dir.path());
    tmux.set_session_option(&session, "@grove_project", "bp-api");
    tmux.set_session_option(&session, "@grove_root", "/nowhere/bp-api");
    tmux.set_session_option(&session, "@grove_worktree", "feature");

    assert_eq!(pills(&left(&tmux, &session)), checkout("bp-api", "wt"));
}

#[test]
fn a_long_project_name_is_clipped_with_an_ellipsis() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("a-really-long-project-name-that-overflows");
    common::git_repo(&repo);
    let tmux = TmuxServer::start();
    let session = tmux.new_session("long", &repo);

    assert_eq!(
        pills(&left(&tmux, &session)),
        checkout("a-really-long-project-n…", "root")
    );
}

#[test]
fn on_a_wide_client_the_block_is_as_wide_as_the_right_one_whatever_the_locale() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("a-really-long-project-name-that-overflows");
    common::git_repo(&repo);
    let tmux = TmuxServer::start();
    let sessions = [
        tmux.new_session("scratch", dir.path()),
        tmux.new_session("long", &repo),
    ];

    for client_width in ["160", "200"] {
        for locale in [
            [("LANG", "C"), ("LC_ALL", "C")],
            [("LANG", "C.UTF-8"), ("LC_ALL", "")],
        ] {
            let right = tmux.atelier_stdout_with_env(
                &["bar", "right", repo.to_str().unwrap(), client_width],
                &locale,
            );
            for session in &sessions {
                let left = tmux.atelier_stdout_with_env(
                    &["bar", "left", "-t", session, client_width],
                    &locale,
                );
                assert_eq!(
                    visible(&left).chars().count(),
                    visible(&right).chars().count(),
                    "{client_width} columns, {locale:?}: {left}"
                );
            }
        }
    }
}

#[test]
fn on_a_narrow_client_the_padding_gives_way() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let session = tmux.new_session("scratch", dir.path());
    let block = tmux.atelier_stdout(&["bar", "left", "-t", &session, "60"]);

    assert_eq!(visible(&block), bare("scratch"));
}

#[test]
fn run_from_inside_tmux_it_finds_the_server_through_the_environment() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let session = tmux.new_session("scratch", dir.path());
    let args = ["bar", "left", "-t", &session, "200"];

    assert_eq!(
        pills(&common::stdout_of(&args, tmux.atelier_inside(&args))),
        bare("scratch")
    );
}

#[test]
fn an_unknown_session_is_an_error() {
    let tmux = TmuxServer::start();

    assert!(!tmux
        .atelier(&["bar", "left", "-t", "$999", "200"])
        .status
        .success());
}
