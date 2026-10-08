use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use clap::Subcommand;

use crate::session;
use crate::tmux::Tmux;
use crate::Result;

const NO_OTHER_SESSION: &str = "(no other session)";
const NO_OTHER_IN_PROJECT: &str = "(no other session in this project)";
const SCOPE_OPTION: &str = "@atelier_sessions_scope";
const NAVIGATION_KEYS: &str = "j,k,q";

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
        }
    }
}

fn print_actions(actions: &str) {
    if !actions.is_empty() {
        println!("{actions}");
    }
}

fn pick(tmux: &Tmux, target: Option<String>, client: Option<String>) -> Result<()> {
    let target = match target {
        Some(target) => target,
        None => tmux.run(&["display-message", "-p", "#{session_id}"])?,
    };
    let client = match client {
        Some(client) => client,
        None => tmux.run(&["display-message", "-p", "#{client_name}"])?,
    };
    let picker = Picker::open(tmux, &target)?;
    picker.set_scope(Scope::Project)?;
    let rows = picker.rows()?;

    let mut switch = format!("{} sessions switch", picker.atelier_command()?);
    if !client.is_empty() {
        switch.push_str(&format!(" -c {}", shell_quote(&client)));
    }
    let mut fzf = std::process::Command::new("fzf")
        .args([
            "--disabled",
            "--delimiter=\t",
            "--with-nth=2..",
            "--header-first",
            "--pointer=▸",
            "--no-multi",
            "--cycle",
        ])
        .arg(format!("--prompt={}", picker.scope()?.prompt()))
        .arg(format!("--header={}", picker.header()?))
        .args(["--bind", "j:down,k:up", "--bind", "q:abort"])
        .arg("--bind")
        .arg(format!(
            "i:enable-search+unbind({NAVIGATION_KEYS})+change-prompt(  search  )"
        ))
        .arg("--bind")
        .arg(format!("esc:transform:{}", picker.call("escape")?))
        .arg("--bind")
        .arg(format!("tab:transform:{}", picker.call("toggle")?))
        .arg("--bind")
        .arg(format!("enter:become:{switch} {{1}}"))
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|error| format!("fzf: {error}"))?;
    if let Some(mut stdin) = fzf.stdin.take() {
        for row in rows {
            writeln!(stdin, "{row}")?;
        }
    }
    fzf.wait()?;
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
        let project = session::resolve(tmux, target)?.root;
        Ok(Picker {
            tmux,
            session: tmux.display(target, "#{session_id}")?,
            project,
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
        let mut listed = Vec::new();
        for id in self
            .tmux
            .run(&["list-sessions", "-F", "#{session_id}"])?
            .lines()
        {
            if id != self.session && in_project(self.tmux, id, project)? {
                listed.push(Listed::read(self.tmux, id)?);
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
            (None, _) => "j/k move   i search   (no project)",
            (Some(_), Scope::Project) => "j/k move   tab show all   i search",
            (Some(_), Scope::All) => "j/k move   tab project only   i search",
        };
        Ok(format!("{navigation}\nenter switch"))
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

    fn escape(&self, input_state: &str) -> Result<String> {
        if input_state != "enabled" {
            return Ok("abort".to_string());
        }
        Ok(format!(
            "disable-search+clear-query+rebind({NAVIGATION_KEYS})+change-prompt({})",
            self.scope()?.prompt()
        ))
    }

    fn atelier_command(&self) -> Result<String> {
        let mut command = shell_quote(&std::env::current_exe()?.to_string_lossy());
        if let Some(socket) = self.tmux.socket() {
            command.push_str(" --socket ");
            command.push_str(&shell_quote(&socket.to_string_lossy()));
        }
        Ok(command)
    }

    fn call(&self, subcommand: &str) -> Result<String> {
        Ok(format!(
            "{} sessions {subcommand} -t {}",
            self.atelier_command()?,
            shell_quote(&self.session)
        ))
    }
}

fn shell_quote(word: &str) -> String {
    format!("'{}'", word.replace('\'', r"'\''"))
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
    fn read(tmux: &Tmux, id: &str) -> Result<Self> {
        let line = tmux.display(
            id,
            "#{session_last_attached} #{session_windows} #{session_attached} #{session_name}",
        )?;
        let mut fields = line.splitn(4, ' ');
        let mut next = || fields.next().unwrap_or_default().to_string();
        let last_attached = next().parse().unwrap_or(0);
        let windows = next();
        let attached = next() != "0";
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
