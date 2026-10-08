use std::collections::VecDeque;
use std::path::Path;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

use super::control::{Event, Notification, Parser};
use super::state::{Query, State};
use super::{Paths, Request, StatusReply};
use crate::tmux::Tmux;
use crate::Result;

type Shared = Arc<Mutex<State>>;

pub fn run(tmux_socket: &Path, paths: &Paths) -> Result<()> {
    let Some(_lock) = paths.lock()? else {
        return Ok(());
    };
    let _ = std::fs::remove_file(&paths.socket);
    let listener = std::os::unix::net::UnixListener::bind(&paths.socket)?;
    listener.set_nonblocking(true)?;
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(serve(tmux_socket, listener));
    let _ = std::fs::remove_file(&paths.socket);
    result
}

async fn serve(tmux_socket: &Path, listener: std::os::unix::net::UnixListener) -> Result<()> {
    let listener = UnixListener::from_std(listener)?;
    let state = Shared::default();
    let tmux = Tmux::new(Some(tmux_socket.to_path_buf()));
    while follow(tmux_socket, &listener, &state).await?
        && tmux
            .run(&["list-sessions", "-F", "x"])
            .is_ok_and(|sessions| !sessions.is_empty())
    {}
    Ok(())
}

async fn follow(tmux_socket: &Path, listener: &UnixListener, state: &Shared) -> Result<bool> {
    let mut child = tokio::process::Command::new("tmux")
        .arg("-S")
        .arg(tmux_socket)
        .args(["-C", "attach-session", "-f", "no-output,ignore-size"])
        .env_remove("TMUX")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    let mut input = child.stdin.take().ok_or("tmux has no stdin")?;
    let mut lines = BufReader::new(child.stdout.take().ok_or("tmux has no stdout")?).lines();
    let mut parser = Parser::default();
    let mut pending = VecDeque::new();
    let mut stale = true;
    let mut attached = false;
    loop {
        if stale && pending.is_empty() {
            let batch: String = Query::ALL.iter().map(|q| q.control_line()).collect();
            if input.write_all(batch.as_bytes()).await.is_err() {
                break;
            }
            pending.extend(Query::ALL);
            stale = false;
        }
        tokio::select! {
            line = lines.next_line() => {
                let Ok(Some(line)) = line else { break };
                match parser.feed(&line) {
                    Some(Event::Reply(reply)) => {
                        attached = true;
                        if reply.ours {
                            if let (Some(query), true) = (pending.pop_front(), reply.ok) {
                                lock(state).apply(query, reply.lines.iter().map(String::as_str));
                            }
                        }
                    }
                    Some(Event::Notification(notification)) => {
                        stale |= update(&mut lock(state), notification);
                    }
                    None => {}
                }
            }
            connection = listener.accept() => {
                tokio::spawn(answer(connection?.0, state.clone()));
            }
        }
    }
    drop(input);
    child.wait().await?;
    Ok(attached)
}

fn update(state: &mut State, notification: Notification) -> bool {
    match notification {
        Notification::SessionRenamed { session, name } => state.rename_session(&session, &name),
        Notification::WindowRenamed { window, name } => state.rename_window(&window, &name),
        Notification::WindowClose { window } => state.close_window(&window),
        Notification::SessionsChanged
        | Notification::SessionChanged
        | Notification::WindowAdd
        | Notification::ClientSessionChanged
        | Notification::ClientDetached => return true,
        Notification::Exit | Notification::Ignored => {}
    }
    false
}

fn lock(state: &Shared) -> std::sync::MutexGuard<'_, State> {
    state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

async fn answer(stream: UnixStream, state: Shared) -> std::io::Result<()> {
    let (read, mut write) = stream.into_split();
    let mut lines = BufReader::new(read).lines();
    while let Some(line) = lines.next_line().await? {
        let mut reply = match serde_json::from_str::<Request>(&line) {
            Ok(Request::Status) => serde_json::to_string(&StatusReply {
                pid: std::process::id(),
                sessions: lock(&state).snapshot(),
            })?,
            Err(error) => serde_json::json!({ "error": error.to_string() }).to_string(),
        };
        reply.push('\n');
        write.write_all(reply.as_bytes()).await?;
    }
    Ok(())
}
