use std::ffi::OsStr;
use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

#[allow(clippy::disallowed_methods)]
pub fn command(program: impl AsRef<OsStr>) -> Command {
    Command::new(program)
}

pub fn output<S: AsRef<OsStr>>(program: impl AsRef<OsStr>, args: &[S]) -> Option<Output> {
    command(program)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .ok()
}

pub fn output_within(command: &mut Command, limit: Duration) -> Option<Output> {
    let mut child = command.stdout(Stdio::piped()).spawn().ok()?;
    let mut pipe = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut stdout = Vec::new();
        let _ = pipe.read_to_end(&mut stdout);
        stdout
    });
    let deadline = Instant::now() + limit;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    Some(Output {
        status,
        stdout: reader.join().ok()?,
        stderr: Vec::new(),
    })
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

pub fn stdout_of<S: AsRef<OsStr>>(program: impl AsRef<OsStr>, args: &[S]) -> Option<String> {
    output(program, args).map(|output| stdout(&output))
}

pub fn succeeds<S: AsRef<OsStr>>(program: impl AsRef<OsStr>, args: &[S]) -> bool {
    command(program)
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
