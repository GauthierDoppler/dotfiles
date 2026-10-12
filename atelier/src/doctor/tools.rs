use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::process::{output as run, stdout};

use super::check::{fail, Context, Outcome};
use super::version::{dotted, version};

const FZF_MINIMUM: (u32, u32) = (0, 45);

pub(super) fn fzf(_: &Context) -> Outcome {
    let fix = if cfg!(target_os = "macos") {
        "brew install fzf".to_owned()
    } else {
        format!(
            "install fzf {} or later from github.com/junegunn/fzf/releases",
            dotted(FZF_MINIMUM)
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
                dotted(FZF_MINIMUM)
            ),
            fix,
        ),
    }
}

pub(super) fn installed(context: &Context) -> Outcome {
    let binary = context.home.join(".local/bin/atelier");
    let crate_dir = context
        .repo()
        .map(|repo| repo.join("atelier"))
        .unwrap_or_else(|_| context.home.join("dotfiles/atelier"));
    let install = format!(
        "cargo install --locked --root ~/.local --path {}",
        crate_dir.display()
    );
    let meta = std::fs::metadata(&binary)
        .ok()
        .filter(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0);
    let Some(meta) = meta else {
        return fail(
            format!(
                "{} is missing, and tmux calls atelier by that path",
                binary.display()
            ),
            install,
        );
    };
    let sources = ["src", "assets", "Cargo.toml", "Cargo.lock"]
        .iter()
        .filter_map(|part| newest(&crate_dir.join(part)))
        .max();
    match (meta.modified().ok(), sources) {
        (Some(built), Some(edited)) if built < edited => fail(
            format!(
                "{} is older than its sources in {}",
                context.tilde(&binary),
                context.tilde(&crate_dir)
            ),
            install,
        ),
        _ => Outcome::Ok(binary.display().to_string()),
    }
}

fn newest(path: &Path) -> Option<std::time::SystemTime> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_dir() {
        return meta.modified().ok();
    }
    std::fs::read_dir(path)
        .ok()?
        .filter_map(|entry| newest(&entry.ok()?.path()))
        .max()
}
