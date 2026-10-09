#![allow(dead_code)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

static NEXT: AtomicUsize = AtomicUsize::new(0);

pub const DEADLINE: Duration = Duration::from_secs(30);

pub trait BoundedOutput {
    fn bounded_output(&mut self) -> std::io::Result<Output>;
}

impl BoundedOutput for Command {
    fn bounded_output(&mut self) -> std::io::Result<Output> {
        let child = self
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        Ok(wait_bounded(child, &format!("{self:?}")))
    }
}

pub fn wait_bounded(mut child: Child, what: &str) -> Output {
    let (sender, received) = mpsc::channel();
    let streams: [Option<Box<dyn Read + Send>>; 2] = [
        child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>),
        child
            .stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>),
    ];
    let mut readers = 0;
    for (index, stream) in streams.into_iter().enumerate() {
        if let Some(mut stream) = stream {
            readers += 1;
            let sender = sender.clone();
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                let _ = stream.read_to_end(&mut bytes);
                let _ = sender.send((index, bytes));
            });
        }
    }
    let deadline = Instant::now() + DEADLINE;
    let mut pause = Duration::from_millis(1);
    let status = loop {
        if let Some(status) = child.try_wait().expect("child can be polled") {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{what} was still running after {DEADLINE:?}, killed");
        }
        std::thread::sleep(pause);
        pause = (pause * 2).min(Duration::from_millis(20));
    };
    let mut output = Output {
        status,
        stdout: Vec::new(),
        stderr: Vec::new(),
    };
    for _ in 0..readers {
        let left = deadline.saturating_duration_since(Instant::now());
        let (index, bytes) = received
            .recv_timeout(left.max(Duration::from_secs(1)))
            .unwrap_or_else(|_| {
                panic!("{what} exited, but a process it started still holds its output open")
            });
        if index == 0 {
            output.stdout = bytes;
        } else {
            output.stderr = bytes;
        }
    }
    output
}

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
            .bounded_output()
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
            .bounded_output()
            .expect("atelier runs")
    }

    pub fn atelier_inside(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_atelier"))
            .args(args)
            .env("TMUX", format!("{},1,0", self.socket.display()))
            .env("XDG_RUNTIME_DIR", self.runtime.path())
            .bounded_output()
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

    pub fn attach_terminal_client(&self, session: &str) -> TerminalClient {
        let attach = format!(
            "tmux -S '{}' attach-session -t '{session}'",
            self.socket.display()
        );
        let mut command = Command::new("script");
        if cfg!(target_os = "macos") {
            command.args(["-q", "/dev/null", "sh", "-c", &attach]);
        } else {
            command.args(["-qfc", &attach, "/dev/null"]);
        }
        let client = TerminalClient(
            command
                .env("TERM", "xterm-256color")
                .env_remove("TMUX")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("script runs"),
        );
        for _ in 0..200 {
            let modes = self.tmux(&["list-clients", "-F", "#{client_control_mode}"]);
            if modes.lines().any(|mode| mode == "0") {
                return client;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("terminal client for {session} never attached");
    }
}

pub struct TerminalClient(Child);

impl Drop for TerminalClient {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
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
        .bounded_output()
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

pub struct FakeOpener {
    dir: tempfile::TempDir,
}

impl FakeOpener {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("opener dir created");
        let log = dir.path().join("opened");
        let logs = format!("#!/bin/sh\nprintf '%s\\n' \"$@\" >> '{}'\n", log.display());
        for (program, body) in [
            ("open", logs.as_str()),
            ("xdg-open", logs.as_str()),
            ("osascript", "#!/bin/sh\necho none\n"),
        ] {
            let script = dir.path().join(program);
            std::fs::write(&script, body).expect("opener written");
            std::fs::set_permissions(&script, std::os::unix::fs::PermissionsExt::from_mode(0o755))
                .expect("opener made executable");
        }
        FakeOpener { dir }
    }

    pub fn dir(&self) -> &Path {
        self.dir.path()
    }

    pub fn path(&self) -> String {
        format!(
            "{}:{}",
            self.dir.path().display(),
            std::env::var("PATH").expect("PATH is set")
        )
    }

    pub fn opened(&self) -> String {
        std::fs::read_to_string(self.dir.path().join("opened")).unwrap_or_default()
    }

    pub fn browser_call(url: &str) -> String {
        if cfg!(target_os = "macos") {
            format!("-a\nGoogle Chrome\n{url}\n")
        } else {
            format!("{url}\n")
        }
    }

    pub fn wait_opened(&self) -> String {
        for _ in 0..200 {
            let opened = self.opened();
            if opened.ends_with('\n') {
                return opened;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        panic!("the opener was never called");
    }
}

pub fn fzf_available() -> bool {
    let version = Command::new("fzf")
        .arg("--version")
        .bounded_output()
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default();
    let mut numbers = version
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .split('.')
        .map(|part| part.parse::<u32>().unwrap_or(0));
    let recent = (numbers.next().unwrap_or(0), numbers.next().unwrap_or(0)) >= (0, 45);
    if !recent {
        assert!(
            std::env::var_os("CI").is_none(),
            "CI must install fzf >= 0.45, found {version:?}"
        );
        eprintln!("fzf >= 0.45 is not installed: skipping");
    }
    recent
}

pub fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crate sits inside the dotfiles repo")
        .to_path_buf()
}

pub fn repo_with_fixture_services() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir created");
    for entry in std::fs::read_dir(repo()).expect("the repo is readable") {
        let entry = entry.expect("a repo entry");
        let name = entry.file_name();
        if name != "services.toml" && name != ".git" {
            std::os::unix::fs::symlink(entry.path(), dir.path().join(&name))
                .expect("repo entry linked");
        }
    }
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/services.toml"),
        dir.path().join("services.toml"),
    )
    .expect("fixture services.toml copied");
    dir
}

pub fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .expect("a free port")
        .local_addr()
        .expect("a bound address")
        .port()
}

pub fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(25));
    }
}

pub fn real_tempdir() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir created");
    let real = dir.path().canonicalize().expect("temp dir resolves");
    (dir, real)
}
