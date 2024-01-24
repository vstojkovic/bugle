use anyhow::{bail, Result};
use slog::{trace, Logger};

#[cfg(not(windows))]
pub fn is_battleye_installed(_logger: &Logger) -> Result<bool> {
    bail!("BattlEye detection for Linux is not yet implemented in BUGLE");
}

#[cfg(windows)]
pub fn is_battleye_installed(logger: &Logger) -> Result<bool> {
    use std::ffi::OsString;
    use std::mem::{size_of, size_of_val};
    use std::os::windows::ffi::OsStringExt;
    use std::ptr::{null, null_mut};

    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::winnt::SERVICE_WIN32_OWN_PROCESS;
    use winapi::um::winsvc::{
        EnumServicesStatusExW, OpenSCManagerW, ENUM_SERVICE_STATUS_PROCESSW, SC_ENUM_PROCESS_INFO,
        SC_MANAGER_CONNECT, SC_MANAGER_ENUMERATE_SERVICE, SERVICE_STATE_ALL,
    };

    trace!(logger, "Connecting to service control manager");
    let scm_handle = unsafe {
        OpenSCManagerW(
            null(),
            null(),
            SC_MANAGER_CONNECT | SC_MANAGER_ENUMERATE_SERVICE,
        )
    };
    if scm_handle == null_mut() {
        let err = unsafe { GetLastError() };
        bail!("Error accessing service control manager, code: {}", err);
    }

    const LIST_SIZE: usize = 256 * 1024 / size_of::<ENUM_SERVICE_STATUS_PROCESSW>();
    let mut svc_list: [ENUM_SERVICE_STATUS_PROCESSW; LIST_SIZE] = [Default::default(); LIST_SIZE];

    let mut ret_count = 0u32;
    let mut bytes_needed = 0u32;
    let mut resume_handle = 0u32;
    let battleye_name: OsString = "BEService".into();
    let ret = unsafe {
        EnumServicesStatusExW(
            scm_handle,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32_OWN_PROCESS,
            SERVICE_STATE_ALL,
            &mut svc_list as *mut ENUM_SERVICE_STATUS_PROCESSW as _,
            size_of_val(&svc_list) as u32,
            &mut bytes_needed,
            &mut ret_count,
            &mut resume_handle,
            null_mut(),
        )
    };
    trace!(
        logger,
        "EnumServicesStatusExW returned {}", ret;
        "ret_count" => ret_count,
        "resume_handle" => resume_handle,
        "bytes_needed" => bytes_needed,
    );
    if ret == 0 {
        let err = unsafe { GetLastError() };
        bail!("Error enumerating services, code: {}", err);
    }
    for idx in 0..(ret_count as usize) {
        let svc_name: OsString = unsafe {
            let svc_name = svc_list[idx].lpServiceName;
            let path_len = (0..).take_while(|&i| *svc_name.offset(i) != 0).count();
            let slice = std::slice::from_raw_parts(svc_name, path_len);
            OsStringExt::from_wide(slice)
        };
        if svc_name == battleye_name {
            return Ok(true);
        }
    }
    Ok(false)
}
