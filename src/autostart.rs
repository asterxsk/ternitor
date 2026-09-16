//! "Start with Windows": one value under the per-user Run key.
//!
//! The Run key rather than a Startup shortcut because a GUI-subsystem exe needs
//! no launcher shim -- the whole reason the PowerShell version needed a `.vbs`
//! was to avoid its own console, and this build has no console to hide. A Run
//! entry is also a single registry value the app can read back, so the settings
//! screen shows real state rather than a remembered one.

use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SAM_FLAGS, REG_SZ,
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};

use crate::win::{pcw, wide};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE: &str = "Ternitor";

fn open(access: REG_SAM_FLAGS) -> Option<HKEY> {
    let key = wide(RUN_KEY);
    let mut hkey = HKEY::default();
    let rc = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, pcw(&key), None, access, &mut hkey) };
    (rc == ERROR_SUCCESS).then_some(hkey)
}

/// The command the Run entry holds, so a stale entry pointing at a moved exe can
/// be told apart from a live one.
fn entry() -> Option<String> {
    let hkey = open(KEY_QUERY_VALUE)?;
    let name = wide(VALUE);
    let mut kind = Default::default();
    let mut len = 0u32;

    // Ask for the size first: the value is a path of unknown length.
    let rc =
        unsafe { RegQueryValueExW(hkey, pcw(&name), None, Some(&mut kind), None, Some(&mut len)) };
    if rc != ERROR_SUCCESS || len == 0 {
        unsafe {
            let _ = RegCloseKey(hkey);
        }
        return None;
    }

    let mut buf = vec![0u8; len as usize];
    let rc = unsafe {
        RegQueryValueExW(
            hkey,
            pcw(&name),
            None,
            Some(&mut kind),
            Some(buf.as_mut_ptr()),
            Some(&mut len),
        )
    };
    unsafe {
        let _ = RegCloseKey(hkey);
    }
    if rc != ERROR_SUCCESS {
        return None;
    }

    let units: Vec<u16> = buf
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&c| c != 0)
        .collect();
    Some(String::from_utf16_lossy(&units))
}

/// Enabled only when the entry points at *this* exe. A leftover entry from a
/// previous location would otherwise make the toggle claim a state the next
/// logon would contradict.
pub fn is_enabled() -> bool {
    let Some(value) = entry() else { return false };
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let want = exe.to_string_lossy();
    value.trim_matches('"').eq_ignore_ascii_case(want.trim_matches('"'))
}

pub fn set(enabled: bool) -> Result<(), String> {
    let Some(hkey) = open(KEY_SET_VALUE) else {
        return Err("could not open the Run key".into());
    };
    let name = wide(VALUE);

    let rc = if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        // Quoted: a path with spaces would otherwise be split at the first one.
        let command = wide(&format!("\"{}\"", exe.display()));
        let bytes =
            unsafe { std::slice::from_raw_parts(command.as_ptr() as *const u8, command.len() * 2) };
        unsafe { RegSetValueExW(hkey, pcw(&name), None, REG_SZ, Some(bytes)) }
    } else {
        let rc = unsafe { RegDeleteValueW(hkey, pcw(&name)) };
        // Already absent is the state we wanted.
        if rc == ERROR_FILE_NOT_FOUND { ERROR_SUCCESS } else { rc }
    };

    unsafe {
        let _ = RegCloseKey(hkey);
    }
    if rc == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(format!("registry write failed ({})", rc.0))
    }
}
