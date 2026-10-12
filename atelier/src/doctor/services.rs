use crate::service;

use super::check::{fail, no_server, Context, Outcome};

pub(super) fn daemon(context: &Context) -> Outcome {
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

pub(super) fn services(context: &Context) -> Outcome {
    if let Some((problem, fix)) = service::agents_dir_problem(context.target) {
        return fail(problem, fix);
    }
    let health = match service::health(context.target) {
        Ok(health) => health,
        Err(error) => return fail(error.to_string(), "atelier service install"),
    };
    let problems: Vec<String> = [("not running", health.stopped), ("outdated", health.outdated)]
        .into_iter()
        .filter(|(_, labels)| !labels.is_empty())
        .map(|(problem, labels)| format!("{problem}: {}", labels.join(", ")))
        .collect();
    if problems.is_empty() {
        return Outcome::Ok("all running".into());
    }
    fail(problems.join("; "), "atelier service install")
}

pub(super) fn linger(context: &Context) -> Outcome {
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
