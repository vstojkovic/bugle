use std::ffi::c_void;
use std::path::Path;

use anyhow::Result;
use dlopen::symbor::Library;

pub fn get_game_version(game_path: &Path) -> Result<(u32, u16)> {
    let dw_lib = Library::open(game_path.join("ConanSandbox/Binaries/Win64/Dreamworld.dll"))?;
    let get_version = unsafe { dw_lib.symbol::<DwFnGetVersion>(SYM_GET_VERSION)? };
    let get_revision = unsafe { dw_lib.symbol::<DwFnGetRevision>(SYM_GET_REVISION)? };
    let get_snapshot = unsafe { dw_lib.symbol::<DwFnGetSnapshot>(SYM_GET_SNAPSHOT)? };

    unsafe {
        let version = get_version();
        Ok((get_revision(version), get_snapshot(version)))
    }
}

type DwFnGetVersion = unsafe extern "C" fn() -> *mut c_void;
type DwFnGetRevision = unsafe extern "C" fn(*mut c_void) -> u32;
type DwFnGetSnapshot = unsafe extern "C" fn(*mut c_void) -> u16;

const SYM_GET_VERSION: &str = "?Get@Version@dw@@SAAEAV12@XZ";
const SYM_GET_REVISION: &str = "?GetRevision@Version@dw@@QEBA?BHXZ";
const SYM_GET_SNAPSHOT: &str = "?GetSnapshot@Version@dw@@QEBA?BHXZ";
