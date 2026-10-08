use std::fmt::Write;

use serde::{Deserialize, Serialize};

use crate::session::Field;
use crate::tmux::Tmux;
use crate::Result;

#[derive(Clone, Copy)]
pub enum Query {
    Sessions,
    Windows,
    Clients,
}

impl Query {
    pub const ALL: [Query; 3] = [Query::Sessions, Query::Windows, Query::Clients];

    pub fn args(self) -> [&'static str; 3] {
        match self {
            Query::Sessions => [
                "list-sessions",
                "-F",
                "#{session_id} #{q:@grove_project} #{q:@grove_root} #{q:@grove_worktree} #{q:session_path} #{session_name}",
            ],
            Query::Windows => [
                "list-windows",
                "-aF",
                "#{session_id} #{window_id} #{window_index} #{window_name}",
            ],
            Query::Clients => [
                "list-clients",
                "-F",
                "#{client_control_mode} #{session_id} #{client_width} #{client_activity} #{client_name}",
            ],
        }
    }

    pub fn control_line(self) -> String {
        let [command, flag, format] = self.args();
        format!("{command} {flag} '{format}'\n")
    }
}

struct SessionRow {
    id: String,
    fields: [String; Field::ALL.len()],
}

impl SessionRow {
    fn parse(line: &str) -> Option<SessionRow> {
        let (id, mut rest) = line.split_once(' ')?;
        let mut fields: [String; Field::ALL.len()] = Default::default();
        for field in &mut fields[..Field::Name as usize] {
            let mut chars = rest.char_indices();
            loop {
                match chars.next() {
                    Some((_, '\\')) => field.push(chars.next()?.1),
                    Some((at, ' ')) => {
                        rest = &rest[at + 1..];
                        break;
                    }
                    Some((_, c)) => field.push(c),
                    None => return None,
                }
            }
        }
        fields[Field::Name as usize] = rest.to_string();
        Some(SessionRow {
            id: id.to_string(),
            fields,
        })
    }

    fn name(&self) -> &str {
        &self.fields[Field::Name as usize]
    }
}

struct Winlink {
    session: String,
    window: String,
    index: u32,
    name: String,
}

struct Watcher {
    session: String,
    width: u16,
    activity: u64,
}

pub struct Watched {
    pub session: String,
    pub width: u16,
    pub fields: [String; Field::ALL.len()],
}

#[derive(Default)]
pub struct State {
    sessions: Vec<SessionRow>,
    winlinks: Vec<Winlink>,
    watchers: Vec<Watcher>,
}

impl State {
    pub fn from_tmux(tmux: &Tmux) -> Result<State> {
        let mut state = State::default();
        for query in Query::ALL {
            let output = tmux.run(&query.args())?;
            state.apply(query, output.lines());
        }
        Ok(state)
    }

    pub fn apply<'a>(&mut self, query: Query, lines: impl Iterator<Item = &'a str>) {
        match query {
            Query::Sessions => self.sessions = lines.filter_map(SessionRow::parse).collect(),
            Query::Windows => {
                self.winlinks = lines
                    .filter_map(|line| {
                        let mut fields = line.splitn(4, ' ');
                        Some(Winlink {
                            session: fields.next()?.to_string(),
                            window: fields.next()?.to_string(),
                            index: fields.next()?.parse().ok()?,
                            name: fields.next()?.to_string(),
                        })
                    })
                    .collect()
            }
            Query::Clients => {
                self.watchers = lines
                    .filter_map(|line| {
                        let mut fields = line.splitn(5, ' ');
                        if fields.next()? != "0" {
                            return None;
                        }
                        Some(Watcher {
                            session: fields.next()?.to_string(),
                            width: fields.next()?.parse().ok()?,
                            activity: fields.next()?.parse().ok()?,
                        })
                    })
                    .collect()
            }
        }
    }

    pub fn rename_session(&mut self, id: &str, name: &str) {
        for session in self.sessions.iter_mut().filter(|s| s.id == id) {
            session.fields[Field::Name as usize] = name.to_string();
        }
    }

    pub fn rename_window(&mut self, id: &str, name: &str) {
        for winlink in self.winlinks.iter_mut().filter(|w| w.window == id) {
            winlink.name = name.to_string();
        }
    }

    pub fn close_window(&mut self, id: &str) {
        self.winlinks.retain(|w| w.window != id);
    }

    pub fn watched(&self) -> Vec<Watched> {
        self.sessions
            .iter()
            .filter_map(|session| {
                let watcher = self
                    .watchers
                    .iter()
                    .filter(|w| w.session == session.id)
                    .max_by_key(|w| w.activity)?;
                Some(Watched {
                    session: session.id.clone(),
                    width: watcher.width,
                    fields: session.fields.clone(),
                })
            })
            .collect()
    }

    pub fn snapshot(&self) -> Vec<Session> {
        let mut sessions: Vec<Session> = self
            .sessions
            .iter()
            .map(|session| {
                let mut windows: Vec<Window> = self
                    .winlinks
                    .iter()
                    .filter(|w| w.session == session.id)
                    .map(|w| Window {
                        index: w.index,
                        name: w.name.clone(),
                    })
                    .collect();
                windows.sort_by_key(|w| w.index);
                Session {
                    name: session.name().to_string(),
                    watchers: self
                        .watchers
                        .iter()
                        .filter(|w| w.session == session.id)
                        .count(),
                    windows,
                }
            })
            .collect();
        sessions.sort_by(|a, b| a.name.cmp(&b.name));
        sessions
    }
}

#[derive(Serialize, Deserialize)]
pub struct Session {
    pub name: String,
    pub watchers: usize,
    pub windows: Vec<Window>,
}

#[derive(Serialize, Deserialize)]
pub struct Window {
    pub index: u32,
    pub name: String,
}

pub fn render(daemon: Option<u32>, sessions: &[Session]) -> String {
    let mut out = match daemon {
        Some(pid) => format!("daemon: running (pid {pid})\n"),
        None => "daemon: not running\n".to_string(),
    };
    for session in sessions {
        out.push_str(&session.name);
        if session.watchers > 0 {
            out.push_str(" (attached)");
        }
        out.push('\n');
        for window in &session.windows {
            let _ = writeln!(out, "  {} {}", window.index, window.name);
        }
    }
    out
}
