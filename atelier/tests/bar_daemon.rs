mod common;

use std::path::Path;
use std::process::Child;
use std::time::{Duration, Instant};

use common::TmuxServer;

struct Daemon(Child);

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn daemon(tmux: &TmuxServer) -> Daemon {
    Daemon(
        tmux.atelier_command(&["daemon"])
            .spawn()
            .expect("daemon starts"),
    )
}

fn server() -> TmuxServer {
    let tmux = TmuxServer::start();
    tmux.tmux(&["set-option", "-gw", "automatic-rename", "off"]);
    tmux
}

struct Viewer {
    window: String,
}

impl Viewer {
    fn client(&self, tmux: &TmuxServer) -> String {
        tmux.tmux(&[
            "list-clients",
            "-F",
            "#{?client_control_mode,,#{client_name}}",
        ])
        .lines()
        .find(|name| !name.is_empty())
        .expect("a terminal client")
        .to_string()
    }

    fn status_line(&self, tmux: &TmuxServer) -> String {
        let screen = tmux.tmux(&["capture-pane", "-p", "-t", &self.window]);
        screen.lines().last().unwrap_or_default().to_string()
    }

    fn resize(&self, tmux: &TmuxServer, width: u16) {
        tmux.tmux(&[
            "resize-window",
            "-t",
            &self.window,
            "-x",
            &width.to_string(),
            "-y",
            "24",
        ]);
    }
}

fn attach_terminal_client(tmux: &TmuxServer, session: &str, width: u16) -> Viewer {
    let dir = tempfile::tempdir().unwrap();
    let window = tmux.new_session("viewer", dir.path());
    let viewer = Viewer {
        window: format!("{window}:"),
    };
    tmux.tmux(&["set-option", "-t", "viewer", "window-size", "manual"]);
    viewer.resize(tmux, width);
    let attach = format!(
        "unset TMUX; exec tmux -S '{}' attach -t '{}'",
        tmux.socket().display(),
        session
    );
    tmux.tmux(&["respawn-pane", "-k", "-t", &viewer.window, &attach]);
    common::wait_until("a terminal client attaches", || {
        tmux.tmux(&["list-clients", "-F", "#{client_control_mode}"])
            .lines()
            .any(|mode| mode == "0")
    });
    viewer
}

fn option(tmux: &TmuxServer, session: &str, name: &str) -> String {
    tmux.tmux(&["-u", "show-options", "-t", session, "-qv", name])
}

fn rendered_left(tmux: &TmuxServer, session: &str, width: u16) -> String {
    tmux.atelier_stdout(&["bar", "left", "-t", session, &width.to_string()])
}

fn pushed_left_becomes(tmux: &TmuxServer, session: &str, expected: &str) {
    let mut last = String::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        last = option(tmux, session, "@bar_left");
        if last == expected {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(last, expected, "@bar_left of {session}");
}

fn conf_line(prefix: &str) -> String {
    let conf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../dot_tmux.conf");
    std::fs::read_to_string(conf)
        .expect("dot_tmux.conf readable")
        .lines()
        .find(|line| line.starts_with(prefix))
        .unwrap_or_else(|| panic!("no `{prefix}` line in dot_tmux.conf"))
        .to_string()
}

fn load_the_bar_from_dot_tmux_conf(tmux: &TmuxServer, home: &Path) {
    let conf = home.join("bar.conf");
    let lines = [
        conf_line("set -g status-left '"),
        conf_line("set -g status-right '"),
    ];
    std::fs::write(&conf, lines.join("\n")).unwrap();
    tmux.tmux(&["set-environment", "-g", "HOME", home.to_str().unwrap()]);
    tmux.tmux(&["source-file", conf.to_str().unwrap()]);
}

fn shown(tmux: &TmuxServer, client: &str, session: &str, format: &str) -> String {
    tmux.tmux(&[
        "-u",
        "display-message",
        "-p",
        "-c",
        client,
        "-t",
        session,
        format,
    ])
}

#[test]
fn the_attached_session_gets_both_blocks_and_switching_pushes_the_other() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let tmux = server();
    tmux.new_session("work", dir.path());
    tmux.new_session("api", dir.path());
    let viewer = attach_terminal_client(&tmux, "work", 130);
    load_the_bar_from_dot_tmux_conf(&tmux, home.path());
    let _daemon = daemon(&tmux);

    pushed_left_becomes(&tmux, "work", &rendered_left(&tmux, "work", 130));
    assert_ne!(option(&tmux, "work", "@bar_right"), "");
    assert_eq!(option(&tmux, "api", "@bar_left"), "");

    tmux.tmux(&["switch-client", "-c", &viewer.client(&tmux), "-t", "api"]);

    pushed_left_becomes(&tmux, "api", &rendered_left(&tmux, "api", 130));
    assert_ne!(option(&tmux, "api", "@bar_right"), "");
    common::wait_until(
        "the pushed api block on screen, with no atelier for #()",
        || {
            let line = viewer.status_line(&tmux);
            line.contains(" api ") && !line.starts_with(" api")
        },
    );
}

#[test]
fn grove_options_and_a_path_with_spaces_reach_the_pushed_left_block() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("my repo");
    common::git_repo(&repo);
    let tmux = server();
    let session = tmux.new_session("grove_my_repo_main_f2d1", &repo);
    tmux.set_session_option(&session, "@grove_project", "my repo");
    tmux.set_session_option(&session, "@grove_root", repo.to_str().unwrap());
    tmux.set_session_option(&session, "@grove_worktree", "");
    attach_terminal_client(&tmux, &session, 130);
    let _daemon = daemon(&tmux);

    let expected = rendered_left(&tmux, &session, 130);
    assert!(expected.contains(" my repo "), "{expected}");
    pushed_left_becomes(&tmux, &session, &expected);
}

#[test]
fn resizing_the_client_re_renders_at_the_new_width_tier() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = server();
    tmux.new_session("work", dir.path());
    let viewer = attach_terminal_client(&tmux, "work", 130);
    let _daemon = daemon(&tmux);
    pushed_left_becomes(&tmux, "work", &rendered_left(&tmux, "work", 130));
    assert!(option(&tmux, "work", "@bar_right").contains("%a %d %b"));

    viewer.resize(&tmux, 100);

    pushed_left_becomes(&tmux, "work", &rendered_left(&tmux, "work", 100));
    assert!(!option(&tmux, "work", "@bar_right").contains("%a %d %b"));
}

#[test]
fn the_pushed_right_block_shows_what_the_fallback_renders() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let tmux = server();
    tmux.new_session("work", dir.path());
    let viewer = attach_terminal_client(&tmux, "work", 130);
    load_the_bar_from_dot_tmux_conf(&tmux, home.path());
    let _daemon = daemon(&tmux);
    pushed_left_becomes(&tmux, "work", &rendered_left(&tmux, "work", 130));
    let client = viewer.client(&tmux);

    for key_table in ["root", "prefix"] {
        tmux.tmux(&["switch-client", "-c", &client, "-T", key_table]);
        let path = dir.path().to_str().unwrap();
        let fallback = tmux.atelier_stdout(&["bar", "right", path, "130", key_table]);
        assert_eq!(shown(&tmux, &client, "work", "#{T:status-right}"), fallback);
    }
    assert_eq!(
        shown(&tmux, &client, "work", "#{E:status-left}"),
        rendered_left(&tmux, "work", 130)
    );
}

#[test]
fn killing_the_daemon_leaves_the_bar_on_its_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let tmux = server();
    tmux.new_session("work", dir.path());
    let viewer = attach_terminal_client(&tmux, "work", 130);
    load_the_bar_from_dot_tmux_conf(&tmux, home.path());
    let mut daemon = daemon(&tmux);
    pushed_left_becomes(&tmux, "work", &rendered_left(&tmux, "work", 130));
    common::wait_until("the pushed left block on screen", || {
        !viewer.status_line(&tmux).starts_with(" work")
    });

    daemon.0.kill().unwrap();
    daemon.0.wait().unwrap();

    common::wait_until("the fallback left block on screen", || {
        viewer.status_line(&tmux).starts_with(" work")
    });
}

#[test]
fn without_atelier_the_bar_still_shows_the_session_name_and_the_clock() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let tmux = server();
    tmux.new_session("work", dir.path());
    let viewer = attach_terminal_client(&tmux, "work", 130);
    load_the_bar_from_dot_tmux_conf(&tmux, home.path());

    common::wait_until("both fallbacks on screen", || {
        let line = viewer.status_line(&tmux);
        let clock = line.trim_end().rsplit(' ').next().unwrap_or_default();
        line.starts_with(" work")
            && clock.len() == 5
            && clock.as_bytes()[2] == b':'
            && clock.bytes().filter(u8::is_ascii_digit).count() == 4
    });
}

#[test]
fn a_session_seen_only_by_control_clients_gets_no_bar() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = server();
    tmux.new_session("work", dir.path());
    tmux.new_session("watched", dir.path());
    let _control = tmux.attach_control_client("work");
    attach_terminal_client(&tmux, "watched", 130);
    let _daemon = daemon(&tmux);
    pushed_left_becomes(&tmux, "watched", &rendered_left(&tmux, "watched", 130));

    std::thread::sleep(Duration::from_millis(300));

    assert_eq!(option(&tmux, "work", "@bar_left"), "");
}

#[test]
fn a_control_client_does_not_lend_its_width_to_the_bar() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = server();
    tmux.new_session("work", dir.path());
    let viewer = attach_terminal_client(&tmux, "work", 130);
    let _daemon = daemon(&tmux);
    pushed_left_becomes(&tmux, "work", &rendered_left(&tmux, "work", 130));

    let _control = tmux.attach_control_client("work");
    viewer.resize(&tmux, 100);

    pushed_left_becomes(&tmux, "work", &rendered_left(&tmux, "work", 100));
}
