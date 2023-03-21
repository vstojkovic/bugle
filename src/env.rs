use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;

pub fn current_exe_dir() -> Result<PathBuf> {
    let exe_path = std::env::current_exe()?;
    exe_path
        .parent()
        .map(ToOwned::to_owned)
        .ok_or_else(|| Error::new(ErrorKind::Other, "Malformed executable path"))
}
