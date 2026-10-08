use std::io::Read;

use clap::Subcommand;

use crate::tmux::Tmux;
use crate::Result;

#[derive(Subcommand)]
pub enum Command {
    /// Claude Code hook: mark its window waiting or done, from the payload on stdin
    Claude,
}

impl Command {
    pub fn run(self, tmux: &Tmux) -> Result<()> {
        match self {
            Command::Claude => {
                let _ = claude(tmux);
                Ok(())
            }
        }
    }
}

fn claude(tmux: &Tmux) -> Result<()> {
    let mut payload = String::new();
    std::io::stdin().read_to_string(&mut payload)?;
    let Ok(pane) = std::env::var("TMUX_PANE") else {
        return Ok(());
    };
    let window = tmux.display(&pane, "#{window_id}")?;
    if window.is_empty() {
        return Ok(());
    }
    let payload: serde_json::Value = serde_json::from_str(&payload)?;
    let status = match payload["hook_event_name"].as_str() {
        Some("Notification") => "waiting",
        Some("Stop") => "done",
        Some("UserPromptSubmit") => {
            tmux.run(&["set-option", "-uw", "-t", &window, "@claude_status"])?;
            return Ok(());
        }
        _ => return Ok(()),
    };
    if watched(tmux, &window)? {
        return Ok(());
    }
    tmux.run(&["set-option", "-w", "-t", &window, "@claude_status", status])?;
    Ok(())
}

fn watched(tmux: &Tmux, window: &str) -> Result<bool> {
    let clients = tmux.run(&["list-clients", "-F", "#{client_control_mode} #{window_id}"])?;
    Ok(clients
        .lines()
        .any(|client| client.strip_prefix("0 ") == Some(window)))
}
