use std::io::{Read, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::tmux::Tmux;
use crate::Result;

use super::catalog::{self, Catalog};

enum Placement {
    Window,
    Popup,
    Delegated,
}

impl Placement {
    fn of(header: &str) -> Self {
        match header.split_whitespace().next() {
            None | Some("window") => Placement::Window,
            Some("popup") => Placement::Popup,
            Some(_) => Placement::Delegated,
        }
    }
}

pub fn place(tmux: &Tmux, catalog: &Catalog, name: &str, in_popup: bool) -> Result<()> {
    let script = catalog.script(name);
    if !catalog.names().iter().any(|known| known == name) {
        tmux.run(&["display-message", &format!("  no such task: {name}")])?;
        return Err(format!("no such task: {name}").into());
    }
    catalog.touch(name)?;
    match Placement::of(&catalog::header(&script, "tmux")) {
        Placement::Window => in_window(tmux, catalog, name),
        Placement::Popup if in_popup => Err(Command::new(legacy("tmux-task-run"))
            .arg(&catalog.root)
            .arg(name)
            .arg(&script)
            .arg("0")
            .arg(catalog::header(&script, "close"))
            .exec()
            .into()),
        Placement::Popup | Placement::Delegated => {
            let status = Command::new(legacy("tmux-tasks"))
                .args(["--run", name])
                .status()?;
            if status.success() {
                Ok(())
            } else {
                Err(format!("tmux-tasks --run {name}: {status}").into())
            }
        }
    }
}

fn legacy(script: &str) -> PathBuf {
    let home = std::env::var_os("HOME").unwrap_or_default();
    Path::new(&home).join(".local/bin").join(script)
}

fn in_window(tmux: &Tmux, catalog: &Catalog, name: &str) -> Result<()> {
    let slug = name.replace('/', "-");
    let exe = std::env::current_exe()?;
    let root = path_arg(&catalog.root)?;
    let exe = path_arg(&exe)?;
    let wrapper = [exe, "tasks", "exec", "--mark", root, name];
    let windows = tmux.run(&[
        "list-windows",
        "-t",
        &catalog.session,
        "-F",
        "#{window_id} #{window_name}",
    ])?;
    let existing = windows
        .lines()
        .filter_map(|line| line.split_once(' '))
        .find(|(_, window_name)| *window_name == slug)
        .map(|(id, _)| id.to_string());
    match existing {
        Some(window) => {
            let mut respawn = vec!["respawn-window", "-k", "-t", &window, "-c", root, "--"];
            respawn.extend(wrapper);
            tmux.run(&respawn)?;
            tmux.run(&["select-window", "-t", &window])?;
        }
        None => {
            let at = format!("{}:", catalog.session);
            let mut create = vec!["new-window", "-t", &at, "-n", &slug, "-c", root, "--"];
            create.extend(wrapper);
            tmux.run(&create)?;
        }
    }
    Ok(())
}

fn path_arg(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| format!("not a UTF-8 path: {}", path.display()).into())
}

pub fn execute(tmux: &Tmux, root: &Path, name: &str, mark: bool) -> Result<()> {
    let window = if mark {
        std::env::var("TMUX_PANE")
            .ok()
            .and_then(|pane| tmux.display(&pane, "#{window_id}").ok())
            .filter(|window| !window.is_empty())
    } else {
        None
    };
    let set_status = |status: &str| {
        if let Some(window) = &window {
            let _ = tmux.run(&["set-option", "-w", "-t", window, "@task_status", status]);
        }
    };
    set_status("running");

    let script = root.join(".tmux").join(name);
    let outcome = if root.is_dir() {
        Command::new(&script)
            .current_dir(root)
            .env("TMUX_TASK_ROOT", root)
            .env("TMUX_TASK_NAME", name)
            .status()
            .map(|status| status.code().unwrap_or(128 + status.signal().unwrap_or(0)))
            .map_err(|error| format!("cannot run {}: {error}", script.display()))
    } else {
        Err(format!("cannot cd to {}", root.display()))
    };

    if outcome == Ok(0) && catalog::header(&script, "close") == "ok" {
        return Ok(());
    }
    let mut out = std::io::stdout();
    match &outcome {
        Ok(0) => {
            set_status("ok");
            write!(out, "\n\x1b[32m✓ {name}\x1b[0m")?;
        }
        Ok(code) => {
            set_status("fail");
            write!(out, "\n\x1b[31m✗ {name} — exit {code}\x1b[0m")?;
        }
        Err(error) => {
            set_status("fail");
            write!(out, "\n\x1b[31m{error}\x1b[0m")?;
        }
    }
    if let Some(window) = &window {
        if !watched(tmux, window) {
            write!(out, "\x07")?;
        }
    }
    write!(out, "  ·  any key to close")?;
    out.flush()?;
    wait_for_key();
    Ok(())
}

fn watched(tmux: &Tmux, window: &str) -> bool {
    let Ok(active) = tmux.display(window, "#{window_active}") else {
        return false;
    };
    let Ok(session) = tmux.display(window, "#{session_id}") else {
        return false;
    };
    let Ok(clients) = tmux.run(&[
        "list-clients",
        "-F",
        "#{client_control_mode} #{session_id}",
    ]) else {
        return false;
    };
    active == "1" && clients.lines().any(|client| client == format!("0 {session}"))
}

fn wait_for_key() {
    let stty = |args: &[&str]| {
        Command::new("stty")
            .args(args)
            .stdin(Stdio::inherit())
            .stderr(Stdio::null())
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
    };
    let saved = stty(&["-g"]);
    stty(&["-icanon", "-echo", "min", "1"]);
    let _ = std::io::stdin().read(&mut [0u8]);
    if let Some(saved) = saved {
        stty(&[&saved]);
    }
}
