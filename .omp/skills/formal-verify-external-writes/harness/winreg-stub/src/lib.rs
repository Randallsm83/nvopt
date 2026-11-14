//! Compile-only stand-in for the Windows-only `winreg` crate.
//!
//! `rtss.rs` reads the registry in `detect_rtss_path`, which only `RtssManager::new`
//! calls. The property tests build `RtssManager` with a temporary `Profiles` directory
//! and never call `new`, so the harness must not reach a real registry on any machine.
//! Every lookup here fails, which is what a machine without the key returns.

use std::ffi::OsStr;
use std::io;

pub mod enums {
    pub struct PredefinedKey;
    pub const HKEY_LOCAL_MACHINE: PredefinedKey = PredefinedKey;
    pub const HKEY_CURRENT_USER: PredefinedKey = PredefinedKey;
}

pub struct RegKey;

impl RegKey {
    pub fn predef(_key: enums::PredefinedKey) -> RegKey {
        RegKey
    }

    pub fn open_subkey<P: AsRef<OsStr>>(&self, _path: P) -> io::Result<RegKey> {
        Err(io::Error::new(io::ErrorKind::NotFound, "winreg stand-in: no registry"))
    }

    pub fn get_value<T, N: AsRef<OsStr>>(&self, _name: N) -> io::Result<T> {
        Err(io::Error::new(io::ErrorKind::NotFound, "winreg stand-in: no registry"))
    }
}
