use std::collections::VecDeque;
use std::path::Path;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

use super::bar::{adopt, RESIZED};
use super::control::{Event, Notification, Parser};
use super::render::{self, Input};
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
    while let Some(target) = blocking(tmux_socket, first_in_picker_order).await? {
        let attached = follow(&tmux, &target, &listener, &state).await?;
        let gone = {
            let target = target.clone();
            blocking(tmux_socket, move |tmux| {
                tmux.run(&["has-session", "-t", &target]).is_err()
            })
            .await?
        };
        if !attached && !gone {
            break;
        }
    }
    Ok(())
}

async fn blocking<T: Send + 'static>(
    tmux_socket: &Path,
    work: impl FnOnce(&Tmux) -> T + Send + 'static,
) -> Result<T> {
    let socket = tmux_socket.to_path_buf();
    Ok(tokio::task::spawn_blocking(move || work(&Tmux::new(Some(socket)))).await?)
}

fn first_in_picker_order(tmux: &Tmux) -> Option<String> {
    let sessions = tmux
        .run(&[
            "list-sessions",
            "-F",
            "#{session_last_attached} #{session_id} #{session_name}",
        ])
        .ok()?;
    sessions
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, ' ');
            let last_attached = fields.next()?.parse::<u64>().unwrap_or(0);
            Some((last_attached, fields.next()?, fields.next().unwrap_or("")))
        })
        .min_by(|a, b| b.0.cmp(&a.0).then_with(|| a.2.cmp(b.2)))
        .map(|(_, id, _)| id.to_string())
}

async fn follow(
    tmux: &Tmux,
    target: &str,
    listener: &UnixListener,
    state: &Shared,
) -> Result<bool> {
    let mut child = tokio::process::Command::from(tmux.command())
        .args(["-C", "attach-session", "-f", "no-output,ignore-size", "-t", target])
        .env_remove("TMUX")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    let mut input = child.stdin.take().ok_or("tmux has no stdin")?;
    let mut lines = BufReader::new(child.stdout.take().ok_or("tmux has no stdout")?).lines();
    let mut parser = Parser::default();
    let mut pending = VecDeque::from([Pending::Me]);
    let mut out = "display-message -p '#{client_name}'\n".to_string();
    let (rendered, mut commands) = tokio::sync::mpsc::unbounded_channel();
    let render = render::spawn(rendered);
    let mut push_due = false;
    let mut stale = true;
    let mut attached = false;
    loop {
        if stale && pending.is_empty() {
            out.extend(Query::ALL.iter().map(|q| q.control_line()));
            pending.extend(Query::ALL.map(Pending::Query));
            stale = false;
            push_due = true;
        }
        if push_due && pending.is_empty() {
            let (watched, sessions) = {
                let state = lock(state);
                (state.watched(), state.fields())
            };
            if render.send(Input::Push { watched, sessions }).is_err() {
                break;
            }
            push_due = false;
        }
        if !out.is_empty() && input.write_all(std::mem::take(&mut out).as_bytes()).await.is_err() {
            break;
        }
        tokio::select! {
            line = lines.next_line() => {
                let Ok(Some(line)) = line else { break };
                match parser.feed(&line) {
                    Some(Event::Reply(reply)) => {
                        attached = true;
                        if !reply.ours {
                            continue;
                        }
                        match (pending.pop_front(), reply.ok) {
                            (Some(Pending::Query(query)), true) => {
                                lock(state).apply(query, reply.lines.iter().map(String::as_str));
                            }
                            (Some(Pending::Me), true) => {
                                for command in adopt(&reply.lines.concat()) {
                                    out.push_str(&command);
                                    out.push('\n');
                                    pending.push_back(Pending::Ignored);
                                }
                            }
                            _ => {}
                        }
                    }
                    Some(Event::Notification(notification)) => {
                        stale |= update(&mut lock(state), notification);
                    }
                    None => {}
                }
            }
            Some(pushed) = commands.recv() => {
                for command in pushed {
                    out.push_str(&command);
                    out.push('\n');
                    pending.push_back(Pending::Ignored);
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

enum Pending {
    Me,
    Query(Query),
    Ignored,
}

fn update(state: &mut State, notification: Notification) -> bool {
    match notification {
        Notification::SessionRenamed { session, name } => {
            state.rename_session(&session, &name);
            return true;
        }
        Notification::WindowRenamed { window, name } => state.rename_window(&window, &name),
        Notification::WindowClose { window } => state.close_window(&window),
        Notification::Message(message) => return message == RESIZED,
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
