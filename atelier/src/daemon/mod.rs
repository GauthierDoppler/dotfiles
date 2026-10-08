mod control;
mod server;
mod state;

use std::fs::{DirBuilder, File, OpenOptions, TryLockError};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::tmux::Tmux;
use crate::Result;

use state::{Session, State};

#[derive(Subcommand)]
pub enum Command {
    /// Keep a live view of this tmux server's sessions and windows
    Daemon {
        /// Start the daemon in the background unless one already runs
        #[arg(long)]
        ensure: bool,
    },
    /// Print the sessions and windows the daemon sees
    Status,
}

impl Command {
    pub fn run(self, tmux: &Tmux) -> Result<()> {
        match self {
            Command::Daemon { ensure: true } => ensure(tmux),
            Command::Daemon { ensure: false } => {
                let socket = tmux.socket_path()?;
                server::run(&socket, &Paths::for_socket(&socket)?)
            }
            Command::Status => {
                let answer = tmux
                    .socket_path()
                    .and_then(|socket| Paths::for_socket(&socket))
                    .ok()
                    .and_then(|paths| ask_status(&paths));
                let text = match answer {
                    Some(reply) => state::render(Some(reply.pid), &reply.sessions),
                    None => state::render(None, &State::from_tmux(tmux)?.snapshot()),
                };
                print!("{text}");
                Ok(())
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "kebab-case")]
enum Request {
    Status,
}

#[derive(Serialize, Deserialize)]
struct StatusReply {
    pid: u32,
    sessions: Vec<Session>,
}

struct Paths {
    lock: PathBuf,
    socket: PathBuf,
    log: PathBuf,
}

impl Paths {
    fn for_socket(tmux_socket: &Path) -> Result<Paths> {
        let dir = match std::env::var_os("XDG_RUNTIME_DIR").filter(|d| !d.is_empty()) {
            Some(runtime) => PathBuf::from(runtime).join("atelier"),
            None => std::env::temp_dir().join(format!(
                "atelier-{}",
                std::fs::metadata(tmux_socket)?.uid()
            )),
        };
        DirBuilder::new().recursive(true).mode(0o700).create(&dir)?;
        let key = format!("{:016x}", fnv1a(tmux_socket.as_os_str().as_encoded_bytes()));
        Ok(Paths {
            lock: dir.join(format!("{key}.lock")),
            socket: dir.join(format!("{key}.sock")),
            log: dir.join(format!("{key}.log")),
        })
    }

    fn lock(&self) -> Result<Option<File>> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&self.lock)?;
        match file.try_lock() {
            Ok(()) => Ok(Some(file)),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Error(error)) => Err(error.into()),
        }
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

fn ensure(tmux: &Tmux) -> Result<()> {
    let socket = tmux.socket_path()?;
    let paths = Paths::for_socket(&socket)?;
    if paths.lock()?.is_none() {
        return Ok(());
    }
    let log = File::create(&paths.log)?;
    std::process::Command::new(std::env::current_exe()?)
        .arg("--socket")
        .arg(&socket)
        .arg("daemon")
        .env_remove("TMUX")
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        .process_group(0)
        .spawn()?;
    Ok(())
}

fn ask_status(paths: &Paths) -> Option<StatusReply> {
    let mut stream = UnixStream::connect(&paths.socket).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
    let mut request = serde_json::to_string(&Request::Status).ok()?;
    request.push('\n');
    stream.write_all(request.as_bytes()).ok()?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).ok()?;
    serde_json::from_str(&line).ok()
}
