use std::path::{Path, PathBuf};

use clap::Subcommand;

use crate::fzf;
use crate::session;
use crate::shell;
use crate::tmux::Tmux;
use crate::Result;

const NO_OTHER_SESSION: &str = "(no other session)";
const NO_OTHER_IN_PROJECT: &str = "(no other session in this project)";
const SCOPE_OPTION: &str = "@atelier_sessions_scope";
const NAVIGATION_KEYS: &str = "j,k,q,h,l";

#[derive(Subcommand)]
pub enum Command {
    /// Open the session picker in fzf; meant for `display-popup -E`
    Pick {
        /// The session the picker is opened from; defaults to the current one
        #[arg(short = 't', long, value_name = "SESSION")]
        target: Option<String>,
        /// The client to switch; defaults to the current one
        #[arg(short = 'c', long, value_name = "CLIENT")]
        client: Option<String>,
    },
    /// Print the picker's rows, one per other session: its id, a tab, its label
    Rows {
        #[arg(short = 't', long, value_name = "SESSION")]
        target: String,
    },
    /// Toggle between this project's sessions and all of them; prints fzf actions
    Toggle {
        #[arg(short = 't', long, value_name = "SESSION")]
        target: String,
    },
    /// Print the picker's key help for the current scope
    Header {
        #[arg(short = 't', long, value_name = "SESSION")]
        target: String,
    },
    /// Leave search, or close the picker when not searching; prints fzf actions
    Escape {
        #[arg(short = 't', long, value_name = "SESSION")]
        target: String,
    },
    /// Switch a client to a session; an empty session does nothing
    Switch {
        #[arg(short = 'c', long, value_name = "CLIENT")]
        client: Option<String>,
        session: String,
    },
    /// Print a session's window list and the tail of its current window
    Preview { session: String },
    /// Move a session to its next window; the picker's `l`
    Next { session: String },
    /// Move a session to its previous window; the picker's `h`
    Prev { session: String },
    /// Kill a session and reload the picker's rows; prints fzf actions
    Kill {
        #[arg(short = 't', long, value_name = "SESSION")]
        target: String,
        session: String,
    },
}

impl Command {
    pub fn run(self, tmux: &Tmux) -> Result<()> {
        match self {
            Command::Pick { target, client } => pick(tmux, target, client),
            Command::Rows { target } => {
                for row in Picker::open(tmux, &target)?.rows()? {
                    println!("{row}");
                }
                Ok(())
            }
            Command::Toggle { target } => {
                print_actions(&Picker::open(tmux, &target)?.toggle()?);
                Ok(())
            }
            Command::Header { target } => {
                println!("{}", Picker::open(tmux, &target)?.header()?);
                Ok(())
            }
            Command::Escape { target } => {
                let input_state = std::env::var("FZF_INPUT_STATE").unwrap_or_default();
                print_actions(&Picker::open(tmux, &target)?.escape(&input_state)?);
                Ok(())
            }
            Command::Switch { client, session } => switch(tmux, client.as_deref(), &session),
            Command::Preview { session } => {
                let lines = std::env::var("FZF_PREVIEW_LINES")
                    .ok()
                    .and_then(|lines| lines.parse().ok())
                    .unwrap_or(24);
                print!("{}", preview(tmux, session.trim(), lines)?);
                Ok(())
            }
            Command::Next { session } => cycle(tmux, "next-window", session.trim()),
            Command::Prev { session } => cycle(tmux, "previous-window", session.trim()),
            Command::Kill { target, session } => {
                print_actions(&Picker::open(tmux, &target)?.kill(session.trim())?);
                Ok(())
            }
        }
    }
}

fn cycle(tmux: &Tmux, direction: &str, session: &str) -> Result<()> {
    if session.is_empty() || tmux.display(session, "#{session_windows}")? == "1" {
        return Ok(());
    }
    tmux.run(&[direction, "-t", session])?;
    Ok(())
}

fn preview(tmux: &Tmux, session: &str, lines: usize) -> Result<String> {
    if session.is_empty() {
        return Ok(String::new());
    }
    let windows = tmux.run(&[
        "list-windows",
        "-t",
        session,
        "-F",
        "#{window_active}#{window_index}: #{window_name}  (#{window_panes}p)",
    ])?;
    let mut shown = String::new();
    for window in windows.lines() {
        let (active, label) = window.split_at(1);
        let marker = if active == "1" { " ←" } else { "" };
        shown.push_str(&format!("{label}{marker}\n"));
    }
    shown.push('\n');
    let room = lines.saturating_sub(windows.lines().count() + 1);
    let capture = tmux.run(&["capture-pane", "-p", "-e", "-t", session])?;
    let captured: Vec<&str> = capture.lines().collect();
    let end = captured
        .iter()
        .rposition(|line| !without_sgr(line).trim().is_empty())
        .map_or(0, |last| last + 1);
    for line in &captured[end.saturating_sub(room)..end] {
        shown.push_str(line);
        shown.push('\n');
    }
    Ok(shown)
}

fn without_sgr(line: &str) -> String {
    let mut bare = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            bare.push(c);
            continue;
        }
        if chars.next() == Some('[') {
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        }
    }
    bare
}

fn print_actions(actions: &str) {
    if !actions.is_empty() {
        println!("{actions}");
    }
}

fn pick(tmux: &Tmux, target: Option<String>, client: Option<String>) -> Result<()> {
    let target = tmux.resolve(target.as_deref(), "#{session_id}")?;
    let client = match client {
        Some(client) => client,
        None => tmux.run(&["display-message", "-p", "#{client_name}"])?,
    };
    let picker = Picker::open(tmux, &target)?;
    picker.set_scope(Scope::Project)?;
    let rows = picker.rows()?;

    let atelier = fzf::atelier(tmux)?;
    let mut switch = format!("{atelier} sessions switch");
    if !client.is_empty() {
        switch.push_str(&format!(" -c {}", shell::quote(&client)));
    }
    let mut finder = fzf::picker(
        picker.scope()?.prompt(),
        &picker.header()?,
        NAVIGATION_KEYS,
    );
    finder
        .args(["--delimiter=\t", "--with-nth=2.."])
        .arg("--bind")
        .arg(format!("esc:transform:{}", picker.call("escape")?))
        .arg("--bind")
        .arg(format!("tab:transform:{}", picker.call("toggle")?))
        .arg("--bind")
        .arg(format!("enter:become:{switch} {{1}}"))
        .arg("--bind")
        .arg(format!(
            "l:execute-silent({atelier} sessions next {{1}})+refresh-preview"
        ))
        .arg("--bind")
        .arg(format!(
            "h:execute-silent({atelier} sessions prev {{1}})+refresh-preview"
        ))
        .arg("--bind")
        .arg(format!("ctrl-x:transform:{} {{1}}", picker.call("kill")?))
        .arg(format!("--preview={atelier} sessions preview {{1}}"))
        .arg("--preview-window=right,70%,border-left,nowrap");
    fzf::spawn(&mut finder, rows)?.wait()?;
    Ok(())
}

fn switch(tmux: &Tmux, client: Option<&str>, session: &str) -> Result<()> {
    let session = session.trim();
    if session.is_empty() {
        return Ok(());
    }
    let mut args = vec!["switch-client"];
    if let Some(client) = client {
        args.extend(["-c", client]);
    }
    args.extend(["-t", session]);
    tmux.run(&args)?;
    Ok(())
}

#[derive(Clone, Copy, PartialEq)]
enum Scope {
    Project,
    All,
}

impl Scope {
    fn prompt(self) -> &'static str {
        match self {
            Scope::Project => "  project  ",
            Scope::All => "  all  ",
        }
    }

    fn option_value(self) -> &'static str {
        match self {
            Scope::Project => "project",
            Scope::All => "all",
        }
    }
}

struct Picker<'a> {
    tmux: &'a Tmux,
    session: String,
    project: Option<PathBuf>,
}

impl<'a> Picker<'a> {
    fn open(tmux: &'a Tmux, target: &str) -> Result<Self> {
        let session = tmux.resolve(Some(target), "#{session_id}")?;
        Ok(Picker {
            tmux,
            project: session::resolve(tmux, &session)?.root,
            session,
        })
    }

    fn scope(&self) -> Result<Scope> {
        if self.project.is_none() {
            return Ok(Scope::All);
        }
        let stored = self
            .tmux
            .display(&self.session, &format!("#{{{SCOPE_OPTION}}}"))?;
        Ok(if stored == Scope::All.option_value() {
            Scope::All
        } else {
            Scope::Project
        })
    }

    fn set_scope(&self, scope: Scope) -> Result<()> {
        self.tmux.run(&[
            "set-option",
            "-t",
            &self.session,
            SCOPE_OPTION,
            scope.option_value(),
        ])?;
        Ok(())
    }

    fn rows(&self) -> Result<Vec<String>> {
        let project = match self.scope()? {
            Scope::Project => self.project.as_deref(),
            Scope::All => None,
        };
        let clients = self.tmux.run(&[
            "list-clients",
            "-F",
            "#{client_control_mode} #{session_id}",
        ])?;
        let watched: Vec<&str> = clients
            .lines()
            .filter_map(|line| line.strip_prefix("0 "))
            .collect();
        let mut listed = Vec::new();
        for id in self
            .tmux
            .run(&["list-sessions", "-F", "#{session_id}"])?
            .lines()
        {
            if id != self.session && in_project(self.tmux, id, project)? {
                listed.push(Listed::read(self.tmux, id, watched.contains(&id))?);
            }
        }
        if listed.is_empty() {
            let notice = if project.is_some() {
                NO_OTHER_IN_PROJECT
            } else {
                NO_OTHER_SESSION
            };
            return Ok(vec![format!("\t{notice}")]);
        }
        listed.sort_by(|a, b| {
            b.last_attached
                .cmp(&a.last_attached)
                .then_with(|| a.name.cmp(&b.name))
        });
        let width = listed
            .iter()
            .map(|session| session.name.chars().count())
            .max()
            .unwrap_or(0);
        Ok(listed.iter().map(|session| session.row(width)).collect())
    }

    fn header(&self) -> Result<String> {
        let navigation = match (&self.project, self.scope()?) {
            (None, _) => "j/k move   h/l window   i search   (no project)",
            (Some(_), Scope::Project) => "j/k move   h/l window   tab show all   i search",
            (Some(_), Scope::All) => "j/k move   h/l window   tab project only   i search",
        };
        Ok(format!("{navigation}\nenter switch   ctrl-x kill"))
    }

    fn toggle(&self) -> Result<String> {
        if self.project.is_none() {
            return Ok(String::new());
        }
        let scope = match self.scope()? {
            Scope::Project => Scope::All,
            Scope::All => Scope::Project,
        };
        self.set_scope(scope)?;
        Ok(format!(
            "reload({})+change-prompt({})+transform-header({})+first",
            self.call("rows")?,
            scope.prompt(),
            self.call("header")?
        ))
    }

    fn kill(&self, session: &str) -> Result<String> {
        if session.is_empty() {
            return Ok(String::new());
        }
        self.tmux.run(&["kill-session", "-t", session])?;
        Ok(format!("reload({})", self.call("rows")?))
    }

    fn escape(&self, input_state: &str) -> Result<String> {
        if input_state != "enabled" {
            return Ok("abort".to_string());
        }
        Ok(format!(
            "disable-search+clear-query+rebind({NAVIGATION_KEYS})+change-prompt({})",
            self.scope()?.prompt()
        ))
    }

    fn call(&self, subcommand: &str) -> Result<String> {
        Ok(format!(
            "{} sessions {subcommand} -t {}",
            fzf::atelier(self.tmux)?,
            shell::quote(&self.session)
        ))
    }
}

fn in_project(tmux: &Tmux, id: &str, project: Option<&Path>) -> Result<bool> {
    Ok(match project {
        Some(root) => session::resolve(tmux, id)?.root.as_deref() == Some(root),
        None => true,
    })
}

struct Listed {
    id: String,
    name: String,
    windows: String,
    attached: bool,
    last_attached: u64,
}

impl Listed {
    fn read(tmux: &Tmux, id: &str, attached: bool) -> Result<Self> {
        let line = tmux.display(
            id,
            "#{session_last_attached} #{session_windows} #{session_name}",
        )?;
        let mut fields = line.splitn(3, ' ');
        let mut next = || fields.next().unwrap_or_default().to_string();
        let last_attached = next().parse().unwrap_or(0);
        let windows = next();
        Ok(Listed {
            id: id.to_string(),
            name: next(),
            windows,
            attached,
            last_attached,
        })
    }

    fn row(&self, width: usize) -> String {
        let attached = if self.attached { "  · attached" } else { "" };
        format!(
            "{}\t{:<width$}  {:>3} win{attached}",
            self.id, self.name, self.windows
        )
    }
}
