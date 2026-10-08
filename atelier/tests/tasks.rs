mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use common::TmuxServer;

struct Project {
    _dir: tempfile::TempDir,
    root: PathBuf,
    tmpdir: PathBuf,
}

impl Project {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("app");
        let tmpdir = dir.path().join("tmp");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&tmpdir).unwrap();
        Project {
            root,
            tmpdir,
            _dir: dir,
        }
    }

    fn task(&self, name: &str, body: &str) -> &Self {
        self.file(name, body, 0o755)
    }

    fn file(&self, name: &str, body: &str, mode: u32) -> &Self {
        let path = self.root.join(".tmux").join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, body).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        self
    }

    fn out(&self, name: &str) -> PathBuf {
        self.tmpdir.join(name)
    }
}

fn tasks(tmux: &TmuxServer, project: &Project, cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_atelier"))
        .arg("--socket")
        .arg(tmux.socket())
        .arg("tasks")
        .args(args)
        .current_dir(cwd)
        .env("TMPDIR", &project.tmpdir)
        .env_remove("TMUX")
        .output()
        .expect("atelier runs")
}

fn rows(tmux: &TmuxServer, project: &Project, session: &str) -> Vec<String> {
    let args = ["list", "-t", session];
    common::stdout_of(&args, tasks(tmux, project, &project.root, &args))
        .lines()
        .map(str::to_string)
        .collect()
}

fn callback(tmux: &TmuxServer, project: &Project, session: &str, name: &str) -> String {
    let args = [name, "-t", session];
    common::stdout_of(&args, tasks(tmux, project, &project.root, &args))
}

fn run(tmux: &TmuxServer, project: &Project, session: &str, cwd: &Path, task: &str) {
    let args = ["run", "-t", session, task];
    common::stdout_of(&args, tasks(tmux, project, cwd, &args));
}

fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let start = Instant::now();
    while !done() {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "timed out waiting for {what}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn window_option(tmux: &TmuxServer, session: &str, window: &str, option: &str) -> String {
    tmux.tmux(&[
        "display-message",
        "-p",
        "-t",
        &format!("{session}:{window}"),
        &format!("#{{{option}}}"),
    ])
}

fn windows_named(tmux: &TmuxServer, session: &str, name: &str) -> usize {
    tmux.tmux(&["list-windows", "-t", session, "-F", "#{window_name}"])
        .lines()
        .filter(|line| *line == name)
        .count()
}

#[test]
fn executable_files_at_depth_one_and_two_are_tasks() {
    let project = Project::new();
    project
        .task("doctor", "#!/bin/sh\n")
        .task("android/build", "#!/bin/sh\n")
        .task("android/deep/nested", "#!/bin/sh\n")
        .file("lib/common.sh", "helper\n", 0o644)
        .file("notes", "#!/bin/sh\n", 0o644);
    let tmux = TmuxServer::start();
    let session = tmux.new_session("app", &project.root);

    assert_eq!(rows(&tmux, &project, &session), ["android/build", "doctor"]);
}

#[test]
fn the_description_comes_from_the_task_header() {
    let project = Project::new();
    project
        .task(
            "android/build",
            "#!/bin/sh\n# task: assemble the debug APK\n# tmux: window\n",
        )
        .task("doctor", "#!/bin/sh\n#task:check the toolchain\n");
    let tmux = TmuxServer::start();
    let session = tmux.new_session("app", &project.root);

    assert_eq!(
        rows(&tmux, &project, &session),
        [
            "android/build  assemble the debug APK",
            "doctor         check the toolchain"
        ]
    );
}

#[test]
fn a_header_below_line_twenty_is_ignored() {
    let project = Project::new();
    let late = format!("#!/bin/sh\n{}# task: too late\n", "true\n".repeat(19));
    project.task("late", &late);
    let tmux = TmuxServer::start();
    let session = tmux.new_session("app", &project.root);

    assert_eq!(rows(&tmux, &project, &session), ["late"]);
}

#[test]
fn a_project_without_tmux_folder_gets_a_placeholder_row() {
    let project = Project::new();
    let tmux = TmuxServer::start();
    let session = tmux.new_session("app", &project.root);

    assert_eq!(
        rows(&tmux, &project, &session),
        ["(no .tmux/ directory in this project)"]
    );
}

#[test]
fn a_tmux_folder_without_executable_gets_a_placeholder_row() {
    let project = Project::new();
    project.file("lib/common.sh", "helper\n", 0o644);
    let tmux = TmuxServer::start();
    let session = tmux.new_session("app", &project.root);

    assert_eq!(
        rows(&tmux, &project, &session),
        ["(no executable task in .tmux/)"]
    );
}

#[test]
fn the_root_is_the_session_path_not_the_cwd() {
    let project = Project::new();
    project.task("doctor", "#!/bin/sh\n");
    let deep = project.root.join("src/main/kotlin");
    std::fs::create_dir_all(&deep).unwrap();
    let elsewhere = Project::new();
    elsewhere.task("other", "#!/bin/sh\n");
    let tmux = TmuxServer::start();
    let session = tmux.new_session("app", &project.root);
    tmux.tmux(&["new-window", "-t", &session, "-c", deep.to_str().unwrap()]);

    let args = ["list", "-t", &session];
    let listed = common::stdout_of(&args, tasks(&tmux, &project, &elsewhere.root, &args));

    assert_eq!(listed, "doctor");
}

#[test]
fn a_session_opened_in_a_subdirectory_finds_the_tasks_above_it() {
    let project = Project::new();
    project.task("doctor", "#!/bin/sh\n");
    let deep = project.root.join("src/main/kotlin");
    std::fs::create_dir_all(&deep).unwrap();
    let tmux = TmuxServer::start();
    let session = tmux.new_session("app", &deep);

    assert_eq!(rows(&tmux, &project, &session), ["doctor"]);
}

#[test]
fn tab_cycles_through_the_groups_and_back_to_all() {
    let project = Project::new();
    project
        .task("doctor", "#!/bin/sh\n")
        .task("ios/simulator", "#!/bin/sh\n")
        .task("android/build", "#!/bin/sh\n")
        .task("android/logcat", "#!/bin/sh\n");
    let tmux = TmuxServer::start();
    let session = tmux.new_session("app", &project.root);

    let mut seen = Vec::new();
    for _ in 0..4 {
        let prompt = callback(&tmux, &project, &session, "prompt");
        seen.push(format!(
            "{prompt}| {}",
            rows(&tmux, &project, &session).join(", ")
        ));
        callback(&tmux, &project, &session, "advance");
    }

    assert_eq!(
        seen,
        [
            "  all  | android/build, android/logcat, doctor, ios/simulator",
            "  android  | android/build, android/logcat",
            "  ios  | ios/simulator",
            "  all  | android/build, android/logcat, doctor, ios/simulator",
        ]
    );
}

#[test]
fn the_header_offers_tab_only_when_there_are_groups() {
    let flat = Project::new();
    flat.task("doctor", "#!/bin/sh\n");
    let grouped = Project::new();
    grouped.task("android/build", "#!/bin/sh\n");
    let tmux = TmuxServer::start();
    let flat_session = tmux.new_session("flat", &flat.root);
    let grouped_session = tmux.new_session("grouped", &grouped.root);

    assert_eq!(
        [
            callback(&tmux, &flat, &flat_session, "header"),
            callback(&tmux, &grouped, &grouped_session, "header"),
        ],
        [
            "j/k move   enter run   i search   ctrl-e edit",
            "j/k move   enter run   tab group   i search   ctrl-e edit",
        ]
    );
}

fn gated_task(project: &Project) -> String {
    let gate = project.out("gate");
    format!(
        "#!/bin/sh\n# task: wait for the gate\nwhile [ ! -e '{gate}' ]; do sleep 0.05; done\nexit \"$(cat '{gate}')\"\n",
        gate = gate.display()
    )
}

fn open_gate(project: &Project, code: &str) {
    let partial = project.out("gate.partial");
    std::fs::write(&partial, code).unwrap();
    std::fs::rename(partial, project.out("gate")).unwrap();
}

fn canonical(path: impl AsRef<Path>) -> PathBuf {
    std::fs::canonicalize(path).unwrap()
}

#[test]
fn a_task_without_headers_runs_in_a_window_from_the_project_root() {
    let project = Project::new();
    let out = project.out("env");
    project.task(
        "android/build",
        &format!(
            "#!/bin/sh\npwd -P > '{out}'\nprintf '%s\\n%s\\n' \"$TMUX_TASK_ROOT\" \"$TMUX_TASK_NAME\" >> '{out}'\n",
            out = out.display()
        ),
    );
    let deep = project.root.join("src/main/kotlin");
    std::fs::create_dir_all(&deep).unwrap();
    let tmux = TmuxServer::start();
    let session = tmux.new_session("app", &project.root);

    run(&tmux, &project, &session, &deep, "android/build");
    wait_until("the task to succeed", || {
        window_option(&tmux, &session, "android-build", "@task_status") == "ok"
    });

    let written = std::fs::read_to_string(&out).unwrap();
    let lines: Vec<&str> = written.lines().collect();
    assert_eq!(
        (
            canonical(lines[0]),
            canonical(lines[1]),
            lines[2],
            tmux.tmux(&["display-message", "-p", "-t", &session, "#{window_name}"]),
        ),
        (
            canonical(&project.root),
            canonical(&project.root),
            "android/build",
            "android-build".to_string(),
        )
    );
}

#[test]
fn a_failing_task_marks_its_window_failed() {
    let project = Project::new();
    project.task("lint", "#!/bin/sh\nexit 3\n");
    let tmux = TmuxServer::start();
    let session = tmux.new_session("app", &project.root);

    run(&tmux, &project, &session, &project.root, "lint");

    wait_until("the task to fail", || {
        window_option(&tmux, &session, "lint", "@task_status") == "fail"
    });
}

#[test]
fn rerunning_reuses_the_window_and_resets_the_marker_to_running() {
    let project = Project::new();
    project.task("build", &gated_task(&project));
    let tmux = TmuxServer::start();
    let session = tmux.new_session("app", &project.root);
    run(&tmux, &project, &session, &project.root, "build");
    open_gate(&project, "1");
    wait_until("the first run to fail", || {
        window_option(&tmux, &session, "build", "@task_status") == "fail"
    });
    let first = window_option(&tmux, &session, "build", "window_id");
    std::fs::remove_file(project.out("gate")).unwrap();

    run(&tmux, &project, &session, &project.root, "build");
    wait_until("the re-run to show running", || {
        window_option(&tmux, &session, "build", "@task_status") == "running"
    });
    open_gate(&project, "0");
    wait_until("the re-run to succeed", || {
        window_option(&tmux, &session, "build", "@task_status") == "ok"
    });

    assert_eq!(
        (
            windows_named(&tmux, &session, "build"),
            window_option(&tmux, &session, "build", "window_id"),
        ),
        (1, first)
    );
}

#[test]
fn the_task_run_last_is_listed_first() {
    let project = Project::new();
    project
        .task("android/build", "#!/bin/sh\n")
        .task("doctor", "#!/bin/sh\n")
        .task("ios/simulator", "#!/bin/sh\n");
    let tmux = TmuxServer::start();
    let session = tmux.new_session("app", &project.root);

    run(&tmux, &project, &session, &project.root, "doctor");
    run(&tmux, &project, &session, &project.root, "ios/simulator");

    assert_eq!(
        rows(&tmux, &project, &session),
        ["ios/simulator", "doctor", "android/build"]
    );
}

#[test]
fn close_ok_closes_the_window_on_success() {
    let project = Project::new();
    project.task("launch", "#!/bin/sh\n# close: ok\ntrue\n");
    let tmux = TmuxServer::start();
    let session = tmux.new_session("app", &project.root);

    run(&tmux, &project, &session, &project.root, "launch");

    wait_until("the window to close", || {
        windows_named(&tmux, &session, "launch") == 0
    });
}

struct Client(std::process::Child);

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn attach(tmux: &TmuxServer, session: &str) -> Client {
    let attach = format!(
        "tmux -S '{}' attach-session -t '{session}'",
        tmux.socket().display()
    );
    let mut command = Command::new("script");
    if cfg!(target_os = "macos") {
        command.args(["-q", "/dev/null", "sh", "-c", &attach]);
    } else {
        command.args(["-qfc", &attach, "/dev/null"]);
    }
    let client = command
        .env("TERM", "xterm-256color")
        .env_remove("TMUX")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("script runs");
    let client = Client(client);
    wait_until("the client to attach", || {
        tmux.tmux(&["list-clients", "-F", "#{client_control_mode}"])
            .lines()
            .any(|mode| mode == "0")
    });
    client
}

fn attach_control_mode(tmux: &TmuxServer, session: &str) -> Client {
    let client = Command::new("tmux")
        .arg("-S")
        .arg(tmux.socket())
        .args(["-C", "attach-session", "-t", session])
        .env_remove("TMUX")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("tmux -C runs");
    let client = Client(client);
    wait_until("the control client to attach", || {
        !tmux.tmux(&["list-clients"]).is_empty()
    });
    client
}

fn count_bells(tmux: &TmuxServer) {
    tmux.tmux(&["set-hook", "-g", "alert-bell", "set -g @rang yes"]);
}

fn rang(tmux: &TmuxServer) -> bool {
    tmux.tmux(&["show-options", "-gqv", "@rang"]) == "yes"
}

#[test]
fn a_finished_task_rings_when_nobody_watches_its_window() {
    let project = Project::new();
    project.task("build", "#!/bin/sh\n");
    let tmux = TmuxServer::start();
    let session = tmux.new_session("app", &project.root);
    count_bells(&tmux);

    run(&tmux, &project, &session, &project.root, "build");

    wait_until("the bell", || rang(&tmux));
}

#[test]
fn a_control_mode_client_does_not_count_as_watching() {
    let project = Project::new();
    project.task("build", "#!/bin/sh\n");
    let tmux = TmuxServer::start();
    let session = tmux.new_session("app", &project.root);
    count_bells(&tmux);
    let _control = attach_control_mode(&tmux, &session);

    run(&tmux, &project, &session, &project.root, "build");

    wait_until("the bell", || rang(&tmux));
}

#[test]
fn a_finished_task_stays_silent_in_the_window_being_watched() {
    let project = Project::new();
    project.task("build", "#!/bin/sh\n");
    let tmux = TmuxServer::start();
    let session = tmux.new_session("app", &project.root);
    count_bells(&tmux);
    let _client = attach(&tmux, &session);

    run(&tmux, &project, &session, &project.root, "build");
    wait_until("the task to succeed", || {
        window_option(&tmux, &session, "build", "@task_status") == "ok"
    });
    std::thread::sleep(Duration::from_millis(300));

    assert!(!rang(&tmux), "the watched window rang");
}
