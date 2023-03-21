use slog::{o, Discard, Drain, Logger};

#[cfg(not(windows))]
pub fn create_root_logger() -> Logger {
    create_term_logger()
}

#[cfg(windows)]
pub fn create_root_logger() -> Logger {
    if unsafe { winapi::um::wincon::AttachConsole(u32::MAX) } != 0 {
        create_term_logger()
    } else {
        try_create_portable_mode_logger()
            .or_else(|_| try_create_appdata_logger())
            .unwrap_or_else(|_| create_discard_logger())
    }
}

fn create_term_logger() -> Logger {
    let drain = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(drain).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    Logger::root(drain, o!())
}

fn create_discard_logger() -> Logger {
    Logger::root(Discard, o!())
}

#[cfg(windows)]
fn try_create_portable_mode_logger() -> anyhow::Result<Logger> {
    use crate::env::current_exe_dir;

    try_create_logger_in_dir(current_exe_dir()?)
}

#[cfg(windows)]
fn try_create_appdata_logger() -> anyhow::Result<Logger> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;
    use std::ptr::null_mut;

    use anyhow::bail;
    use winapi::um::knownfolders::FOLDERID_LocalAppDataLow;
    use winapi::um::shlobj::SHGetKnownFolderPath;
    use winapi::um::winnt::PWSTR;

    let log_dir: OsString = unsafe {
        let mut folder_path: PWSTR = null_mut();
        let hr = SHGetKnownFolderPath(&FOLDERID_LocalAppDataLow, 0, null_mut(), &mut folder_path);
        if hr < 0 {
            bail!("Failed to get LocalLow appdata folder");
        }
        let path_len = (0..).take_while(|&i| *folder_path.offset(i) != 0).count();
        let slice = std::slice::from_raw_parts(folder_path, path_len);
        OsStringExt::from_wide(slice)
    };

    let mut log_path = PathBuf::from(log_dir);
    log_path.push("bugle");
    std::fs::create_dir_all(&log_path)?;

    try_create_logger_in_dir(log_path)
}

#[cfg(windows)]
fn try_create_logger_in_dir(mut path: std::path::PathBuf) -> anyhow::Result<Logger> {
    path.push("bugle.log");

    let log_file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)?;

    let drain = slog_term::PlainDecorator::new(log_file);
    let drain = slog_term::FullFormat::new(drain).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    Ok(Logger::root(drain, o!()))
}
