#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

pub struct TmuxServer {
    name: String,
    socket: PathBuf,
    runtime: tempfile::TempDir,
}

impl TmuxServer {
    pub fn start() -> Self {
        let name = format!(
            "atelier-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let mut server = TmuxServer {
            name,
            socket: PathBuf::new(),
            runtime: tempfile::tempdir().expect("runtime dir created"),
        };
        server.tmux(&[
            "-f",
            "/dev/null",
            "start-server",
            ";",
            "set",
            "-g",
            "exit-empty",
            "off",
            ";",
            "set",
            "-g",
            "default-shell",
            "/bin/sh",
        ]);
        server.socket = PathBuf::from(server.tmux(&["display", "-p", "#{socket_path}"]));
        server
    }

    pub fn socket(&self) -> &Path {
        &self.socket
    }

    pub fn tmux(&self, args: &[&str]) -> String {
        let output = Command::new("tmux")
            .arg("-L")
            .arg(&self.name)
            .args(args)
            .env_remove("TMUX")
            .env("XDG_RUNTIME_DIR", self.runtime.path())
            .output()
            .expect("tmux runs");
        assert!(
            output.status.success(),
            "tmux {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .expect("tmux prints utf-8")
            .trim_end_matches('\n')
            .to_string()
    }

    pub fn new_session(&self, name: &str, dir: &Path) -> String {
        self.tmux(&[
            "new-session",
            "-d",
            "-s",
            name,
            "-c",
            dir.to_str().expect("utf-8 path"),
            "-P",
            "-F",
            "#{session_id}",
        ])
    }

    pub fn set_session_option(&self, session: &str, option: &str, value: &str) {
        self.tmux(&["set-option", "-t", session, option, value]);
    }

    pub fn atelier_command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_atelier"));
        command
            .arg("--socket")
            .arg(&self.socket)
            .args(args)
            .env_remove("TMUX")
            .env("XDG_RUNTIME_DIR", self.runtime.path());
        command
    }

    pub fn atelier(&self, args: &[&str]) -> Output {
        self.atelier_command(args).output().expect("atelier runs")
    }

    pub fn atelier_inside(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_atelier"))
            .args(args)
            .env("TMUX", format!("{},1,0", self.socket.display()))
            .env("XDG_RUNTIME_DIR", self.runtime.path())
            .output()
            .expect("atelier runs")
    }

    pub fn atelier_stdout(&self, args: &[&str]) -> String {
        stdout_of(args, self.atelier(args))
    }
}

pub fn stdout_of(args: &[&str], output: Output) -> String {
    assert!(
        output.status.success(),
        "atelier {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("atelier prints utf-8")
        .trim_end_matches('\n')
        .to_string()
}

impl Drop for TmuxServer {
    fn drop(&mut self) {
        let _ = Command::new("tmux")
            .arg("-L")
            .arg(&self.name)
            .arg("kill-server")
            .env_remove("TMUX")
            .output();
    }
}

pub fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=atelier",
            "-c",
            "user.email=atelier@example.com",
        ])
        .args(["-c", "init.defaultBranch=main"])
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn git_repo(dir: &Path) {
    std::fs::create_dir_all(dir).expect("repo dir created");
    git(dir, &["init", "-q"]);
    git(dir, &["commit", "-q", "--allow-empty", "-m", "init"]);
}
