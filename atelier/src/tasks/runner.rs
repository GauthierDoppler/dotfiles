use std::io::{Read, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::fzf;
use crate::shell;
use crate::tmux::Tmux;
use crate::Result;

use super::catalog::{self, Catalog};

enum Placement<'a> {
    Window,
    Detach,
    Popup,
    Split { flag: &'static str, size: &'a str },
}

impl<'a> Placement<'a> {
    fn of(header: &'a str) -> Self {
        let mut words = header.split_whitespace();
        let placement = words.next();
        let size = words.next().unwrap_or("30%");
        match placement {
            Some("popup") => Placement::Popup,
            Some("detach") => Placement::Detach,
            Some("split" | "split-down") => Placement::Split { flag: "-v", size },
            Some("split-right") => Placement::Split { flag: "-h", size },
            _ => Placement::Window,
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
        Placement::Window => in_window(tmux, catalog, name, true),
        Placement::Detach => in_window(tmux, catalog, name, false),
        Placement::Split { flag, size } => in_split(tmux, catalog, name, flag, size),
        Placement::Popup if in_popup => execute(tmux, &catalog.root, name, false),
        Placement::Popup => in_popup_of_its_own(tmux, catalog, name),
    }
}

fn in_window(tmux: &Tmux, catalog: &Catalog, name: &str, select: bool) -> Result<()> {
    let slug = name.replace('/', "-");
    let root = path_arg(&catalog.root)?;
    let wrapper = wrapper(tmux, catalog, name, true)?;
    let wrapper = wrapper.iter().map(String::as_str);
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
            let respawn = if select {
                "respawn-window"
            } else {
                "respawn-pane"
            };
            let mut respawn = vec![respawn, "-k", "-t", &window, "-c", root, "--"];
            respawn.extend(wrapper);
            tmux.run(&respawn)?;
            if select {
                tmux.run(&["select-window", "-t", &window])?;
            }
        }
        None => {
            let at = format!("{}:", catalog.session);
            let mut create = vec!["new-window", "-t", &at, "-n", &slug, "-c", root];
            if !select {
                create.push("-d");
            }
            create.push("--");
            create.extend(wrapper);
            tmux.run(&create)?;
        }
    }
    Ok(())
}

fn in_split(tmux: &Tmux, catalog: &Catalog, name: &str, flag: &str, size: &str) -> Result<()> {
    let wrapper = wrapper(tmux, catalog, name, false)?;
    let root = path_arg(&catalog.root)?;
    let mut split = vec![
        "split-window",
        flag,
        "-l",
        size,
        "-t",
        &catalog.session,
        "-c",
        root,
        "--",
    ];
    split.extend(wrapper.iter().map(String::as_str));
    tmux.run(&split)?;
    Ok(())
}

fn in_popup_of_its_own(tmux: &Tmux, catalog: &Catalog, name: &str) -> Result<()> {
    let command: Vec<String> = wrapper(tmux, catalog, name, false)?
        .iter()
        .map(|word| shell::quote(word))
        .collect();
    tmux.command()
        .args(["display-popup", "-E", "-w", "80%", "-h", "60%"])
        .args(["-t", &catalog.session, "-d", path_arg(&catalog.root)?])
        .arg(command.join(" "))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

fn wrapper(tmux: &Tmux, catalog: &Catalog, name: &str, mark: bool) -> Result<Vec<String>> {
    let mut command = fzf::atelier_argv(tmux)?;
    command.extend(["tasks".to_string(), "exec".to_string()]);
    if mark {
        command.push("--mark".to_string());
    }
    command.push(path_arg(&catalog.root)?.to_string());
    command.push(name.to_string());
    Ok(command)
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
        if !tmux.watched(window).unwrap_or(false) {
            write!(out, "\x07")?;
        }
    }
    write!(out, "  ·  any key to close")?;
    out.flush()?;
    wait_for_key();
    Ok(())
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
