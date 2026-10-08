mod common;

use std::path::{Path, PathBuf};

use common::{FakeOpener, TmuxServer};

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/pick/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn touch(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, "").unwrap();
}

fn wait_for(what: &str, mut done: impl FnMut() -> bool) {
    for _ in 0..200 {
        if done() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    panic!("timed out waiting for {what}");
}

fn idle_pane(tmux: &TmuxServer, dir: &Path) -> String {
    tmux.tmux(&[
        "new-session",
        "-d",
        "-c",
        dir.to_str().unwrap(),
        "-P",
        "-F",
        "#{pane_id}",
        "exec cat",
    ])
}

struct FakeBin {
    opener: FakeOpener,
}

impl FakeBin {
    fn new() -> Self {
        let opener = FakeOpener::new();
        std::fs::copy(which("cat"), opener.dir().join("nvim")).unwrap();
        FakeBin { opener }
    }

    fn nvim(&self) -> String {
        format!("exec '{}/nvim'", self.opener.dir().display())
    }

    fn path(&self) -> String {
        self.opener.path()
    }

    fn opened(&self) -> String {
        self.opener.opened()
    }
}

fn which(program: &str) -> PathBuf {
    std::env::var("PATH")
        .unwrap()
        .split(':')
        .map(|dir| Path::new(dir).join(program))
        .find(|path| path.is_file())
        .unwrap()
}

fn act(tmux: &TmuxServer, bin: &FakeBin, args: &[&str]) {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_atelier"))
        .arg("--socket")
        .arg(tmux.socket())
        .args(args)
        .env("PATH", bin.path())
        .env_remove("TMUX")
        .output()
        .unwrap();
    common::stdout_of(args, output);
}

fn display(tmux: &TmuxServer, target: &str, format: &str) -> String {
    tmux.tmux(&["display", "-p", "-t", target, format])
}

fn pane_showing(tmux: &TmuxServer, dir: &Path, file: &str) -> String {
    let session = tmux.tmux(&[
        "new-session",
        "-d",
        "-x",
        "200",
        "-y",
        "50",
        "-c",
        dir.to_str().unwrap(),
        "-P",
        "-F",
        "#{pane_id}",
        &format!("cat '{file}'; exec cat"),
    ]);
    wait_for("the capture to be printed", || {
        !tmux
            .tmux(&["capture-pane", "-p", "-t", &session])
            .trim()
            .is_empty()
    });
    session
}

fn list(tmux: &TmuxServer, pane: &str, home: &Path) -> Vec<String> {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_atelier"))
        .arg("--socket")
        .arg(tmux.socket())
        .args(["pick", "list", "-t", pane])
        .env("HOME", home)
        .env_remove("TMUX")
        .output()
        .unwrap();
    common::stdout_of(&["pick", "list"], output)
        .lines()
        .map(String::from)
        .collect()
}

#[test]
fn a_claude_code_capture_yields_its_urls_then_the_paths_that_exist() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("missions");
    touch(&project.join("core/use-cases/get-mission-progress.ts"));
    touch(&project.join("core/use-cases/get-mission-progress.test.ts"));
    let tmux = TmuxServer::start();
    let pane = pane_showing(&tmux, &project, &fixture("claude-code.txt"));

    assert_eq!(
        list(&tmux, &pane, dir.path()),
        [
            "https://github.com/acme/missions/pull/42",
            "https://docs.acme.dev/progress",
            "core/use-cases/get-mission-progress.ts",
            "core/use-cases/get-mission-progress.test.ts",
        ]
    );
}

#[test]
fn a_shell_capture_resolves_relative_and_home_paths_from_the_pane() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("missions");
    let home = dir.path().join("home");
    touch(&project.join("src/main.rs"));
    touch(&project.join("README.md"));
    touch(&dir.path().join("outside/ref.txt"));
    touch(&home.join("notes/todo.md"));
    let tmux = TmuxServer::start();
    let pane = pane_showing(&tmux, &project, &fixture("shell.txt"));

    assert_eq!(
        list(&tmux, &pane, &home),
        [
            "https://github.com/acme/missions/pull/42",
            "ftp://mirror.acme.dev/pub/archive.tgz",
            "git@github.com:acme/missions.git",
            "~/notes/todo.md",
            "./README.md",
            "../outside/ref.txt",
            "src/main.rs",
        ]
    );
}

#[test]
fn a_url_wrapped_across_the_pane_width_comes_back_whole() {
    let dir = tempfile::tempdir().unwrap();
    let url = "https://github.com/acme/missions/blob/main/core/use-cases/get-mission-progress.ts";
    let file = dir.path().join("capture.txt");
    std::fs::write(&file, format!("see {url}\n")).unwrap();
    let tmux = TmuxServer::start();
    let pane = pane_showing(&tmux, dir.path(), file.to_str().unwrap());
    tmux.tmux(&["resize-window", "-t", &pane, "-x", "30"]);

    assert_eq!(list(&tmux, &pane, dir.path()), [url]);
}

#[test]
fn a_pane_with_nothing_to_pick_lists_a_placeholder_row() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("capture.txt");
    std::fs::write(&file, "nothing/here.txt and no link\n").unwrap();
    let tmux = TmuxServer::start();
    let pane = pane_showing(&tmux, dir.path(), file.to_str().unwrap());

    assert_eq!(
        list(&tmux, &pane, dir.path()),
        ["(no URL or existing path on this pane)"]
    );
}

#[test]
fn an_unexpanded_pane_format_falls_back_to_the_current_pane() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("capture.txt");
    std::fs::write(&file, "https://acme.dev/a\n").unwrap();
    let tmux = TmuxServer::start();
    pane_showing(&tmux, dir.path(), file.to_str().unwrap());

    assert_eq!(
        list(&tmux, "#{pane_id}", dir.path()),
        ["https://acme.dev/a"]
    );
}

#[test]
fn a_full_scrollback_is_listed_without_a_process_per_line() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("src/lib.rs"));
    let capture: String = (0..2000)
        .map(|i| format!("{i} src/lib.rs:{i}:1 https://acme.dev/build/{i} gone/{i}.rs\n"))
        .collect();
    let file = dir.path().join("capture.txt");
    std::fs::write(&file, capture).unwrap();
    let tmux = TmuxServer::start();
    let pane = pane_showing(&tmux, dir.path(), file.to_str().unwrap());
    wait_for("the scrollback to fill", || {
        tmux.tmux(&["display", "-p", "-t", &pane, "#{history_size}"]) != "0"
            && tmux
                .tmux(&["capture-pane", "-p", "-t", &pane])
                .contains("1999 src/lib.rs")
    });

    let started = std::time::Instant::now();
    let rows = list(&tmux, &pane, dir.path());

    assert!(started.elapsed() < std::time::Duration::from_secs(2));
    assert_eq!(rows.first().unwrap(), "https://acme.dev/build/1999");
    assert_eq!(rows.len(), 200);
}

#[test]
fn opening_a_url_hands_it_to_the_browser() {
    let dir = tempfile::tempdir().unwrap();
    let bin = FakeBin::new();
    let tmux = TmuxServer::start();
    let pane = idle_pane(&tmux, dir.path());

    act(
        &tmux,
        &bin,
        &["pick", "open", "-t", &pane, "https://acme.dev/a"],
    );

    assert_eq!(bin.opener.wait_opened(), "https://acme.dev/a\n");
}

#[test]
fn opening_a_file_edits_it_in_the_nvim_of_this_session() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("src/my lib.rs"));
    let bin = FakeBin::new();
    let tmux = TmuxServer::start();
    let pane = idle_pane(&tmux, dir.path());
    let nvim = tmux.tmux(&[
        "new-window",
        "-d",
        "-t",
        &format!("{}:", display(&tmux, &pane, "#{session_id}")),
        "-P",
        "-F",
        "#{pane_id}",
        &bin.nvim(),
    ]);
    wait_for("nvim to start", || {
        display(&tmux, &nvim, "#{pane_current_command}") == "nvim"
    });

    act(&tmux, &bin, &["pick", "open", "-t", &pane, "src/my lib.rs"]);

    let typed = format!(":e {}/src/my\\ lib.rs", dir.path().display());
    wait_for("nvim to receive :e", || {
        tmux.tmux(&["capture-pane", "-p", "-t", &nvim])
            .contains(&typed)
    });
    assert_eq!(display(&tmux, &pane, "#{pane_id}"), pane);
    assert_eq!(display(&tmux, &pane, "#{window_active}"), "0");
    assert_eq!(display(&tmux, &nvim, "#{window_active}"), "1");
}

#[test]
fn opening_a_file_without_nvim_opens_a_new_nvim_window_at_the_session_root() {
    let dir = tempfile::tempdir().unwrap();
    let deep = dir.path().join("src/handlers");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(deep.join("auth.rs"), "fn auth() {}\n").unwrap();
    let bin = FakeBin::new();
    let tmux = TmuxServer::start();
    tmux.tmux(&["set", "-g", "remain-on-exit", "on"]);
    let pane = idle_pane(&tmux, dir.path());
    let deep_path = deep.to_str().unwrap();
    tmux.tmux(&[
        "respawn-pane",
        "-k",
        "-t",
        &pane,
        "-c",
        deep_path,
        "exec cat",
    ]);

    act(&tmux, &bin, &["pick", "open", "-t", &pane, "auth.rs"]);

    let session = display(&tmux, &pane, "#{session_id}");
    let window = display(&tmux, &session, "#{window_id}");
    assert_ne!(window, display(&tmux, &pane, "#{window_id}"));
    assert_eq!(display(&tmux, &window, "#{window_name}"), "nvim");
    assert_eq!(
        display(&tmux, &window, "#{pane_start_path}"),
        dir.path().to_str().unwrap()
    );
    assert_eq!(
        display(&tmux, &window, "#{pane_start_command}"),
        format!("nvim {}", deep.join("auth.rs").display())
    );
}

#[test]
fn an_nvim_in_another_session_is_left_alone() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("lib.rs"));
    let bin = FakeBin::new();
    let tmux = TmuxServer::start();
    tmux.tmux(&["set", "-g", "remain-on-exit", "on"]);
    let other = tmux.tmux(&["new-session", "-d", "-P", "-F", "#{pane_id}", &bin.nvim()]);
    let pane = idle_pane(&tmux, dir.path());
    wait_for("nvim to start", || {
        display(&tmux, &other, "#{pane_current_command}") == "nvim"
    });

    act(&tmux, &bin, &["pick", "open", "-t", &pane, "lib.rs"]);

    assert!(!tmux
        .tmux(&["capture-pane", "-p", "-t", &other])
        .contains(":e"));
    let session = display(&tmux, &pane, "#{session_id}");
    assert_eq!(display(&tmux, &session, "#{window_name}"), "nvim");
}

#[test]
fn opening_a_directory_hands_it_to_the_os() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src/handlers")).unwrap();
    let bin = FakeBin::new();
    let tmux = TmuxServer::start();
    let pane = idle_pane(&tmux, dir.path());

    act(&tmux, &bin, &["pick", "open", "-t", &pane, "src/handlers"]);

    assert_eq!(
        bin.opener.wait_opened(),
        format!("{}/src/handlers\n", dir.path().display())
    );
}

#[test]
fn handing_a_relative_path_to_the_os_resolves_it_from_the_pane() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("docs/plan.md"));
    let bin = FakeBin::new();
    let tmux = TmuxServer::start();
    let pane = idle_pane(&tmux, dir.path());

    act(
        &tmux,
        &bin,
        &["pick", "system", "-t", &pane, "docs/plan.md"],
    );

    assert_eq!(
        bin.opener.wait_opened(),
        format!("{}/docs/plan.md\n", dir.path().display())
    );
}

#[test]
fn copying_puts_the_token_in_a_tmux_buffer() {
    let bin = FakeBin::new();
    let tmux = TmuxServer::start();

    act(&tmux, &bin, &["pick", "copy", "https://acme.dev/a b"]);

    assert_eq!(tmux.tmux(&["show-buffer"]), "https://acme.dev/a b");
}

#[test]
fn copying_reaches_the_terminal_of_the_attached_client_through_osc_52() {
    let dir = tempfile::tempdir().unwrap();
    let bin = FakeBin::new();
    let remote = TmuxServer::start();
    remote.tmux(&["set", "-as", "terminal-features", ",*:clipboard"]);
    let session = idle_pane(&remote, dir.path());
    let terminal = TmuxServer::start();
    terminal.tmux(&["set", "-g", "set-clipboard", "on"]);
    terminal.tmux(&[
        "new-session",
        "-d",
        "--",
        "env",
        "-u",
        "TMUX",
        "tmux",
        "-S",
        remote.socket().to_str().unwrap(),
        "attach",
        "-t",
        &session,
    ]);
    wait_for("the client to attach", || {
        !remote.tmux(&["list-clients"]).is_empty()
    });

    act(&remote, &bin, &["pick", "copy", "https://acme.dev/a"]);

    wait_for("the terminal to receive the clipboard", || {
        !terminal.tmux(&["list-buffers"]).is_empty()
    });
    assert_eq!(terminal.tmux(&["show-buffer"]), "https://acme.dev/a");
}

#[test]
fn every_action_on_the_placeholder_row_does_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let bin = FakeBin::new();
    let tmux = TmuxServer::start();
    let pane = idle_pane(&tmux, dir.path());
    let placeholder = "(no URL or existing path on this pane)";

    act(&tmux, &bin, &["pick", "open", "-t", &pane, placeholder]);
    act(&tmux, &bin, &["pick", "system", "-t", &pane, placeholder]);
    act(&tmux, &bin, &["pick", "copy", placeholder]);

    assert_eq!(bin.opened(), "");
    assert_eq!(tmux.tmux(&["list-buffers"]), "");
    assert_eq!(tmux.tmux(&["list-windows", "-a"]).lines().count(), 1);
}

fn picker_on(tmux: &TmuxServer, bin: &FakeBin, pane: &str) -> Option<String> {
    if !common::fzf_available() {
        return None;
    }
    let picker = tmux.tmux(&[
        "new-window",
        "-d",
        "-t",
        &format!("{}:", display(tmux, pane, "#{session_id}")),
        "-P",
        "-F",
        "#{pane_id}",
        "--",
        "env",
        &format!("PATH={}", bin.path()),
        env!("CARGO_BIN_EXE_atelier"),
        "pick",
        "popup",
        "-t",
        pane,
    ]);
    wait_for("the picker to list the rows", || {
        tmux.tmux(&["capture-pane", "-p", "-t", &picker])
            .contains("https://acme.dev/b")
    });
    Some(picker)
}

#[test]
fn enter_in_the_picker_opens_the_newest_token() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("capture.txt");
    std::fs::write(&file, "https://acme.dev/a\nhttps://acme.dev/b\n").unwrap();
    let bin = FakeBin::new();
    let tmux = TmuxServer::start();
    let pane = pane_showing(&tmux, dir.path(), file.to_str().unwrap());
    let Some(picker) = picker_on(&tmux, &bin, &pane) else {
        return;
    };

    tmux.tmux(&["send-keys", "-t", &picker, "Enter"]);

    wait_for("the browser to be called", || !bin.opened().is_empty());
    assert_eq!(bin.opened(), "https://acme.dev/b\n");
}

#[test]
fn ctrl_y_in_the_picker_copies_the_selected_token() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("capture.txt");
    std::fs::write(&file, "https://acme.dev/a\nhttps://acme.dev/b\n").unwrap();
    let bin = FakeBin::new();
    let tmux = TmuxServer::start();
    let pane = pane_showing(&tmux, dir.path(), file.to_str().unwrap());
    let Some(picker) = picker_on(&tmux, &bin, &pane) else {
        return;
    };

    tmux.tmux(&["send-keys", "-t", &picker, "j", "C-y"]);

    wait_for("the buffer to be set", || {
        !tmux.tmux(&["list-buffers"]).is_empty()
    });
    assert_eq!(tmux.tmux(&["show-buffer"]), "https://acme.dev/a");
}

fn preview(tmux: &TmuxServer, port: u16, home: &Path, pane: &str, token: &str) -> String {
    let args = ["pick", "preview", "-t", pane, token];
    let opener = FakeOpener::new();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_atelier"))
        .env("PATH", opener.path())
        .arg("--socket")
        .arg(tmux.socket())
        .args(args)
        .env("HOME", home)
        .env("MD_PREVIEW_PORT", port.to_string())
        .env_remove("TMUX")
        .output()
        .unwrap();
    common::stdout_of(&args, output)
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

struct PreviewServer(std::process::Child);

impl Drop for PreviewServer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn preview_server(port: u16, home: &Path) -> PreviewServer {
    let server = PreviewServer(
        std::process::Command::new(env!("CARGO_BIN_EXE_atelier"))
            .args(["preview", "serve"])
            .env("HOME", home)
            .env("MD_PREVIEW_PORT", port.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    );
    wait_for("the preview server", || {
        std::net::TcpStream::connect(("127.0.0.1", port)).is_ok()
    });
    server
}

#[test]
fn previewing_a_markdown_path_opens_it_from_the_pane_directory() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    touch(&root.join("docs/plan.md"));
    let port = free_port();
    let _server = preview_server(port, &root);
    let tmux = TmuxServer::start();
    let pane = idle_pane(&tmux, &root);

    assert_eq!(
        preview(&tmux, port, &root, &pane, "docs/plan.md"),
        format!(
            "http://127.0.0.1:{port}{}",
            root.join("docs/plan.md").display()
        )
    );
}

#[test]
fn previewing_anything_but_a_markdown_path_does_nothing() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("src/main.rs"));
    let port = free_port();
    let tmux = TmuxServer::start();
    let pane = idle_pane(&tmux, dir.path());

    for token in [
        "https://acme.dev/plan.md",
        "src/main.rs",
        "(no URL or existing path on this pane)",
    ] {
        assert_eq!(preview(&tmux, port, dir.path(), &pane, token), "");
    }
    assert!(std::net::TcpStream::connect(("127.0.0.1", port)).is_err());
}
