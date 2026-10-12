use crate::process::{output as run, stdout_of};

use super::check::{fail, Context, Outcome};

pub(super) fn terminfo(context: &Context) -> Outcome {
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
    let host = stdout_of("uname", &["-n"])
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

pub(super) fn nerd_font(_: &Context) -> Outcome {
    Outcome::Look(
        "do \u{e0b6}  \u{e0b4} read as a left and a right half-round cap? \
         If not, select a Nerd Font in the terminal"
            .into(),
    )
}
