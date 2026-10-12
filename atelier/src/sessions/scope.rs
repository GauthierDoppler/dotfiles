use crate::tmux::Tmux;
use crate::Result;

const SCOPE_OPTION: &str = "@atelier_sessions_scope";

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Scope {
    Project,
    All,
}

impl Scope {
    pub(super) fn prompt(self) -> &'static str {
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

    pub(super) fn stored(tmux: &Tmux, session: &str) -> Result<Scope> {
        let stored = tmux.display(session, &format!("#{{{SCOPE_OPTION}}}"))?;
        Ok(if stored == Scope::All.option_value() {
            Scope::All
        } else {
            Scope::Project
        })
    }

    pub(super) fn store(self, tmux: &Tmux, session: &str) -> Result<()> {
        tmux.run(&[
            "set-option",
            "-t",
            session,
            SCOPE_OPTION,
            self.option_value(),
        ])?;
        Ok(())
    }
}
