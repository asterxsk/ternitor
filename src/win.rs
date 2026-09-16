//! Small Win32 helpers shared by the rest of the crate.

use windows::core::PCWSTR;
use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HINSTANCE};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

/// UTF-16, NUL-terminated, for the `*W` APIs.
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Borrow a `wide` buffer as the pointer the Win32 signatures want.
pub fn pcw(s: &[u16]) -> PCWSTR {
    PCWSTR(s.as_ptr())
}

/// The module handle, which is also this process's instance handle.
pub fn instance() -> HINSTANCE {
    unsafe { GetModuleHandleW(None).unwrap_or_default().into() }
}

/// True when the last call failed specifically with "already exists" -- how the
/// single-instance mutex reports that another Ternitor is running.
pub fn already_exists() -> bool {
    unsafe { GetLastError() == ERROR_ALREADY_EXISTS }
}
