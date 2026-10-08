mod common;

use common::TmuxServer;

#[test]
fn a_session_outside_any_repo_is_named_after_itself() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let session = tmux.new_session("scratch", dir.path());

    assert_eq!(
        tmux.atelier_stdout(&["bar", "left", "-t", &session]),
        "scratch"
    );
}

#[test]
fn a_session_in_a_git_repo_is_named_after_the_repo() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("bp-api");
    common::git_repo(&repo);
    let deep = repo.join("src/handlers");
    std::fs::create_dir_all(&deep).unwrap();
    let tmux = TmuxServer::start();
    let session = tmux.new_session("hand-named", &deep);

    assert_eq!(
        tmux.atelier_stdout(&["bar", "left", "-t", &session]),
        "bp-api"
    );
}

#[test]
fn a_linked_worktree_is_named_after_the_main_checkout() {
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

    assert_eq!(
        tmux.atelier_stdout(&["bar", "left", "-t", &session]),
        "bp-api"
    );
}

#[test]
fn a_path_containing_a_space_keeps_the_space() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("my project");
    common::git_repo(&repo);
    let tmux = TmuxServer::start();
    let session = tmux.new_session("spaced", &repo);

    assert_eq!(
        tmux.atelier_stdout(&["bar", "left", "-t", &session]),
        "my project"
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

    assert_eq!(
        tmux.atelier_stdout(&["bar", "left", "-t", &session]),
        "bp-api"
    );
}

#[test]
fn run_from_inside_tmux_it_finds_the_server_through_the_environment() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let session = tmux.new_session("scratch", dir.path());
    let args = ["bar", "left", "-t", &session];

    assert_eq!(
        common::stdout_of(&args, tmux.atelier_inside(&args)),
        "scratch"
    );
}

#[test]
fn an_unknown_session_is_an_error() {
    let tmux = TmuxServer::start();

    assert!(!tmux
        .atelier(&["bar", "left", "-t", "$999"])
        .status
        .success());
}
