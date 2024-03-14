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

#[cfg(windows)]
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum AppDataFolder {
    Roaming,
    Local,
    LocalLow,
}

#[cfg(windows)]
impl AppDataFolder {
    fn id(self) -> &'static winapi::shared::guiddef::GUID {
        use winapi::um::knownfolders::{
            FOLDERID_LocalAppData, FOLDERID_LocalAppDataLow, FOLDERID_RoamingAppData,
        };
        match self {
            Self::Roaming => &FOLDERID_RoamingAppData,
            Self::Local => &FOLDERID_LocalAppData,
            Self::LocalLow => &FOLDERID_LocalAppDataLow,
        }
    }
}

#[cfg(windows)]
pub fn appdata_dir(folder: AppDataFolder) -> Result<PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::ptr::null_mut;

    use winapi::um::combaseapi::CoTaskMemFree;
    use winapi::um::shlobj::SHGetKnownFolderPath;
    use winapi::um::winnt::PWSTR;

    let dir: OsString = unsafe {
        let mut folder_path: PWSTR = null_mut();
        let hr = SHGetKnownFolderPath(folder.id(), 0, null_mut(), &mut folder_path);
        if hr < 0 {
            return Err(Error::from_raw_os_error(hr));
        }
        let path_len = (0..).take_while(|&i| *folder_path.offset(i) != 0).count();
        let slice = std::slice::from_raw_parts(folder_path, path_len);
        let dir = OsStringExt::from_wide(slice);
        CoTaskMemFree(folder_path as _);
        dir
    };
    Ok(PathBuf::from(dir))
}
