use std::fmt::Write;

use serde::{Deserialize, Serialize};

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
            Query::Sessions => ["list-sessions", "-F", "#{session_id} #{session_name}"],
            Query::Windows => [
                "list-windows",
                "-aF",
                "#{session_id} #{window_id} #{window_index} #{window_name}",
            ],
            Query::Clients => [
                "list-clients",
                "-F",
                "#{client_control_mode} #{session_id} #{client_name}",
            ],
        }
    }

    pub fn control_line(self) -> String {
        let [command, flag, format] = self.args();
        format!("{command} {flag} '{format}'\n")
    }
}

struct Winlink {
    session: String,
    window: String,
    index: u32,
    name: String,
}

#[derive(Default)]
pub struct State {
    sessions: Vec<(String, String)>,
    winlinks: Vec<Winlink>,
    watchers: Vec<String>,
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
            Query::Sessions => {
                self.sessions = lines
                    .filter_map(|line| line.split_once(' '))
                    .map(|(id, name)| (id.to_string(), name.to_string()))
                    .collect()
            }
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
                        let mut fields = line.splitn(3, ' ');
                        let control = fields.next()?;
                        (control == "0").then(|| fields.next().map(str::to_string))?
                    })
                    .collect()
            }
        }
    }

    pub fn rename_session(&mut self, id: &str, name: &str) {
        for session in self.sessions.iter_mut().filter(|(sid, _)| sid == id) {
            session.1 = name.to_string();
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

    pub fn snapshot(&self) -> Vec<Session> {
        let mut sessions: Vec<Session> = self
            .sessions
            .iter()
            .map(|(id, name)| {
                let mut windows: Vec<Window> = self
                    .winlinks
                    .iter()
                    .filter(|w| &w.session == id)
                    .map(|w| Window {
                        index: w.index,
                        name: w.name.clone(),
                    })
                    .collect();
                windows.sort_by_key(|w| w.index);
                Session {
                    name: name.clone(),
                    watchers: self.watchers.iter().filter(|s| *s == id).count(),
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
