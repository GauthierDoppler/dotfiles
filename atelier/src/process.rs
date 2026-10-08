use std::ffi::OsStr;
use std::process::{Command, Output, Stdio};

pub fn output<S: AsRef<OsStr>>(program: impl AsRef<OsStr>, args: &[S]) -> Option<Output> {
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .ok()
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

pub fn stdout_of<S: AsRef<OsStr>>(program: impl AsRef<OsStr>, args: &[S]) -> Option<String> {
    output(program, args).map(|output| stdout(&output))
}

pub fn succeeds<S: AsRef<OsStr>>(program: impl AsRef<OsStr>, args: &[S]) -> bool {
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

pub fn uid() -> u32 {
    unsafe { libc::getuid() }
}
