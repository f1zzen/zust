use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;

pub fn command(program: impl AsRef<OsStr>) -> Command {
    Command::new(program)
}

pub fn run(program: &str, args: &[&str]) -> bool {
    command(program)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn kill_process(pid: u32) {
    unsafe {
        libc::kill(pid as libc::pid_t, libc::SIGTERM);
    }
}

pub fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

/// Экранирование для sh: 'abc' → 'abc', it's → 'it'\''s'
pub fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

pub fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}
