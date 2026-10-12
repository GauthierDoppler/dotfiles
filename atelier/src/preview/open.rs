use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

use super::{is_markdown, normalize, port, DEFAULT_PORT, LABEL};
use crate::service;
use crate::Result;

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

fn start_server(port: u16) -> Result<()> {
    use std::os::unix::process::CommandExt;
    if port == DEFAULT_PORT && service::installed(LABEL) {
        if service::start(LABEL) {
            return Ok(());
        }
        return Err(format!("server down and service {LABEL} did not start -- see atelier service list").into());
    }
    crate::process::command(std::env::current_exe()?)
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
    let focused = crate::process::command("osascript")
        .args(["-l", "JavaScript", "-e", FOCUS, url])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|out| out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "found");
    if focused {
        return;
    }
    let opened = crate::process::command("open")
        .args(["-a", "Google Chrome", url])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if !opened {
        let _ = crate::opener::open(url);
    }
}

#[cfg(not(target_os = "macos"))]
fn show(url: &str) {
    let _ = crate::opener::open(url);
}
