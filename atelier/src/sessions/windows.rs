use crate::tmux::Tmux;
use crate::Result;

pub(super) fn cycle(tmux: &Tmux, direction: &str, session: &str) -> Result<()> {
    if session.is_empty() || tmux.display(session, "#{session_windows}")? == "1" {
        return Ok(());
    }
    tmux.run(&[direction, "-t", session])?;
    Ok(())
}
