use crate::tmux::Tmux;
use crate::Result;

pub(super) fn preview(tmux: &Tmux, session: &str, lines: usize) -> Result<String> {
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
