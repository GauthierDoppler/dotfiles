use super::check::{fail, no_server, Context, Outcome};
use super::version::{dotted, version};

const TMUX_MINIMUM: (u32, u32) = (3, 3);

pub(super) fn tmux_version(context: &Context) -> Outcome {
    let Some(text) = context.tmux.version() else {
        return fail("tmux not found", install_tmux());
    };
    let reported = text.strip_prefix("tmux ").unwrap_or(&text).to_owned();
    match version(&reported) {
        None => Outcome::Look(format!(
            "cannot read a version from {text:?}; atelier needs {} or later",
            dotted(TMUX_MINIMUM)
        )),
        Some(found) if found < TMUX_MINIMUM => fail(
            format!(
                "{reported}, atelier needs {} or later (allow-passthrough, pane-border-indicators)",
                dotted(TMUX_MINIMUM)
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
            dotted(TMUX_MINIMUM)
        )
    }
}

fn server_option(context: &Context, option: &str) -> Option<String> {
    context
        .server
        .then(|| context.tmux.run(&["show-options", "-sv", option]).ok())
        .flatten()
}

pub(super) fn extended_keys(context: &Context) -> Outcome {
    match server_option(context, "extended-keys").as_deref() {
        None => no_server(),
        Some(value @ ("on" | "always")) => Outcome::Ok(value.into()),
        Some(value) => fail(
            format!("extended-keys is {value}, Shift+Enter cannot reach applications"),
            "tmux set -s extended-keys on, and check ~/.tmux.conf links to the dotfiles",
        ),
    }
}

pub(super) fn terminal_features(context: &Context) -> Outcome {
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
