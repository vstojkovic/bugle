use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;
use std::process::Child;

pub fn current_exe_dir() -> Result<PathBuf> {
    let exe_path = std::env::current_exe()?;
    exe_path
        .parent()
        .map(ToOwned::to_owned)
        .ok_or_else(|| Error::new(ErrorKind::Other, "Malformed executable path"))
}

pub fn restart_process() -> Result<Child> {
    let mut args = std::env::args_os();
    let mut cmd = std::process::Command::new(args.next().unwrap());
    cmd.args(args);
    cmd.spawn()
}
