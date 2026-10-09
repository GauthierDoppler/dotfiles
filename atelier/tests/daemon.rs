#![allow(clippy::disallowed_methods)]

mod common;

use std::time::{Duration, Instant};

use common::TmuxServer;

fn server() -> TmuxServer {
    let tmux = TmuxServer::start();
    tmux.tmux(&["set-option", "-gw", "automatic-rename", "off"]);
    tmux
}

fn ensure_from_tmux(tmux: &TmuxServer) {
    tmux.tmux(&[
        "run-shell",
        "-b",
        &format!("{} daemon --ensure", env!("CARGO_BIN_EXE_atelier")),
    ]);
}

fn status(tmux: &TmuxServer) -> String {
    let status = tmux.atelier_stdout(&["status"]);
    match status.split_once('\n') {
        Some((header, rest)) if header.starts_with("daemon: running (pid ") => {
            format!("daemon: running\n{rest}")
        }
        _ => status,
    }
}

fn eventually(tmux: &TmuxServer, expected: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut last = status(tmux);
    while last != expected && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
        last = status(tmux);
    }
    assert_eq!(last, expected);
}

#[test]
fn without_a_daemon_status_reads_tmux_directly() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = server();
    tmux.new_session("work", dir.path());
    tmux.tmux(&["new-window", "-d", "-t", "work", "-n", "logs"]);
    tmux.new_session("api", dir.path());

    assert_eq!(
        status(&tmux),
        "daemon: not running\napi\n  0 sh\nwork\n  0 sh\n  1 logs"
    );
}

#[test]
fn the_daemon_follows_sessions_and_windows_as_they_change() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = server();
    tmux.new_session("work", dir.path());
    ensure_from_tmux(&tmux);
    eventually(&tmux, "daemon: running\nwork\n  0 sh");

    tmux.new_session("api", dir.path());
    tmux.tmux(&["new-window", "-d", "-t", "work", "-n", "logs"]);
    eventually(
        &tmux,
        "daemon: running\napi\n  0 sh\nwork\n  0 sh\n  1 logs",
    );

    tmux.tmux(&["rename-session", "-t", "api", "backend"]);
    tmux.tmux(&["rename-window", "-t", "work:logs", "tail"]);
    eventually(
        &tmux,
        "daemon: running\nbackend\n  0 sh\nwork\n  0 sh\n  1 tail",
    );

    tmux.tmux(&["kill-window", "-t", "work:tail"]);
    tmux.tmux(&["kill-session", "-t", "backend"]);
    eventually(&tmux, "daemon: running\nwork\n  0 sh");
}

fn control_clients(tmux: &TmuxServer) -> usize {
    tmux.tmux(&["list-clients", "-F", "#{client_control_mode}"])
        .lines()
        .filter(|line| *line == "1")
        .count()
}

fn exits_within(child: &mut std::process::Child, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if child.try_wait().unwrap().is_some() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn ensuring_the_daemon_again_keeps_a_single_one() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = server();
    tmux.new_session("work", dir.path());
    ensure_from_tmux(&tmux);
    ensure_from_tmux(&tmux);
    eventually(&tmux, "daemon: running\nwork\n  0 sh");
    ensure_from_tmux(&tmux);
    std::thread::sleep(Duration::from_millis(500));

    assert_eq!(control_clients(&tmux), 1);
}

#[test]
fn a_second_daemon_on_the_same_server_exits_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = server();
    tmux.new_session("work", dir.path());
    let mut first = tmux.atelier_command(&["daemon"]).spawn().unwrap();
    eventually(&tmux, "daemon: running\nwork\n  0 sh");

    let mut second = tmux.atelier_command(&["daemon"]).spawn().unwrap();
    let second_exited = exits_within(&mut second, Duration::from_secs(5));
    let first_running = first.try_wait().unwrap().is_none();
    for daemon in [&mut first, &mut second] {
        let _ = daemon.kill();
        daemon.wait().unwrap();
    }

    assert!(second_exited);
    assert!(first_running);
}

#[test]
fn the_daemon_exits_with_the_tmux_server() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = server();
    tmux.new_session("work", dir.path());
    let mut daemon = tmux.atelier_command(&["daemon"]).spawn().unwrap();
    eventually(&tmux, "daemon: running\nwork\n  0 sh");

    tmux.tmux(&["kill-server"]);

    assert!(exits_within(&mut daemon, Duration::from_secs(5)));
}

#[test]
fn the_daemon_creates_no_session_of_its_own() {
    let tmux = server();

    assert!(tmux.atelier(&["daemon"]).status.success());
    assert_eq!(tmux.tmux(&["list-sessions", "-F", "#{session_name}"]), "");
}

#[test]
fn the_daemon_outlives_the_session_it_watches_through() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = server();
    tmux.new_session("first", dir.path());
    tmux.new_session("second", dir.path());
    ensure_from_tmux(&tmux);
    eventually(&tmux, "daemon: running\nfirst\n  0 sh\nsecond\n  0 sh");
    tmux.new_session("third", dir.path());

    tmux.tmux(&["kill-session", "-t", "first"]);
    tmux.tmux(&["kill-session", "-t", "second"]);

    eventually(&tmux, "daemon: running\nthird\n  0 sh");
    assert_eq!(control_clients(&tmux), 1);
}

#[test]
fn a_killed_daemon_leaves_status_reading_tmux_directly() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = server();
    tmux.new_session("work", dir.path());
    let mut daemon = tmux.atelier_command(&["daemon"]).spawn().unwrap();
    eventually(&tmux, "daemon: running\nwork\n  0 sh");

    daemon.kill().unwrap();
    daemon.wait().unwrap();

    assert_eq!(status(&tmux), "daemon: not running\nwork\n  0 sh");
}

#[test]
fn the_daemon_runs_when_path_lacks_tmux() {
    let dir = tempfile::tempdir().unwrap();
    let empty = tempfile::tempdir().unwrap();
    let tmux = server();
    tmux.new_session("work", dir.path());
    let mut daemon = tmux
        .atelier_command(&["daemon"])
        .env("PATH", empty.path())
        .spawn()
        .unwrap();

    eventually(&tmux, "daemon: running\nwork\n  0 sh");
    let _ = daemon.kill();
    daemon.wait().unwrap();
}

fn picker_rows(tmux: &TmuxServer, from: &str) -> String {
    tmux.atelier_stdout(&["sessions", "rows", "-t", from])
}

fn picker_order_around_the_daemon(tmux: &TmuxServer) -> (String, String) {
    let before = picker_rows(tmux, "gamma");
    let mut daemon = tmux.atelier_command(&["daemon"]).spawn().unwrap();
    eventually(
        tmux,
        "daemon: running\nalpha\n  0 sh\nbeta\n  0 sh\ngamma\n  0 sh",
    );
    let after = picker_rows(tmux, "gamma");
    let _ = daemon.kill();
    daemon.wait().unwrap();
    (before, after)
}

#[test]
fn the_daemon_attaching_leaves_the_picker_order_alone() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = server();
    for name in ["gamma", "alpha", "beta"] {
        tmux.new_session(name, dir.path());
    }

    let (before, after) = picker_order_around_the_daemon(&tmux);

    assert_eq!(after, before);
}

#[test]
fn the_daemon_attaching_keeps_the_last_attached_session_first() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = server();
    for name in ["gamma", "alpha"] {
        tmux.new_session(name, dir.path());
    }
    drop(tmux.attach_control_client("alpha"));
    std::thread::sleep(Duration::from_millis(1100));
    tmux.new_session("beta", dir.path());

    let (before, after) = picker_order_around_the_daemon(&tmux);

    assert!(before.starts_with("$1\talpha"), "{before}");
    assert_eq!(after, before);
}
