mod open;
mod picker;
mod server;

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

use clap::Subcommand;
use percent_encoding::percent_decode_str;

use crate::tmux::Tmux;
use crate::Result;

pub use open::open;

const DEFAULT_PORT: u16 = 33440;
const LABEL: &str = "com.github.gauthierdoppler.md-preview";

#[derive(Subcommand)]
pub enum Command {
    /// Run the preview server on 127.0.0.1 (what the service runs)
    Serve,
    /// Print the session's markdown files, newest first, relative to its root
    List {
        #[arg(short = 't', long, value_name = "SESSION")]
        target: Option<String>,
    },
    /// Pick a markdown file of the session in fzf and preview it; for `display-popup -E`
    Pick {
        #[arg(short = 't', long, value_name = "SESSION")]
        target: Option<String>,
    },
    #[command(external_subcommand)]
    Open(Vec<OsString>),
}

impl Command {
    pub fn run(self, tmux: &Tmux) -> Result<()> {
        match self {
            Command::Serve => server::serve(port()),
            Command::List { target } => {
                for row in picker::rows(&picker::session_root(tmux, target.as_deref())?) {
                    println!("{row}");
                }
                Ok(())
            }
            Command::Pick { target } => picker::pick(tmux, target.as_deref()),
            Command::Open(args) => match args.as_slice() {
                [file] if file == picker::PLACEHOLDER => Ok(()),
                [file] => open(Path::new(file)),
                _ => Err("usage: atelier preview <file.md> | atelier preview serve".into()),
            },
        }
    }
}

fn port() -> u16 {
    std::env::var("MD_PREVIEW_PORT")
        .ok()
        .and_then(|port| port.parse().ok())
        .unwrap_or(DEFAULT_PORT)
}

pub fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
}

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::from("/");
    for component in path.components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::ParentDir => {
                out.pop();
            }
            Component::RootDir | Component::CurDir | Component::Prefix(_) => {}
        }
    }
    out
}

fn path_from_url(encoded: &str) -> Option<PathBuf> {
    let decoded = percent_decode_str(encoded).decode_utf8().ok()?;
    Some(normalize(Path::new(decoded.as_ref())))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_label_names_the_service_that_runs_the_preview_server() {
        let label = format!("label = \"{}\"", super::LABEL);
        let service = include_str!("../../services.toml")
            .split("[[service]]")
            .find(|block| block.contains(&label))
            .expect("services.toml declares the preview's label");
        assert!(service.contains(r#""preview", "serve""#), "{service}");
        assert!(service.contains(&format!("127.0.0.1:{}", super::DEFAULT_PORT)));
    }
}
