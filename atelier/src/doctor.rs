use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command as Process, Output, Stdio};

use clap::Args;

use crate::service::{self, Target};
use crate::tmux::Tmux;
use crate::Result;

const TMUX_MINIMUM: (u32, u32) = (3, 3);
const FZF_MINIMUM: (u32, u32) = (0, 45);

#[derive(Args)]
pub struct Command {
    #[command(flatten)]
    target: Target,
}

crate::flags_only!(Command);

struct Context<'a> {
    tmux: &'a Tmux,
    server: bool,
    home: PathBuf,
    target: &'a Target,
}

enum Outcome {
    Ok(String),
    Fail { problem: String, fix: String },
    Look(String),
    Skip(String),
}

fn fail(problem: impl Into<String>, fix: impl Into<String>) -> Outcome {
    Outcome::Fail {
        problem: problem.into(),
        fix: fix.into(),
    }
}

type Check = fn(&Context) -> Outcome;

const CHECKS: &[(&str, Check)] = &[
    ("terminfo", terminfo),
    ("nerd font", nerd_font),
    ("tmux version", tmux_version),
    ("extended keys", extended_keys),
    ("terminal features", terminal_features),
    ("fzf", fzf),
    ("atelier installed", installed),
    ("setup", setup),
    ("daemon", daemon),
    ("services", services),
    ("linger", linger),
];

impl Command {
    pub fn run(self, tmux: &Tmux) -> Result<()> {
        let context = Context {
            tmux,
            server: tmux.run(&["display-message", "-p", "#{pid}"]).is_ok(),
            home: PathBuf::from(std::env::var_os("HOME").ok_or("HOME is not set")?),
            target: &self.target,
        };
        let mut failed = 0;
        for (name, check) in CHECKS {
            match check(&context) {
                Outcome::Ok(detail) => println!("ok    {name:<18} {detail}"),
                Outcome::Skip(why) => println!("skip  {name:<18} {why}"),
                Outcome::Look(what) => println!("look  {name:<18} {what}"),
                Outcome::Fail { problem, fix } => {
                    failed += 1;
                    println!("FAIL  {name:<18} {problem}");
                    println!("      fix: {fix}");
                }
            }
        }
        match failed {
            0 => Ok(()),
            1 => Err("1 check failed".into()),
            n => Err(format!("{n} checks failed").into()),
        }
    }
}

fn run(program: &str, args: &[&str]) -> Option<Output> {
    Process::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .ok()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn version(text: &str) -> Option<(u32, u32)> {
    let digits = text.trim_start_matches(|c: char| !c.is_ascii_digit());
    let mut parts = digits.split(|c: char| !c.is_ascii_digit());
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

fn shown((major, minor): (u32, u32)) -> String {
    format!("{major}.{minor}")
}

fn terminfo(context: &Context) -> Outcome {
    let mut terms: Vec<String> = std::env::var("TERM")
        .ok()
        .filter(|term| !term.is_empty())
        .into_iter()
        .collect();
    if context.server {
        let clients = context
            .tmux
            .run(&[
                "list-clients",
                "-F",
                "#{client_control_mode} #{client_termname}",
            ])
            .unwrap_or_default();
        for term in clients.lines().filter_map(|line| line.strip_prefix("0 ")) {
            if !term.is_empty() && !terms.iter().any(|known| known == term) {
                terms.push(term.to_owned());
            }
        }
    }
    if terms.is_empty() {
        return Outcome::Skip("TERM is not set".into());
    }
    let mut missing = Vec::new();
    for term in &terms {
        match run("infocmp", &[term]) {
            None => return Outcome::Skip("infocmp not found".into()),
            Some(output) if !output.status.success() => missing.push(term.as_str()),
            Some(_) => {}
        }
    }
    if missing.is_empty() {
        return Outcome::Ok(terms.join(", "));
    }
    let host = run("uname", &["-n"])
        .map(|output| stdout(&output))
        .filter(|host| !host.is_empty())
        .unwrap_or_else(|| "<this host>".into());
    let fix = missing
        .iter()
        .map(|term| format!("infocmp -x {term} | ssh {host} tic -x -"))
        .collect::<Vec<_>>()
        .join("; ");
    fail(
        format!(
            "no terminfo for {}, run the fix from a machine that has it",
            missing.join(", ")
        ),
        fix,
    )
}

fn nerd_font(_: &Context) -> Outcome {
    Outcome::Look(
        "do \u{e0b6}  \u{e0b4} read as a left and a right half-round cap? \
         If not, select a Nerd Font in the terminal"
            .into(),
    )
}

fn tmux_version(_: &Context) -> Outcome {
    let Some(output) = run("tmux", &["-V"]) else {
        return fail("tmux not found", install_tmux());
    };
    let text = stdout(&output);
    let reported = text.strip_prefix("tmux ").unwrap_or(&text).to_owned();
    match version(&reported) {
        None => Outcome::Look(format!(
            "cannot read a version from {text:?}; atelier needs {} or later",
            shown(TMUX_MINIMUM)
        )),
        Some(found) if found < TMUX_MINIMUM => fail(
            format!(
                "{reported}, atelier needs {} or later (allow-passthrough, pane-border-indicators)",
                shown(TMUX_MINIMUM)
            ),
            install_tmux(),
        ),
        Some(_) => Outcome::Ok(reported),
    }
}

fn install_tmux() -> String {
    if cfg!(target_os = "macos") {
        "brew install tmux".into()
    } else {
        format!(
            "install tmux {} or later; distribution packages can be older than that",
            shown(TMUX_MINIMUM)
        )
    }
}

fn server_option(context: &Context, option: &str) -> Option<String> {
    context
        .server
        .then(|| context.tmux.run(&["show-options", "-sv", option]).ok())
        .flatten()
}

fn no_server() -> Outcome {
    Outcome::Skip("no tmux server to ask".into())
}

fn extended_keys(context: &Context) -> Outcome {
    match server_option(context, "extended-keys").as_deref() {
        None => no_server(),
        Some(value @ ("on" | "always")) => Outcome::Ok(value.into()),
        Some(value) => fail(
            format!("extended-keys is {value}, Shift+Enter cannot reach applications"),
            "tmux set -s extended-keys on, and check ~/.tmux.conf links to the dotfiles",
        ),
    }
}

fn terminal_features(context: &Context) -> Outcome {
    let Some(features) = server_option(context, "terminal-features") else {
        return no_server();
    };
    let mut seen = Vec::new();
    let mut duplicated = Vec::new();
    for feature in features.lines() {
        if seen.contains(&feature) {
            if !duplicated.contains(&feature) {
                duplicated.push(feature);
            }
        } else {
            seen.push(feature);
        }
    }
    if duplicated.is_empty() {
        return Outcome::Ok(format!("{} entries", seen.len()));
    }
    fail(
        format!(
            "duplicated by appending reloads: {}",
            duplicated.join(", ")
        ),
        "tmux set -gu terminal-features && tmux source-file ~/.tmux.conf",
    )
}

fn fzf(_: &Context) -> Outcome {
    let fix = if cfg!(target_os = "macos") {
        "brew install fzf".to_owned()
    } else {
        format!(
            "install fzf {} or later from github.com/junegunn/fzf/releases",
            shown(FZF_MINIMUM)
        )
    };
    let Some(output) = run("fzf", &["--version"]) else {
        return fail("not found, the pickers need it", fix);
    };
    let text = stdout(&output);
    let reported = text.split_whitespace().next().unwrap_or_default().to_owned();
    match version(&reported) {
        Some(found) if found >= FZF_MINIMUM => Outcome::Ok(reported),
        _ => fail(
            format!(
                "{reported}, the pickers need {} or later (transform)",
                shown(FZF_MINIMUM)
            ),
            fix,
        ),
    }
}

fn repo(context: &Context) -> Result<PathBuf> {
    crate::setup::repo_root(context.target.repo(), &context.home)
}

fn installed(context: &Context) -> Outcome {
    let binary = context.home.join(".local/bin/atelier");
    let executable = std::fs::metadata(&binary)
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0);
    if executable {
        return Outcome::Ok(binary.display().to_string());
    }
    let crate_dir = repo(context)
        .map(|repo| repo.join("atelier"))
        .unwrap_or_else(|_| context.home.join("dotfiles/atelier"));
    fail(
        format!(
            "{} is missing, and tmux calls atelier by that path",
            binary.display()
        ),
        format!(
            "cargo install --locked --root ~/.local --path {}",
            crate_dir.display()
        ),
    )
}

fn setup(context: &Context) -> Outcome {
    let repo = match repo(context) {
        Ok(repo) => repo,
        Err(error) => return fail(error.to_string(), "clone the dotfiles to ~/dotfiles"),
    };
    match crate::setup::pending(&repo, &context.home) {
        Err(error) => fail(error.to_string(), "atelier setup"),
        Ok(paths) if paths.is_empty() => Outcome::Ok("links and stubs in place".into()),
        Ok(paths) => fail(
            format!(
                "out of date: {}{}",
                paths
                    .iter()
                    .take(3)
                    .map(|path| tilde(path, &context.home))
                    .collect::<Vec<_>>()
                    .join(", "),
                match paths.len() {
                    0..=3 => String::new(),
                    n => format!(" and {} more", n - 3),
                }
            ),
            "atelier setup",
        ),
    }
}

fn tilde(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(relative) => format!("~/{}", relative.display()),
        Err(_) => path.display().to_string(),
    }
}

fn daemon(context: &Context) -> Outcome {
    if !context.server {
        return no_server();
    }
    match crate::daemon::answering(context.tmux) {
        Some(pid) => Outcome::Ok(format!("pid {pid}")),
        None => fail(
            "not running for this tmux server",
            "~/.local/bin/atelier daemon --ensure",
        ),
    }
}

fn services(context: &Context) -> Outcome {
    if let Some((problem, fix)) = service::agents_dir_problem(context.target) {
        return fail(problem, fix);
    }
    match service::stopped(context.target) {
        Err(error) => fail(error.to_string(), "atelier service install"),
        Ok(stopped) if stopped.is_empty() => Outcome::Ok("all running".into()),
        Ok(stopped) => fail(
            format!("not running: {}", stopped.join(", ")),
            "atelier service install",
        ),
    }
}

fn linger(context: &Context) -> Outcome {
    match service::linger(context.target) {
        None => Outcome::Skip("launchd keeps agents across logout".into()),
        Some(Err(error)) => fail(error.to_string(), "loginctl enable-linger $USER"),
        Some(Ok((user, true))) => Outcome::Ok(format!("enabled for {user}")),
        Some(Ok((user, false))) => fail(
            format!("off for {user}, services stop at logout"),
            format!("loginctl enable-linger {user}"),
        ),
    }
}
