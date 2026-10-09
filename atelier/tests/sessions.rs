#![allow(clippy::disallowed_methods)]

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

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
        "j/k move   h/l window   i search   (no project)\nenter switch   ctrl-x kill"
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
    let _client = tmux.attach_terminal_client("zulu");

    assert_eq!(
        labels(rows(&tmux, &current)),
        vec!["zulu     2 win  · attached", "alpha    1 win"]
    );
}

#[test]
fn a_session_seen_only_by_a_control_client_is_not_marked_attached() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let current = tmux.new_session("scratch", dir.path());
    tmux.new_session("daemon-seat", dir.path());
    let _daemon = tmux.attach_control_client("daemon-seat");

    assert_eq!(labels(rows(&tmux, &current)), vec!["daemon-seat    1 win"]);
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
        "disable-search+clear-query+rebind(j,k,q,h,l)+change-prompt(  project  )"
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

fn preview(tmux: &TmuxServer, session: &str, lines: &str) -> String {
    tmux.atelier_stdout_with_env(
        &["sessions", "preview", session],
        &[("FZF_PREVIEW_LINES", lines)],
    )
}

fn preview_once_printed(tmux: &TmuxServer, session: &str, lines: &str, last: &str) -> String {
    for _ in 0..300 {
        let shown = preview(tmux, session, lines);
        if shown.lines().last() == Some(last) {
            return shown;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the pane of {session} never printed {last}");
}

#[test]
fn the_preview_lists_the_windows_and_tails_the_current_one() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let other = tmux.new_session("other", dir.path());
    tmux.tmux(&["rename-window", "-t", &other, "editor"]);
    tmux.tmux(&[
        "new-window",
        "-t",
        &other,
        "-n",
        "logs",
        "seq 1 30; sleep 100",
    ]);

    assert_eq!(
        preview_once_printed(&tmux, &other, "10", "30"),
        "0: editor  (1p)\n1: logs  (1p) ←\n\n24\n25\n26\n27\n28\n29\n30"
    );
}

#[test]
fn the_preview_tail_skips_trailing_lines_that_are_blank_but_styled() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let other = tmux.new_session("other", dir.path());
    tmux.tmux(&[
        "respawn-window",
        "-k",
        "-t",
        &other,
        r"printf 'one\ntwo\n\033[1m   \033[0m\n\033[4m \033[0m\n'; sleep 100",
    ]);
    tmux.tmux(&["rename-window", "-t", &other, "shell"]);

    assert_eq!(
        preview_once_printed(&tmux, &other, "4", "two"),
        "0: shell  (1p) ←\n\none\ntwo"
    );
}

fn current_window(tmux: &TmuxServer, session: &str) -> String {
    tmux.tmux(&["display", "-p", "-t", session, "#{window_index}"])
}

#[test]
fn l_and_h_cycle_the_other_sessions_window_and_enter_lands_on_it() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    tmux.new_session("scratch", dir.path());
    let other = tmux.new_session("other", dir.path());
    tmux.tmux(&["new-window", "-d", "-t", &other]);
    tmux.tmux(&["new-window", "-d", "-t", &other]);
    let client = tmux.attach_control_client("scratch");

    tmux.atelier_stdout(&["sessions", "next", &other]);
    tmux.atelier_stdout(&["sessions", "next", &other]);
    assert_eq!(current_window(&tmux, &other), "2");
    tmux.atelier_stdout(&["sessions", "prev", &other]);
    assert_eq!(current_window(&tmux, &other), "1");

    tmux.atelier_stdout(&["sessions", "switch", "-c", &client.name, &other]);
    assert_eq!(
        tmux.tmux(&["display", "-p", "-c", &client.name, "#{window_index}"]),
        "1"
    );
}

#[test]
fn cycling_a_single_window_session_or_the_placeholder_is_a_quiet_no_op() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let other = tmux.new_session("other", dir.path());

    assert_eq!(tmux.atelier_stdout(&["sessions", "next", &other]), "");
    assert_eq!(tmux.atelier_stdout(&["sessions", "prev", &other]), "");
    assert_eq!(tmux.atelier_stdout(&["sessions", "next", ""]), "");
    assert_eq!(current_window(&tmux, &other), "0");
}

#[test]
fn ctrl_x_kills_the_highlighted_session_and_reloads_the_rows() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let current = tmux.new_session("scratch", dir.path());
    let doomed = tmux.new_session("doomed", dir.path());
    tmux.new_session("notes", dir.path());

    let actions = tmux.atelier_stdout(&["sessions", "kill", "-t", &current, &doomed]);

    assert_eq!(labels(rows(&tmux, &current)), vec!["notes    1 win"]);
    assert!(actions.starts_with("reload("), "{actions}");
    assert!(actions.contains("sessions rows -t"), "{actions}");
}

#[test]
fn ctrl_x_on_the_placeholder_kills_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let current = tmux.new_session("scratch", dir.path());

    assert_eq!(
        tmux.atelier_stdout(&["sessions", "kill", "-t", &current, ""]),
        ""
    );
    assert_eq!(
        tmux.tmux(&["list-sessions", "-F", "#{session_name}"]),
        "scratch"
    );
}

#[test]
fn the_placeholder_previews_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    tmux.new_session("scratch", dir.path());

    assert_eq!(preview(&tmux, "", "24"), "");
}
