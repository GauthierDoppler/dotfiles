mod common;

use std::io::Write;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use common::TmuxServer;

fn hook(tmux: &TmuxServer, pane: Option<&str>, payload: &str) -> Output {
    hook_with_path(tmux, pane, payload, &std::env::var("PATH").unwrap())
}

fn hook_with_path(tmux: &TmuxServer, pane: Option<&str>, payload: &str, path: &str) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_atelier"));
    command
        .args(["hook", "claude"])
        .env("TMUX", format!("{},1,0", tmux.socket().display()))
        .env_remove("TMUX_PANE")
        .env("PATH", path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(pane) = pane {
        command.env("TMUX_PANE", pane);
    }
    let mut child = command.spawn().expect("atelier runs");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(payload.as_bytes())
        .unwrap();
    common::wait_bounded(child, "atelier hook claude")
}

fn event(name: &str) -> String {
    format!(r#"{{"session_id":"abc","cwd":"/tmp","hook_event_name":"{name}","message":"x"}}"#)
}

fn claude_status(tmux: &TmuxServer, window: &str) -> String {
    tmux.tmux(&["show-options", "-wqv", "-t", window, "@claude_status"])
}

struct Work {
    claude_pane: String,
    claude_window: String,
    other_window: String,
}

fn work_session(tmux: &TmuxServer, dir: &std::path::Path) -> Work {
    tmux.new_session("work", dir);
    let claude_window = tmux.tmux(&["display", "-p", "-t", "work:", "#{window_id}"]);
    let claude_pane = tmux.tmux(&["display", "-p", "-t", "work:", "#{pane_id}"]);
    let other_window = tmux.tmux(&[
        "new-window",
        "-d",
        "-t",
        "work:",
        "-P",
        "-F",
        "#{window_id}",
    ]);
    Work {
        claude_pane,
        claude_window,
        other_window,
    }
}

fn wait_for_a_client(tmux: &TmuxServer) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let clients = tmux.tmux(&["list-clients", "-F", "#{client_name}"]);
        if !clients.is_empty() {
            return;
        }
        assert!(Instant::now() < deadline, "no client attached");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn attach_terminal_client(tmux: &TmuxServer, session: &str) {
    let viewer = tempfile::tempdir().unwrap();
    tmux.new_session("viewer", viewer.path());
    tmux.tmux(&["resize-window", "-t", "viewer:", "-x", "80", "-y", "24"]);
    let attach = format!(
        "unset TMUX; exec tmux -S '{}' attach -t '{}'",
        tmux.socket().display(),
        session
    );
    tmux.tmux(&["respawn-pane", "-k", "-t", "viewer:", &attach]);
    wait_for_a_client(tmux);
}

struct ControlClient(Child);

impl Drop for ControlClient {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn attach_control_client(tmux: &TmuxServer, session: &str) -> ControlClient {
    let child = Command::new("tmux")
        .arg("-S")
        .arg(tmux.socket())
        .args(["-C", "attach", "-t", session])
        .env_remove("TMUX")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("control client starts");
    let client = ControlClient(child);
    wait_for_a_client(tmux);
    client
}

#[test]
fn notification_marks_the_window_waiting() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let work = work_session(&tmux, dir.path());

    let output = hook(&tmux, Some(&work.claude_pane), &event("Notification"));

    assert!(output.status.success());
    assert_eq!(claude_status(&tmux, &work.claude_window), "waiting");
    assert_eq!(claude_status(&tmux, &work.other_window), "");
}

#[test]
fn the_window_is_marked_when_path_lacks_tmux() {
    let dir = tempfile::tempdir().unwrap();
    let empty = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let work = work_session(&tmux, dir.path());

    let output = hook_with_path(
        &tmux,
        Some(&work.claude_pane),
        &event("Notification"),
        empty.path().to_str().unwrap(),
    );

    assert!(output.status.success());
    assert_eq!(claude_status(&tmux, &work.claude_window), "waiting");
}

#[test]
fn stop_marks_the_window_done() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let work = work_session(&tmux, dir.path());
    hook(&tmux, Some(&work.claude_pane), &event("Notification"));

    hook(&tmux, Some(&work.claude_pane), &event("Stop"));

    assert_eq!(claude_status(&tmux, &work.claude_window), "done");
}

#[test]
fn prompt_submit_clears_the_marker() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let work = work_session(&tmux, dir.path());
    hook(&tmux, Some(&work.claude_pane), &event("Stop"));

    let output = hook(&tmux, Some(&work.claude_pane), &event("UserPromptSubmit"));

    assert!(output.status.success());
    assert_eq!(claude_status(&tmux, &work.claude_window), "");
}

#[test]
fn the_pane_s_own_window_is_marked_not_the_session_s_current_one() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let work = work_session(&tmux, dir.path());
    tmux.tmux(&["select-window", "-t", &work.other_window]);

    hook(&tmux, Some(&work.claude_pane), &event("Notification"));

    assert_eq!(claude_status(&tmux, &work.claude_window), "waiting");
    assert_eq!(claude_status(&tmux, &work.other_window), "");
}

#[test]
fn a_window_watched_by_an_attached_terminal_is_not_marked() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let work = work_session(&tmux, dir.path());
    attach_terminal_client(&tmux, "work");

    let output = hook(&tmux, Some(&work.claude_pane), &event("Notification"));

    assert!(output.status.success());
    assert_eq!(claude_status(&tmux, &work.claude_window), "");
}

#[test]
fn a_window_in_the_background_of_an_attached_terminal_is_marked() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let work = work_session(&tmux, dir.path());
    tmux.tmux(&["select-window", "-t", &work.other_window]);
    attach_terminal_client(&tmux, "work");

    hook(&tmux, Some(&work.claude_pane), &event("Notification"));

    assert_eq!(claude_status(&tmux, &work.claude_window), "waiting");
}

#[test]
fn a_control_mode_client_does_not_count_as_watching() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let work = work_session(&tmux, dir.path());
    let _control = attach_control_client(&tmux, "work");

    hook(&tmux, Some(&work.claude_pane), &event("Stop"));

    assert_eq!(claude_status(&tmux, &work.claude_window), "done");
}

#[test]
fn outside_tmux_the_hook_does_nothing_and_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let work = work_session(&tmux, dir.path());

    let output = hook(&tmux, None, &event("Notification"));

    assert!(output.status.success());
    assert_eq!(claude_status(&tmux, &work.claude_window), "");
}

#[test]
fn a_payload_that_is_not_json_succeeds_without_marking() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let work = work_session(&tmux, dir.path());

    let output = hook(&tmux, Some(&work.claude_pane), "not json");

    assert!(output.status.success());
    assert_eq!(claude_status(&tmux, &work.claude_window), "");
}

#[test]
fn a_pane_that_no_longer_exists_succeeds_without_marking() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let work = work_session(&tmux, dir.path());

    let output = hook(&tmux, Some("%999"), &event("Notification"));

    assert!(output.status.success());
    assert_eq!(claude_status(&tmux, &work.claude_window), "");
}

#[test]
fn selecting_the_window_clears_its_marker() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = TmuxServer::start();
    let conf =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../dot_tmux.conf")).unwrap();
    let clear = conf
        .lines()
        .find(|line| line.starts_with("set-hook -g after-select-window"))
        .expect("dot_tmux.conf clears the marker on select");
    let sourced = dir.path().join("hook.conf");
    std::fs::write(&sourced, clear).unwrap();
    tmux.tmux(&["source-file", sourced.to_str().unwrap()]);
    let work = work_session(&tmux, dir.path());
    tmux.tmux(&["select-window", "-t", &work.other_window]);
    hook(&tmux, Some(&work.claude_pane), &event("Notification"));

    tmux.tmux(&["select-window", "-t", &work.claude_window]);

    assert_eq!(claude_status(&tmux, &work.claude_window), "");
}
