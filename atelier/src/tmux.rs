use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Result;

const FALLBACK_DIRS: &[&str] = &[
    "/opt/homebrew/bin",
    "/usr/local/bin",
    "/home/linuxbrew/.linuxbrew/bin",
    "/usr/bin",
    "/bin",
];

pub struct Tmux {
    socket: Option<PathBuf>,
    program: PathBuf,
}

impl Tmux {
    pub fn new(socket: Option<PathBuf>) -> Self {
        Tmux {
            socket: socket.or_else(socket_from_env),
            program: program(),
        }
    }

    pub fn socket(&self) -> Option<&Path> {
        self.socket.as_deref()
    }

    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        if let Some(socket) = &self.socket {
            command.arg("-S").arg(socket);
        }
        command
    }

    pub fn run(&self, args: &[&str]) -> Result<String> {
        let output = self.command().args(args).output()?;
        if !output.status.success() {
            return Err(format!(
                "tmux {}: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            )
            .into());
        }
        let mut stdout = String::from_utf8(output.stdout)?;
        if stdout.ends_with('\n') {
            stdout.pop();
        }
        Ok(stdout)
    }

    pub fn socket_path(&self) -> Result<PathBuf> {
        match &self.socket {
            Some(socket) => Ok(socket.clone()),
            None => Ok(PathBuf::from(self.run(&[
                "display-message",
                "-p",
                "#{socket_path}",
            ])?)),
        }
    }

    pub fn version(&self) -> Option<String> {
        crate::process::stdout_of(&self.program, &["-V"])
    }

    pub fn display(&self, target: &str, format: &str) -> Result<String> {
        self.run(&["display-message", "-p", "-t", target, format])
    }
}

fn program() -> PathBuf {
    let on_path = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    on_path
        .into_iter()
        .chain(FALLBACK_DIRS.iter().map(PathBuf::from))
        .map(|dir| dir.join("tmux"))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| PathBuf::from("tmux"))
}

fn socket_from_env() -> Option<PathBuf> {
    let tmux = std::env::var_os("TMUX")?;
    let socket = tmux.to_str()?.split(',').next()?;
    (!socket.is_empty()).then(|| PathBuf::from(socket))
}
