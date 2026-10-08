#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

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
        self.atelier_with_env(args, &[])
    }

    pub fn atelier_with_env(&self, args: &[&str], env: &[(&str, &str)]) -> Output {
        self.atelier_command(args)
            .envs(env.iter().copied())
            .output()
            .expect("atelier runs")
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

    pub fn atelier_stdout_with_env(&self, args: &[&str], env: &[(&str, &str)]) -> String {
        stdout_of(args, self.atelier_with_env(args, env))
    }

    pub fn attach_control_client(&self, session: &str) -> ControlClient {
        let mut client = ControlClient {
            name: String::new(),
            child: Command::new("tmux")
                .arg("-L")
                .arg(&self.name)
                .args(["-C", "attach-session", "-t", session])
                .env_remove("TMUX")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .spawn()
                .expect("tmux control client starts"),
        };
        let prefix = format!("{} ", client.child.id());
        for _ in 0..200 {
            let clients = self.tmux(&["list-clients", "-F", "#{client_pid} #{client_name}"]);
            if let Some(name) = clients.lines().find_map(|line| line.strip_prefix(&prefix)) {
                client.name = name.to_string();
                return client;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("control client for {session} never attached");
    }

    pub fn client_session(&self, client: &ControlClient) -> String {
        self.tmux(&["display", "-p", "-c", &client.name, "#{session_name}"])
    }
}

pub struct ControlClient {
    pub name: String,
    child: Child,
}

impl Drop for ControlClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
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
