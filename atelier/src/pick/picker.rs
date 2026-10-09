use crate::fzf;
use crate::shell;
use crate::tmux::Tmux;
use crate::Result;

use super::rows::rows;

pub(super) fn popup(tmux: &Tmux, pane: &str) -> Result<()> {
    let rows = rows(tmux, pane)?;
    let atelier = fzf::atelier(tmux)?;
    let pane = shell::quote(pane);
    let mut picker = fzf::picker(
        "  pick  ",
        "j/k move   i search\nenter open   ctrl-y copy   ctrl-o system open   ctrl-v preview .md   esc close",
        fzf::MODAL_KEYS,
    );
    picker.args([
        &fzf::leave_search(fzf::MODAL_KEYS, "change-prompt(  pick  )"),
        &format!("--bind=enter:execute-silent({atelier} pick open -t {pane} {{}})+abort"),
        &format!("--bind=ctrl-y:execute-silent({atelier} pick copy {{}})+abort"),
        &format!("--bind=ctrl-o:execute-silent({atelier} pick system -t {pane} {{}})+abort"),
        &format!("--bind=ctrl-v:execute-silent({atelier} pick preview -t {pane} {{}})+abort"),
    ]);
    fzf::spawn(&mut picker, rows)?.wait()?;
    Ok(())
}
