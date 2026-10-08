mod common;

use std::path::{Path, PathBuf};
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
    eventually("a terminal client attaches", || {
        tmux.tmux(&["list-clients", "-F", "#{client_control_mode}"])
            .lines()
            .any(|mode| mode == "0")
    });
    viewer
}

fn eventually(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(50));
    }
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
    eventually(
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
    eventually("the pushed left block on screen", || {
        !viewer.status_line(&tmux).starts_with(" work")
    });

    daemon.0.kill().unwrap();
    daemon.0.wait().unwrap();

    eventually("the fallback left block on screen", || {
        viewer.status_line(&tmux).starts_with(" work")
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

struct GitLog {
    dir: tempfile::TempDir,
}

impl GitLog {
    fn new() -> GitLog {
        let real = std::env::split_paths(&std::env::var_os("PATH").expect("PATH is set"))
            .map(|dir| dir.join("git"))
            .find(|git| git.is_file())
            .expect("git on PATH");
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("git");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nexec '{}' \"$@\"\n",
                dir.path().join("calls").display(),
                real.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&script, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap();
        GitLog { dir }
    }

    fn calls(&self) -> Vec<String> {
        std::fs::read_to_string(self.dir.path().join("calls"))
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    fn daemon(&self, tmux: &TmuxServer) -> Daemon {
        let path = format!(
            "{}:{}",
            self.dir.path().display(),
            std::env::var("PATH").unwrap()
        );
        Daemon(
            tmux.atelier_command(&["daemon"])
                .env("PATH", path)
                .spawn()
                .expect("daemon starts"),
        )
    }
}

fn repo_with_upstream(dir: &Path) -> PathBuf {
    let origin = dir.join("origin");
    common::git_repo(&origin);
    std::fs::write(origin.join("notes.txt"), "one\ntwo\n").unwrap();
    std::fs::write(origin.join(".gitignore"), "build/\n").unwrap();
    common::git(&origin, &["add", "."]);
    common::git(&origin, &["commit", "-qm", "notes"]);
    common::git(&origin, &["checkout", "-q", "--detach"]);
    common::git(dir, &["clone", "-q", origin.to_str().unwrap(), "work"]);
    dir.join("work")
}

fn counts_become(tmux: &TmuxServer, session: &str, counts: &str, within: Duration) {
    let deadline = Instant::now() + within;
    let mut last = String::new();
    while Instant::now() < deadline {
        last = repo_counts(&option(tmux, session, "@bar_right"));
        if last == counts {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(last, counts, "repo counts of {session} within {within:?}");
}

fn repo_counts(right: &str) -> String {
    let mut text = String::new();
    let mut rest = right;
    while let Some(start) = rest.find("#[") {
        text.push_str(&rest[..start]);
        rest = rest[start..].split_once(']').map_or("", |(_, after)| after);
    }
    text.push_str(rest);
    let before_date = text.split("%a").next().unwrap_or_default();
    before_date.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn watch_session(tmux: &TmuxServer, name: &str, dir: &Path, git: &GitLog) -> Daemon {
    tmux.new_session(name, dir);
    attach_terminal_client(tmux, name, 130);
    let daemon = git.daemon(tmux);
    pushed_left_becomes(tmux, name, &rendered_left(tmux, name, 130));
    daemon
}

#[test]
fn an_unstaged_edit_to_a_tracked_file_updates_the_counts_within_a_second() {
    let dir = tempfile::tempdir().unwrap();
    let work = repo_with_upstream(dir.path());
    let tmux = server();
    let git = GitLog::new();
    let _daemon = watch_session(&tmux, "work", &work, &git);
    counts_become(&tmux, "work", "", Duration::from_secs(5));

    std::fs::write(work.join("notes.txt"), "one\n2\nthree\n").unwrap();

    counts_become(&tmux, "work", "+2 −1", Duration::from_secs(1));
}

#[test]
fn staging_committing_and_checking_out_each_update_the_counts_within_a_second() {
    let dir = tempfile::tempdir().unwrap();
    let work = repo_with_upstream(dir.path());
    let tmux = server();
    let git = GitLog::new();
    let _daemon = watch_session(&tmux, "work", &work, &git);
    counts_become(&tmux, "work", "", Duration::from_secs(5));
    std::fs::write(work.join("new.txt"), "a\nb\n").unwrap();

    common::git(&work, &["add", "new.txt"]);
    counts_become(&tmux, "work", "+2", Duration::from_secs(1));
    std::fs::write(work.join("new.txt"), "a\nb\nc\n").unwrap();
    counts_become(&tmux, "work", "+3", Duration::from_secs(1));
    common::git(&work, &["commit", "-qam", "new"]);
    counts_become(&tmux, "work", "↑1", Duration::from_secs(1));
    common::git(&work, &["push", "-q"]);
    counts_become(&tmux, "work", "", Duration::from_secs(1));
    let origin = dir.path().join("origin");
    common::git(&origin, &["checkout", "-q", "main"]);
    common::git(
        &origin,
        &["commit", "-q", "--allow-empty", "-m", "upstream"],
    );
    common::git(&work, &["fetch", "-q"]);
    counts_become(&tmux, "work", "↓1", Duration::from_secs(1));
    common::git(&work, &["checkout", "-qb", "other", "origin/main"]);
    counts_become(&tmux, "work", "", Duration::from_secs(1));
}

#[test]
fn a_linked_worktree_follows_its_own_index_and_branch() {
    let dir = tempfile::tempdir().unwrap();
    let work = repo_with_upstream(dir.path());
    let linked = dir.path().join("linked");
    common::git(
        &work,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "wt",
            linked.to_str().unwrap(),
            "origin/main",
        ],
    );
    let tmux = server();
    let git = GitLog::new();
    let _daemon = watch_session(&tmux, "linked", &linked, &git);
    counts_become(&tmux, "linked", "", Duration::from_secs(5));

    std::fs::write(linked.join("notes.txt"), "one\n2\nthree\n").unwrap();
    counts_become(&tmux, "linked", "+2 −1", Duration::from_secs(1));
    common::git(&linked, &["commit", "-qam", "edit"]);
    counts_become(&tmux, "linked", "↑1", Duration::from_secs(1));
}

#[test]
fn no_git_process_starts_while_the_repo_is_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let work = repo_with_upstream(dir.path());
    let tmux = server();
    let git = GitLog::new();
    tmux.new_session("work", &work);
    tmux.new_session("other", &work);
    let viewer = attach_terminal_client(&tmux, "work", 130);
    let _daemon = git.daemon(&tmux);
    pushed_left_becomes(&tmux, "work", &rendered_left(&tmux, "work", 130));
    let client = viewer.client(&tmux);
    tmux.tmux(&["switch-client", "-c", &client, "-t", "other"]);
    pushed_left_becomes(&tmux, "other", &rendered_left(&tmux, "other", 130));
    std::thread::sleep(Duration::from_millis(500));
    let before = git.calls();

    std::fs::write(work.join("untracked.txt"), "a\n").unwrap();
    std::fs::create_dir(work.join("build")).unwrap();
    std::fs::write(work.join("build/out.txt"), "a\n").unwrap();
    tmux.tmux(&["switch-client", "-c", &client, "-t", "work"]);
    viewer.resize(&tmux, 100);
    pushed_left_becomes(&tmux, "work", &rendered_left(&tmux, "work", 100));
    tmux.tmux(&["switch-client", "-c", &client, "-t", "other"]);
    pushed_left_becomes(&tmux, "other", &rendered_left(&tmux, "other", 100));
    std::thread::sleep(Duration::from_millis(500));

    assert_eq!(git.calls(), before);
}

#[test]
fn a_repo_no_session_uses_any_more_is_no_longer_watched() {
    let dir = tempfile::tempdir().unwrap();
    let work = repo_with_upstream(dir.path());
    let elsewhere = tempfile::tempdir().unwrap();
    let tmux = server();
    let git = GitLog::new();
    tmux.new_session("work", &work);
    let _daemon = watch_session(&tmux, "other", elsewhere.path(), &git);
    eventually("the daemon to look at the repo", || {
        git.calls().iter().any(|call| call.contains("ls-files"))
    });

    tmux.tmux(&["kill-session", "-t", "work"]);
    eventually("the daemon to see the session go", || {
        !tmux.atelier_stdout(&["status"]).contains("work")
    });
    let before = git.calls();
    std::fs::write(work.join("new.txt"), "a\n").unwrap();
    common::git(&work, &["add", "new.txt"]);
    common::git(&work, &["commit", "-qm", "new"]);
    std::thread::sleep(Duration::from_millis(500));

    assert_eq!(git.calls(), before);
}
