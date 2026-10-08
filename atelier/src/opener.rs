use std::ffi::OsStr;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

use crate::Result;

#[cfg(target_os = "macos")]
const OPENER: &str = "open";
#[cfg(not(target_os = "macos"))]
const OPENER: &str = "xdg-open";

pub fn open(target: impl AsRef<OsStr>) -> Result<()> {
    Command::new(OPENER)
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|error| format!("{OPENER}: {error}"))?;
    Ok(())
}
