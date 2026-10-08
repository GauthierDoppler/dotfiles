use std::ffi::OsStr;
use std::process::{Command, Stdio};

use crate::Result;

#[cfg(target_os = "macos")]
const OPENER: &str = "open";
#[cfg(not(target_os = "macos"))]
const OPENER: &str = "xdg-open";

pub fn open(target: impl AsRef<OsStr>) -> Result<()> {
    let status = Command::new(OPENER)
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| format!("{OPENER}: {error}"))?;
    if !status.success() {
        return Err(format!("{OPENER} exited with {status}").into());
    }
    Ok(())
}
