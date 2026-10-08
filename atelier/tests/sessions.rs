mod common;

use std::path::{Path, PathBuf};

use common::TmuxServer;

fn dir_in(parent: &Path, name: &str) -> PathBuf {
    let dir = parent.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn grove_session(tmux: &TmuxServer, name: &str, project: &str, root: &Path) -> String {
    let session = tmux.new_session(name, root);
    tmux.set_session_option(&session, "@grove_project", project);
    tmux.set_session_option(&session, "@grove_root", root.to_str().unwrap());
    tmux.set_session_option(&session, "@grove_worktree", "");
    session
}

fn labels(rows: Vec<(String, String)>) -> Vec<String> {
    rows.into_iter().map(|(_, label)| label).collect()
}

fn rows(tmux: &TmuxServer, session: &str) -> Vec<(String, String)> {
    tmux.atelier_stdout(&["sessions", "rows", "-t", session])
        .lines()
        .map(|row| {
            let (id, label) = row.split_once('\t').expect("row is id<TAB>label");
            (id.to_string(), label.to_string())
        })
        .collect()
}

#[test]
fn a_lone_session_lists_a_placeholder_with_nothing_to_switch_to() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let session = tmux.new_session("scratch", dir.path());

    assert_eq!(
        rows(&tmux, &session),
        vec![(String::new(), "(no other session)".to_string())]
    );
}

#[test]
fn a_project_session_lists_the_other_sessions_of_its_project_only() {
    let dir = tempfile::tempdir().unwrap();
    let api = dir_in(dir.path(), "api");
    let web = dir_in(dir.path(), "web");
    let tmux = TmuxServer::start();
    let current = grove_session(&tmux, "grove_api_main_0a1b", "api", &api);
    grove_session(&tmux, "grove_api_feature_0a1b", "api", &api);
    grove_session(&tmux, "grove_web_main_9f9f", "web", &web);

    assert_eq!(
        labels(rows(&tmux, &current)),
        vec!["grove_api_feature_0a1b    1 win"]
    );
}

#[test]
fn tab_toggles_between_the_project_and_every_session() {
    let dir = tempfile::tempdir().unwrap();
    let api = dir_in(dir.path(), "api");
    let web = dir_in(dir.path(), "web");
    let tmux = TmuxServer::start();
    let current = grove_session(&tmux, "grove_api_main_0a1b", "api", &api);
    grove_session(&tmux, "grove_api_feature_0a1b", "api", &api);
    grove_session(&tmux, "grove_web_main_9f9f", "web", &web);
    tmux.new_session("notes", dir.path());

    tmux.atelier_stdout(&["sessions", "toggle", "-t", &current]);
    assert_eq!(
        labels(rows(&tmux, &current)),
        vec![
            "grove_api_feature_0a1b    1 win",
            "grove_web_main_9f9f       1 win",
            "notes                     1 win",
        ]
    );

    tmux.atelier_stdout(&["sessions", "toggle", "-t", &current]);
    assert_eq!(
        labels(rows(&tmux, &current)),
        vec!["grove_api_feature_0a1b    1 win"]
    );
}

#[test]
fn the_toggle_tells_fzf_to_reload_the_rows_and_relabel_the_prompt() {
    let dir = tempfile::tempdir().unwrap();
    let api = dir_in(dir.path(), "api");
    let tmux = TmuxServer::start();
    let current = grove_session(&tmux, "grove_api_main_0a1b", "api", &api);

    let actions = tmux.atelier_stdout(&["sessions", "toggle", "-t", &current]);

    assert!(actions.starts_with("reload("), "{actions}");
    assert!(actions.contains("sessions rows -t"), "{actions}");
    assert!(actions.contains("change-prompt(  all  )"), "{actions}");
    assert!(actions.contains("sessions header -t"), "{actions}");
}

#[test]
fn a_project_with_no_other_session_says_so_and_tab_still_reaches_the_others() {
    let dir = tempfile::tempdir().unwrap();
    let api = dir_in(dir.path(), "api");
    let web = dir_in(dir.path(), "web");
    let tmux = TmuxServer::start();
    let current = grove_session(&tmux, "grove_api_main_0a1b", "api", &api);
    grove_session(&tmux, "grove_web_main_9f9f", "web", &web);

    assert_eq!(
        rows(&tmux, &current),
        vec![(
            String::new(),
            "(no other session in this project)".to_string()
        )]
    );

    tmux.atelier_stdout(&["sessions", "toggle", "-t", &current]);
    assert_eq!(
        labels(rows(&tmux, &current)),
        vec!["grove_web_main_9f9f    1 win"]
    );
}

#[test]
fn a_hand_named_session_that_looks_like_grove_belongs_to_no_project() {
    let dir = tempfile::tempdir().unwrap();
    let api = dir_in(dir.path(), "api");
    let tmux = TmuxServer::start();
    let current = tmux.new_session("api_perf_beef", dir.path());
    grove_session(&tmux, "grove_api_main_beef", "api", &api);
    tmux.new_session("notes", dir.path());

    assert_eq!(
        labels(rows(&tmux, &current)),
        vec![
            "grove_api_main_beef    1 win",
            "notes                  1 win"
        ]
    );
    assert_eq!(
        tmux.atelier_stdout(&["sessions", "toggle", "-t", &current]),
        ""
    );
    assert_eq!(
        tmux.atelier_stdout(&["sessions", "header", "-t", &current]),
        "j/k move   i search   (no project)\nenter switch"
    );
}

#[test]
fn a_hand_named_session_in_a_repo_shares_the_project_of_its_grove_sessions() {
    let dir = tempfile::tempdir().unwrap();
    let api = dir.path().join("api");
    common::git_repo(&api);
    let api = api.canonicalize().unwrap();
    let web = dir_in(dir.path(), "web");
    let tmux = TmuxServer::start();
    let current = tmux.new_session("debugging", &dir_in(&api, "src"));
    grove_session(&tmux, "grove_api_main_0a1b", "api", &api);
    grove_session(&tmux, "grove_web_main_9f9f", "web", &web);

    assert_eq!(
        labels(rows(&tmux, &current)),
        vec!["grove_api_main_0a1b    1 win"]
    );
}

#[test]
fn the_last_attached_session_comes_first_and_is_marked_attached() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let current = tmux.new_session("scratch", dir.path());
    tmux.new_session("alpha", dir.path());
    tmux.new_session("zulu", dir.path());
    tmux.tmux(&["new-window", "-d", "-t", "zulu"]);
    let _client = tmux.attach_control_client("zulu");

    assert_eq!(
        labels(rows(&tmux, &current)),
        vec!["zulu     2 win  · attached", "alpha    1 win"]
    );
}

#[test]
fn enter_switches_the_client_to_the_highlighted_session() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let current = tmux.new_session("scratch", dir.path());
    tmux.new_session("notes", dir.path());
    let client = tmux.attach_control_client("scratch");
    let (notes, _) = rows(&tmux, &current).remove(0);

    tmux.atelier_stdout(&["sessions", "switch", "-c", &client.name, &notes]);

    assert_eq!(tmux.client_session(&client), "notes");
}

#[test]
fn enter_on_the_placeholder_leaves_the_client_where_it_is() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let current = tmux.new_session("scratch", dir.path());
    let client = tmux.attach_control_client("scratch");
    let (placeholder, _) = rows(&tmux, &current).remove(0);

    tmux.atelier_stdout(&["sessions", "switch", "-c", &client.name, &placeholder]);

    assert_eq!(tmux.client_session(&client), "scratch");
}

#[test]
fn esc_while_searching_goes_back_to_navigation_with_the_scope_prompt() {
    let dir = tempfile::tempdir().unwrap();
    let api = dir_in(dir.path(), "api");
    let tmux = TmuxServer::start();
    let current = grove_session(&tmux, "grove_api_main_0a1b", "api", &api);

    assert_eq!(
        tmux.atelier_stdout_with_env(
            &["sessions", "escape", "-t", &current],
            &[("FZF_INPUT_STATE", "enabled")]
        ),
        "disable-search+clear-query+rebind(j,k,q)+change-prompt(  project  )"
    );
}

#[test]
fn esc_while_navigating_closes_the_picker() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let current = tmux.new_session("scratch", dir.path());

    assert_eq!(
        tmux.atelier_stdout_with_env(
            &["sessions", "escape", "-t", &current],
            &[("FZF_INPUT_STATE", "disabled")]
        ),
        "abort"
    );
}
