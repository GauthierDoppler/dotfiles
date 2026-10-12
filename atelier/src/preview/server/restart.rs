use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::stamp;
use crate::preview::LABEL;

pub(super) struct Binary {
    exe: PathBuf,
    boot: Option<(SystemTime, u64)>,
    restarting: bool,
}

impl Binary {
    pub(super) fn new(exe: PathBuf) -> Self {
        let boot = stamp(&exe);
        Binary {
            exe,
            boot,
            restarting: false,
        }
    }

    pub(super) fn restart_if_replaced(&mut self) {
        if !self.restarting {
            if let Some(current) = stamp(&self.exe) {
                if Some(current) != self.boot {
                    self.restarting = restart(&self.exe);
                }
            }
        }
    }
}

fn restart(exe: &Path) -> bool {
    if cfg!(target_os = "macos") && std::env::var("XPC_SERVICE_NAME").as_deref() == Ok(LABEL) {
        return crate::service::restart(LABEL);
    }
    let error = crate::process::command(exe).args(["preview", "serve"]).exec();
    eprintln!("md-preview: cannot restart from {}: {error}", exe.display());
    false
}
