use std::fmt::Debug;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use slog::{o, Discard, Drain, FilterLevel, Logger};
use slog_async::Async;

#[cfg(default_log_debug)]
pub const DEFAULT_LOG_LEVEL: FilterLevel = FilterLevel::Debug;
#[cfg(not(default_log_debug))]
pub const DEFAULT_LOG_LEVEL: FilterLevel = FilterLevel::Info;

#[cfg(not(windows))]
pub fn create_root_logger(log_level: &Arc<AtomicUsize>) -> Logger {
    create_term_logger(log_level)
}

#[cfg(windows)]
pub fn create_root_logger(log_level: &Arc<AtomicUsize>) -> Logger {
    if unsafe { winapi::um::wincon::AttachConsole(u32::MAX) } != 0 {
        create_term_logger(log_level)
    } else {
        try_create_portable_mode_logger(log_level)
            .or_else(|_| try_create_appdata_logger(log_level))
            .unwrap_or_else(|_| create_discard_logger(log_level))
    }
}

fn create_term_logger(log_level: &Arc<AtomicUsize>) -> Logger {
    let drain = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(drain).build().fuse();
    create_root_logger_for_drain(drain, log_level)
}

fn create_discard_logger(log_level: &Arc<AtomicUsize>) -> Logger {
    create_root_logger_for_drain(Discard, log_level)
}

#[cfg(windows)]
fn try_create_portable_mode_logger(log_level: &Arc<AtomicUsize>) -> anyhow::Result<Logger> {
    use crate::env::current_exe_dir;

    try_create_logger_in_dir(current_exe_dir()?, log_level)
}

#[cfg(windows)]
fn try_create_appdata_logger(log_level: &Arc<AtomicUsize>) -> anyhow::Result<Logger> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;
    use std::ptr::null_mut;

    use anyhow::bail;
    use winapi::um::combaseapi::CoTaskMemFree;
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
        let dir = OsStringExt::from_wide(slice);
        CoTaskMemFree(folder_path as _);
        dir
    };

    let mut log_path = PathBuf::from(log_dir);
    log_path.push("bugle");
    std::fs::create_dir_all(&log_path)?;

    try_create_logger_in_dir(log_path, log_level)
}

#[cfg(windows)]
fn try_create_logger_in_dir(
    mut path: std::path::PathBuf,
    log_level: &Arc<AtomicUsize>,
) -> anyhow::Result<Logger> {
    path.push("bugle.log");

    let log_file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)?;

    let drain = slog_term::PlainDecorator::new(log_file);
    let drain = slog_term::FullFormat::new(drain).build().fuse();
    Ok(create_root_logger_for_drain(drain, log_level))
}

fn create_root_logger_for_drain<D>(drain: D, log_level: &Arc<AtomicUsize>) -> Logger
where
    D: 'static + Drain + Send,
    D::Err: Debug,
{
    let drain = RuntimeLevelFilter::new(drain, log_level).fuse();
    let drain = Async::new(drain).build().fuse();
    Logger::root(drain, o!())
}

struct RuntimeLevelFilter<D: Drain> {
    drain: D,
    level: Arc<AtomicUsize>,
}

impl<D: Drain> RuntimeLevelFilter<D> {
    fn new(drain: D, log_level: &Arc<AtomicUsize>) -> Self {
        Self {
            drain,
            level: Arc::clone(log_level),
        }
    }
}

impl<D: Drain> Drain for RuntimeLevelFilter<D> {
    type Ok = Option<D::Ok>;
    type Err = Option<D::Err>;
    fn log(
        &self,
        record: &slog::Record,
        values: &slog::OwnedKVList,
    ) -> std::result::Result<Self::Ok, Self::Err> {
        let level = FilterLevel::from_usize(self.level.load(Ordering::Relaxed)).unwrap();
        if level.accepts(record.level()) {
            self.drain.log(record, values).map(Some).map_err(Some)
        } else {
            Ok(None)
        }
    }
}
