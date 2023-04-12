use std::cell::Cell;
use std::process::{Child, Command};

use anyhow::{bail, Result};
use slog::{debug, trace, Logger};

pub struct Launch {
    logger: Logger,
    child: Child,
    poll_impl: PollImpl,
}

#[derive(Debug, Clone, Copy)]
pub enum LaunchState {
    Pending,
    Ready,
}

impl Launch {
    pub(super) fn new(logger: &Logger, mut cmd: Command) -> Result<Self> {
        let logger = logger.clone();

        let child = cmd.spawn()?;
        debug!(logger, "Spawned the child process"; "pid" => child.id());

        let poll_impl = PollImpl::new(&logger, &cmd, &child);
        Ok(Self {
            logger,
            child,
            poll_impl,
        })
    }

    pub fn poll(&mut self) -> Result<LaunchState> {
        debug!(&self.logger, "Checking if the game is visible");
        if let Some(code) = self.child.try_wait()? {
            bail!("Game process ended unexpectedly with status {}", code);
        }
        self.poll_impl.poll()
    }

    pub fn cancel(&mut self) {
        debug!(&self.logger, "Killing the child process"; "pid" => self.child.id());
        let _ = self.child.kill();
        self.poll_impl.cancel();
    }
}

#[cfg(not(windows))]
struct PollImpl;

#[cfg(not(windows))]
impl PollImpl {
    fn new(_: &Logger, _: &Command, _: &Child) -> Self {
        Self
    }

    fn poll(&self) -> Result<LaunchState> {
        LaunchState::Ready
    }

    fn cancel(&self) {
        // nothing to do here
    }
}

#[cfg(windows)]
struct PollImpl {
    logger: Logger,
    pid: Cell<Option<u32>>,
}

#[cfg(windows)]
impl PollImpl {
    fn new(logger: &Logger, cmd: &Command, child: &Child) -> Self {
        let pid = if cmd.get_program().to_string_lossy().ends_with(GAME_EXE) {
            Some(child.id())
        } else {
            None
        };
        Self {
            logger: logger.clone(),
            pid: Cell::new(pid),
        }
    }

    fn poll(&self) -> Result<LaunchState> {
        use winapi::shared::minwindef::{BOOL, DWORD, FALSE, LPARAM, TRUE};
        use winapi::shared::windef::HWND;
        use winapi::um::errhandlingapi::{GetLastError, SetLastError};
        use winapi::um::winuser::{EnumWindows, GetWindowThreadProcessId, IsWindowVisible};

        unsafe extern "system" fn win_enum_fn(hwnd: HWND, param: LPARAM) -> BOOL {
            let game_pid: DWORD = param.try_into().unwrap();
            let mut win_pid: DWORD = 0;
            let rc = GetWindowThreadProcessId(hwnd, &mut win_pid);
            if (rc != 0) && (win_pid == game_pid) && (IsWindowVisible(hwnd) == TRUE) {
                SetLastError(0);
                FALSE
            } else {
                TRUE
            }
        }

        let pid = match self.pid.get() {
            Some(pid) => pid,
            None => match self.find_game_pid()? {
                Some(pid) => {
                    self.pid.set(Some(pid));
                    pid
                }
                None => return Ok(LaunchState::Pending),
            },
        };

        let pid_as_param: LPARAM = pid.try_into()?;

        trace!(&self.logger, "Looking for visible window in process"; "pid" => pid);
        let enum_result = unsafe { EnumWindows(Some(win_enum_fn), pid_as_param) };

        if enum_result == 0 {
            let err = unsafe { GetLastError() };
            if err == 0 {
                Ok(LaunchState::Ready)
            } else {
                bail!("Error enumerating windows, code: {}", err);
            }
        } else {
            trace!(&self.logger, "No visible window found"; "pid" => pid);
            Ok(LaunchState::Pending)
        }
    }

    fn cancel(&self) {
        use std::ptr::null_mut;

        use winapi::shared::minwindef::FALSE;
        use winapi::um::processthreadsapi::{OpenProcess, TerminateProcess};
        use winapi::um::winnt::PROCESS_TERMINATE;

        let pid = match self
            .pid
            .get()
            .or_else(|| self.find_game_pid().ok().unwrap_or_default())
        {
            Some(pid) => pid,
            None => return,
        };

        trace!(&self.logger, "Opening game process handle"; "pid" => pid);
        let handle = unsafe { OpenProcess(PROCESS_TERMINATE, FALSE, pid) };
        if handle != null_mut() {
            let rc = unsafe { TerminateProcess(handle, 0) };
            debug!(&self.logger, "Attempted to terminate the game process"; "result" => rc);
        }
    }

    fn find_game_pid(&self) -> Result<Option<u32>> {
        use std::ffi::{OsStr, OsString};
        use std::os::windows::ffi::OsStringExt;

        use winapi::shared::minwindef::TRUE;
        use winapi::um::errhandlingapi::GetLastError;
        use winapi::um::handleapi::INVALID_HANDLE_VALUE;
        use winapi::um::tlhelp32::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        };

        trace!(&self.logger, "Looking for game PID");

        let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if snap == INVALID_HANDLE_VALUE {
            let err = unsafe { GetLastError() };
            bail!("Error enumerating processes, code: {}", err);
        }

        let mut proc_entry = PROCESSENTRY32W::default();
        proc_entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>().try_into()?;

        let mut keep_iterating = unsafe { Process32FirstW(snap, &mut proc_entry) };
        while keep_iterating == TRUE {
            let path_len = (0..).take_while(|&i| proc_entry.szExeFile[i] != 0).count();
            let slice = &proc_entry.szExeFile[..path_len];
            let exe_name = OsString::from_wide(slice);
            let game_exe: &OsStr = GAME_EXE.as_ref();

            trace!(
                &self.logger,
                "Checking process";
                "pid" => proc_entry.th32ProcessID,
                "exe_name" => ?exe_name,
            );

            if exe_name == game_exe {
                return Ok(Some(proc_entry.th32ProcessID));
            }
            keep_iterating = unsafe { Process32NextW(snap, &mut proc_entry) };
        }

        trace!(&self.logger, "No matching process found");

        Ok(None)
    }
}

const GAME_EXE: &str = "ConanSandbox.exe";
