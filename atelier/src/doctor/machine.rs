use super::check::{fail, Context, Outcome};

pub(super) fn setup(context: &Context) -> Outcome {
    let repo = match context.repo() {
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
                    .map(|path| context.tilde(path))
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

pub(super) fn keyboard_layout(context: &Context) -> Outcome {
    if !cfg!(target_os = "macos") {
        return Outcome::Skip("macOS only".into());
    }
    let bundle = context
        .home
        .join("Library/Keyboard Layouts/FR-AZERTY-num.bundle");
    if bundle.is_dir() {
        return Outcome::Ok("installed; selecting it is manual (Input Sources)".into());
    }
    fail(
        format!(
            "{} is missing, Prefix + 1..9 needs Shift",
            context.tilde(&bundle)
        ),
        "./install.sh copies it, then select it in System Settings > Keyboard > Input Sources",
    )
}
