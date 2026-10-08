use std::path::PathBuf;
use std::process::Command;

use crate::Result;

pub struct Tmux {
    socket: Option<PathBuf>,
}

impl Tmux {
    pub fn new(socket: Option<PathBuf>) -> Self {
        Tmux {
            socket: socket.or_else(socket_from_env),
        }
    }

    pub fn run(&self, args: &[&str]) -> Result<String> {
        let mut command = Command::new("tmux");
        if let Some(socket) = &self.socket {
            command.arg("-S").arg(socket);
        }
        let output = command.args(args).output()?;
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

    pub fn display(&self, target: &str, format: &str) -> Result<String> {
        self.run(&["display-message", "-p", "-t", target, format])
    }
}

fn socket_from_env() -> Option<PathBuf> {
    let tmux = std::env::var_os("TMUX")?;
    let socket = tmux.to_str()?.split(',').next()?;
    (!socket.is_empty()).then(|| PathBuf::from(socket))
}
