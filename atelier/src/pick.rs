use std::collections::HashSet;
use std::path::{Path, PathBuf};

use clap::Subcommand;

use crate::fzf;
use crate::opener;
use crate::shell;
use crate::tmux::Tmux;
use crate::Result;

const URL_SCHEMES: [&str; 3] = ["https://", "http://", "ftp://"];
const PATH_CANDIDATES: usize = 400;
const ROWS: usize = 200;
const PLACEHOLDER: &str = "(no URL or existing path on this pane)";

#[derive(Subcommand)]
pub enum Command {
    /// Print the URLs and existing paths on a pane, newest first
    List {
        #[arg(short = 't', long, value_name = "PANE", default_value = "")]
        target: String,
    },
    /// Open a token: a URL in the browser, a file in this session's nvim
    Open {
        #[arg(short = 't', long, value_name = "PANE", default_value = "")]
        target: String,
        token: String,
    },
    /// Hand a token to the OS opener
    System {
        #[arg(short = 't', long, value_name = "PANE", default_value = "")]
        target: String,
        token: String,
    },
    /// Open a markdown path in the preview; anything else does nothing
    Preview {
        #[arg(short = 't', long, value_name = "PANE", default_value = "")]
        target: String,
        token: String,
    },
    /// Copy a token to the clipboard through tmux (OSC 52)
    Copy { token: String },
    /// Run the fzf picker over a pane, for `display-popup -E`
    Popup {
        #[arg(short = 't', long, value_name = "PANE", default_value = "")]
        target: String,
    },
}

impl Command {
    pub fn run(self, tmux: &Tmux) -> Result<()> {
        match self {
            Command::List { target } => {
                for row in rows(tmux, &resolve_pane(tmux, &target)?)? {
                    println!("{row}");
                }
                Ok(())
            }
            Command::Popup { target } => popup(tmux, &resolve_pane(tmux, &target)?),
            Command::Open { token, .. }
            | Command::System { token, .. }
            | Command::Preview { token, .. }
            | Command::Copy { token }
                if token == PLACEHOLDER =>
            {
                Ok(())
            }
            Command::Open { target, token } => open(tmux, &resolve_pane(tmux, &target)?, &token),
            Command::System { target, token } => {
                if is_url(&token) {
                    return opener::open(&token);
                }
                let pane = resolve_pane(tmux, &target)?;
                opener::open(&absolute(tmux, &pane, &token)?)
            }
            Command::Preview { target, token } => {
                if is_url(&token) || !crate::preview::is_markdown(Path::new(&token)) {
                    return Ok(());
                }
                let pane = resolve_pane(tmux, &target)?;
                crate::preview::open(&absolute(tmux, &pane, &token)?)
            }
            Command::Copy { token } => {
                tmux.run(&["set-buffer", "-w", "--", &token])?;
                Ok(())
            }
        }
    }
}

fn popup(tmux: &Tmux, pane: &str) -> Result<()> {
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

fn is_url(token: &str) -> bool {
    URL_SCHEMES.iter().any(|scheme| token.starts_with(scheme)) || token.starts_with("git@")
}

fn absolute(tmux: &Tmux, pane: &str, token: &str) -> Result<PathBuf> {
    Ok(expand(
        Path::new(&tmux.display(pane, "#{pane_current_path}")?),
        token,
    ))
}

fn open(tmux: &Tmux, pane: &str, token: &str) -> Result<()> {
    if is_url(token) {
        return opener::open(token);
    }
    let path = absolute(tmux, pane, token)?;
    let session = tmux.display(pane, "#{session_id}")?;
    let panes = tmux.run(&[
        "list-panes",
        "-s",
        "-t",
        &session,
        "-F",
        "#{pane_id} #{pane_current_command}",
    ])?;
    let nvim = panes
        .lines()
        .filter_map(|line| line.split_once(' '))
        .find(|(_, command)| *command == "nvim")
        .map(|(id, _)| id);
    let path_str = path.to_string_lossy();
    if let Some(nvim) = nvim {
        tmux.run(&["send-keys", "-t", nvim, "Escape"])?;
        tmux.run(&[
            "send-keys",
            "-t",
            nvim,
            "-l",
            &format!(":e {}", vim_escape(&path_str)),
        ])?;
        tmux.run(&["send-keys", "-t", nvim, "Enter"])?;
        tmux.run(&["select-window", "-t", nvim])?;
        tmux.run(&["select-pane", "-t", nvim])?;
        return Ok(());
    }
    if path.is_dir() {
        return opener::open(&path);
    }
    let root = tmux.display(pane, "#{session_path}")?;
    let root = if Path::new(&root).is_dir() {
        root
    } else {
        path.parent()
            .map_or_else(String::new, |dir| dir.to_string_lossy().into_owned())
    };
    tmux.run(&[
        "new-window",
        "-t",
        &format!("{session}:"),
        "-c",
        &root,
        "-n",
        "nvim",
        "--",
        "nvim",
        &path_str,
    ])?;
    Ok(())
}

fn vim_escape(path: &str) -> String {
    let mut escaped = String::with_capacity(path.len());
    for c in path.chars() {
        if " \t*?[{`$\\%#'\"|!<".contains(c) {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

fn resolve_pane(tmux: &Tmux, hint: &str) -> Result<String> {
    if hint.len() > 1 && hint.starts_with('%') && hint[1..].chars().all(|c| c.is_ascii_digit()) {
        return Ok(hint.to_string());
    }
    let pane = tmux.run(&["display-message", "-p", "#{pane_id}"])?;
    if pane.is_empty() {
        return Err("cannot resolve the current pane".into());
    }
    Ok(pane)
}

fn rows(tmux: &Tmux, pane: &str) -> Result<Vec<String>> {
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

fn candidates(capture: &str) -> (Vec<String>, Vec<String>) {
    let mut urls = Vec::new();
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    for line in capture.lines().rev() {
        for url in urls_in(line) {
            if url.chars().count() > 6 && seen.insert(url.clone()) {
                urls.push(url);
            }
        }
        for path in paths_in(line) {
            if path.chars().count() > 3 && seen.insert(path.clone()) {
                paths.push(path);
            }
        }
    }
    (urls, paths)
}

fn trim_punctuation(token: &str) -> &str {
    token.trim_end_matches(['.', ',', ':', ';'])
}

fn url_start(rest: &str) -> Option<usize> {
    if let Some(scheme) = URL_SCHEMES.iter().find(|s| rest.starts_with(**s)) {
        return Some(scheme.len());
    }
    let host = rest.strip_prefix("git@")?;
    let len = host
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')))
        .unwrap_or(host.len());
    (len > 0 && host[len..].starts_with(':')).then_some(4 + len + 1)
}

fn urls_in(line: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let mut at = 0;
    while at < line.len() {
        let rest = &line[at..];
        let Some(prefix) = url_start(rest) else {
            at += rest.chars().next().map_or(1, char::len_utf8);
            continue;
        };
        let tail = &rest[prefix..];
        let len = tail
            .find(|c: char| c.is_whitespace() || "<>\"'`|(),".contains(c))
            .unwrap_or(tail.len());
        if len == 0 {
            at += prefix;
            continue;
        }
        let url = trim_punctuation(&rest[..prefix + len]);
        if !url.contains('…') {
            urls.push(url.to_string());
        }
        at += prefix + len;
    }
    urls
}

fn is_path_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '~' | '@' | '+' | '-')
}

fn paths_in(line: &str) -> Vec<String> {
    line.split(|c: char| !(is_path_char(c) || c == '/'))
        .flat_map(split_on_double_slash)
        .filter_map(path_from_piece)
        .collect()
}

fn split_on_double_slash(run: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut start = 0;
    while let Some(found) = run[start..].find("//") {
        let at = start + found;
        pieces.push(&run[start..at]);
        let slashes = run[at..].len() - run[at..].trim_start_matches('/').len();
        start = at + slashes - 1;
    }
    pieces.push(&run[start..]);
    pieces
}

fn path_from_piece(piece: &str) -> Option<String> {
    let piece = trim_punctuation(piece.trim_end_matches('/'));
    piece
        .strip_prefix('/')
        .unwrap_or(piece)
        .contains('/')
        .then(|| piece.to_string())
}

fn expand(cwd: &Path, path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return Path::new(&home).join(rest);
        }
    }
    cwd.join(path)
}
