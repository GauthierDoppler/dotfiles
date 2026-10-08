mod picker;
mod server;

use std::ffi::OsString;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Component, Path, PathBuf};
use std::process::{Command as Process, Stdio};
use std::time::Duration;

use clap::Subcommand;
use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

use crate::tmux::Tmux;
use crate::Result;

const DEFAULT_PORT: u16 = 33440;
const LABEL: &str = "com.github.gauthierdoppler.md-preview";

const URI_COMPONENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'!')
    .remove(b'~')
    .remove(b'*')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')');

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

fn url_path(file: &Path) -> String {
    file.to_string_lossy()
        .split('/')
        .map(|segment| utf8_percent_encode(segment, URI_COMPONENT).to_string())
        .collect::<Vec<_>>()
        .join("/")
}

pub fn open(arg: &Path) -> Result<()> {
    let file = normalize(&std::env::current_dir()?.join(arg));
    if !file.exists() {
        return Err(format!("no such file: {}", file.display()).into());
    }
    if !is_markdown(&file) {
        return Err(format!("not a markdown file: {}", file.display()).into());
    }
    let port = port();
    ensure_server(port)?;
    let url = format!("http://127.0.0.1:{port}{}", url_path(&file));
    show(&url);
    println!("{url}");
    Ok(())
}

fn alive(port: u16) -> bool {
    let probe = || -> std::io::Result<bool> {
        let timeout = Duration::from_millis(500);
        let mut stream = TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), timeout)?;
        stream.set_read_timeout(Some(timeout))?;
        write!(
            stream,
            "GET /__meta HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
        )?;
        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        Ok(response.contains(r#""app":"md-preview""#))
    };
    probe().unwrap_or(false)
}

fn ensure_server(port: u16) -> Result<()> {
    if alive(port) {
        return Ok(());
    }
    start_server(port)?;
    for _ in 0..30 {
        std::thread::sleep(Duration::from_millis(100));
        if alive(port) {
            return Ok(());
        }
    }
    Err(format!("server did not come up on port {port}").into())
}

#[cfg(target_os = "macos")]
fn launchd_domain() -> String {
    format!("gui/{}/{LABEL}", unsafe { libc::getuid() })
}

#[cfg(target_os = "macos")]
fn start_server(_port: u16) -> Result<()> {
    let kicked = Process::new("/bin/launchctl")
        .args(["kickstart", &launchd_domain()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?
        .success();
    if !kicked {
        return Err(
            format!("server down and launchd agent {LABEL} not loaded -- run atelier service install").into(),
        );
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn start_server(port: u16) -> Result<()> {
    use std::os::unix::process::CommandExt;
    let started = port == DEFAULT_PORT
        && Process::new("systemctl")
            .args(["--user", "start", &format!("{LABEL}.service")])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
    if started {
        return Ok(());
    }
    Process::new(std::env::current_exe()?)
        .args(["preview", "serve"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()?;
    Ok(())
}

#[cfg(target_os = "macos")]
const FOCUS: &str = r#"
function run([url]) {
  const chrome = Application('Google Chrome')
  if (!chrome.running()) return 'none'
  for (const w of chrome.windows()) {
    const i = w.tabs.url().findIndex((u) => u.split('#')[0] === url)
    if (i === -1) continue
    w.activeTabIndex = i + 1
    w.index = 1
    chrome.activate()
    return 'found'
  }
  return 'none'
}"#;

#[cfg(target_os = "macos")]
fn show(url: &str) {
    let focused = Process::new("/usr/bin/osascript")
        .args(["-l", "JavaScript", "-e", FOCUS, url])
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|out| out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "found");
    if focused {
        return;
    }
    let opened = Process::new("/usr/bin/open")
        .args(["-a", "Google Chrome", url])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if !opened {
        let _ = Process::new("/usr/bin/open").arg(url).status();
    }
}

#[cfg(not(target_os = "macos"))]
fn show(url: &str) {
    let _ = crate::opener::open(url);
}
