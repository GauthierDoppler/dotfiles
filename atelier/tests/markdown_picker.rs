mod common;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, SystemTime};

use common::{git, git_repo, TmuxServer};

const PLACEHOLDER: &str = "(no markdown file under this session)";

fn real_tempdir() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().canonicalize().unwrap();
    (dir, path)
}

fn write_aged(path: &Path, minutes_ago: u64) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, "# hi\n").unwrap();
    let file = std::fs::File::options().write(true).open(path).unwrap();
    file.set_modified(SystemTime::now() - Duration::from_secs(60 * minutes_ago))
        .unwrap();
}

fn rows(tmux: &TmuxServer, session: &str) -> Vec<String> {
    tmux.atelier_stdout(&["preview", "list", "-t", session])
        .lines()
        .map(String::from)
        .collect()
}

#[test]
fn a_repo_lists_its_markdown_files_newest_first() {
    let (_dir, root) = real_tempdir();
    git_repo(&root);
    write_aged(&root.join("README.md"), 30);
    write_aged(&root.join("docs/plan.md"), 1);
    write_aged(&root.join("docs/spec.markdown"), 10);
    write_aged(&root.join("src/main.rs"), 0);
    git(&root, &["add", "README.md"]);
    let tmux = TmuxServer::start();
    let session = tmux.new_session("repo", &root);

    assert_eq!(
        rows(&tmux, &session),
        ["docs/plan.md", "docs/spec.markdown", "README.md"]
    );
}

#[test]
fn gitignored_markdown_is_left_out() {
    let (_dir, root) = real_tempdir();
    git_repo(&root);
    std::fs::write(root.join(".gitignore"), "scratch/\nnode_modules/\n").unwrap();
    write_aged(&root.join("scratch/draft.md"), 0);
    write_aged(&root.join("node_modules/pkg/README.md"), 0);
    write_aged(&root.join("plan.md"), 5);
    let tmux = TmuxServer::start();
    let session = tmux.new_session("repo", &root);

    assert_eq!(rows(&tmux, &session), ["plan.md"]);
}

#[test]
fn a_worktree_session_lists_its_own_checkout() {
    let (_dir, root) = real_tempdir();
    let main = root.join("main");
    git_repo(&main);
    write_aged(&main.join("main.md"), 0);
    git(&main, &["add", "main.md"]);
    git(&main, &["commit", "-q", "-m", "main"]);
    let worktree = root.join("feature");
    git(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "feature",
            worktree.to_str().unwrap(),
        ],
    );
    write_aged(&worktree.join("feature.md"), 0);
    let tmux = TmuxServer::start();
    let session = tmux.new_session("feature", &worktree);

    assert_eq!(rows(&tmux, &session), ["feature.md", "main.md"]);
}

#[test]
fn outside_a_repo_the_directory_is_walked_without_hidden_folders() {
    let (_dir, root) = real_tempdir();
    write_aged(&root.join("notes/today.md"), 2);
    write_aged(&root.join("todo.md"), 4);
    write_aged(&root.join(".cache/log.md"), 0);
    write_aged(&root.join("notes/today.txt"), 0);
    let tmux = TmuxServer::start();
    let session = tmux.new_session("loose", &root);

    assert_eq!(rows(&tmux, &session), ["notes/today.md", "todo.md"]);
}

#[test]
fn a_session_without_markdown_lists_a_placeholder_row() {
    let (_dir, root) = real_tempdir();
    git_repo(&root);
    std::fs::write(root.join("main.rs"), "").unwrap();
    let tmux = TmuxServer::start();
    let session = tmux.new_session("empty", &root);

    assert_eq!(rows(&tmux, &session), [PLACEHOLDER]);
}

#[test]
fn without_a_target_the_current_session_is_listed() {
    let (_dir, root) = real_tempdir();
    write_aged(&root.join("plan.md"), 0);
    let tmux = TmuxServer::start();
    tmux.new_session("only", &root);

    let output = tmux.atelier_inside(&["preview", "list"]);

    assert_eq!(common::stdout_of(&["preview", "list"], output), "plan.md");
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn answers(port: u16) -> bool {
    let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) else {
        return false;
    };
    let _ = write!(
        stream,
        "GET /__meta HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    );
    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);
    response.contains("md-preview")
}

fn wait_for(what: &str, mut done: impl FnMut() -> bool) {
    for _ in 0..400 {
        if done() {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("timed out waiting for {what}");
}

struct Server {
    port: u16,
    _home: tempfile::TempDir,
    child: Child,
}

impl Server {
    fn start() -> Self {
        let port = free_port();
        let home = tempfile::tempdir().unwrap();
        let child = Command::new(env!("CARGO_BIN_EXE_atelier"))
            .args(["preview", "serve"])
            .env("HOME", home.path())
            .env("MD_PREVIEW_PORT", port.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        wait_for("the preview server", || answers(port));
        Server {
            port,
            _home: home,
            child,
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn previewing_the_placeholder_row_does_nothing() {
    let (_dir, root) = real_tempdir();
    let port = free_port();

    let output = Command::new(env!("CARGO_BIN_EXE_atelier"))
        .args(["preview", PLACEHOLDER])
        .current_dir(&root)
        .env("HOME", &root)
        .env("MD_PREVIEW_PORT", port.to_string())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert!(!answers(port));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn enter_in_the_picker_previews_the_newest_file() {
    if !common::fzf_available() {
        return;
    }
    let (_dir, root) = real_tempdir();
    write_aged(&root.join("old.md"), 10);
    write_aged(&root.join("docs/new.md"), 0);
    let server = Server::start();
    let tmux = TmuxServer::start();
    let session = tmux.new_session("docs", &root);
    let printed = root.join("printed");
    let picker = tmux.tmux(&[
        "new-window",
        "-d",
        "-t",
        &format!("{session}:"),
        "-c",
        "/",
        "-P",
        "-F",
        "#{pane_id}",
        &format!(
            "MD_PREVIEW_PORT={} '{}' --socket '{}' preview pick -t '{session}' > '{}'",
            server.port,
            env!("CARGO_BIN_EXE_atelier"),
            tmux.socket().display(),
            printed.display()
        ),
    ]);
    wait_for("the picker to list the rows", || {
        tmux.tmux(&["capture-pane", "-p", "-t", &picker])
            .contains("old.md")
    });

    tmux.tmux(&["send-keys", "-t", &picker, "Enter"]);

    wait_for("the preview URL", || {
        std::fs::read_to_string(&printed).is_ok_and(|out| out.ends_with('\n'))
    });
    assert_eq!(
        std::fs::read_to_string(&printed).unwrap(),
        format!(
            "http://127.0.0.1:{}{}\n",
            server.port,
            root.join("docs/new.md").display()
        )
    );
}
