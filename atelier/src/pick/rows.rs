use std::path::PathBuf;

use crate::tmux::Tmux;
use crate::Result;

use super::paths::expand;
use super::tokens::candidates;

const PATH_CANDIDATES: usize = 400;
const ROWS: usize = 200;
pub(super) const PLACEHOLDER: &str = "(no URL or existing path on this pane)";

pub(super) fn rows(tmux: &Tmux, pane: &str) -> Result<Vec<String>> {
    let cwd = PathBuf::from(tmux.display(pane, "#{pane_current_path}")?);
    let capture = tmux.run(&["capture-pane", "-p", "-J", "-S", "-2000", "-t", pane])?;
    let (urls, paths) = candidates(&capture);
    let rows: Vec<String> = urls
        .into_iter()
        .chain(
            paths
                .into_iter()
                .take(PATH_CANDIDATES)
                .filter(|path| expand(&cwd, path).exists()),
        )
        .take(ROWS)
        .collect();
    if rows.is_empty() {
        return Ok(vec![PLACEHOLDER.to_string()]);
    }
    Ok(rows)
}
