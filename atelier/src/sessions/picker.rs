use std::path::PathBuf;

use crate::fzf;
use crate::session;
use crate::shell;
use crate::tmux::Tmux;
use crate::Result;

use super::rows::rows;
use super::scope::Scope;

const NAVIGATION_KEYS: &str = "j,k,q,h,l";

pub(super) fn pick(tmux: &Tmux, target: Option<String>, client: Option<String>) -> Result<()> {
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

pub(super) fn switch(tmux: &Tmux, client: Option<&str>, session: &str) -> Result<()> {
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

pub(super) struct Picker<'a> {
    tmux: &'a Tmux,
    session: String,
    project: Option<PathBuf>,
}

impl<'a> Picker<'a> {
    pub(super) fn open(tmux: &'a Tmux, target: &str) -> Result<Self> {
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
        Scope::stored(self.tmux, &self.session)
    }

    fn set_scope(&self, scope: Scope) -> Result<()> {
        scope.store(self.tmux, &self.session)
    }

    pub(super) fn rows(&self) -> Result<Vec<String>> {
        let project = match self.scope()? {
            Scope::Project => self.project.as_deref(),
            Scope::All => None,
        };
        rows(self.tmux, &self.session, project)
    }

    pub(super) fn header(&self) -> Result<String> {
        let navigation = match (&self.project, self.scope()?) {
            (None, _) => "j/k move   h/l window   i search   (no project)",
            (Some(_), Scope::Project) => "j/k move   h/l window   tab show all   i search",
            (Some(_), Scope::All) => "j/k move   h/l window   tab project only   i search",
        };
        Ok(format!("{navigation}\nenter switch   ctrl-x kill"))
    }

    pub(super) fn toggle(&self) -> Result<String> {
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

    pub(super) fn kill(&self, session: &str) -> Result<String> {
        if session.is_empty() {
            return Ok(String::new());
        }
        self.tmux.run(&["kill-session", "-t", session])?;
        Ok(format!("reload({})", self.call("rows")?))
    }

    pub(super) fn escape(&self, input_state: &str) -> Result<String> {
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
