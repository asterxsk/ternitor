//! Append-only log next to the exe.
//!
//! The log is the app's only diagnostic surface: the whole point is that nothing
//! is ever shown on screen, so when something is wrong this is where you look.
//! It is written beside the executable rather than under AppData so the whole
//! app stays one portable folder. Failures are swallowed -- a janitor that
//! cannot write a log line should still keep hiding windows.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static PATH: OnceLock<PathBuf> = OnceLock::new();

/// Point the log at a file. Called once, before anything is logged.
pub fn init(path: PathBuf) {
    let _ = PATH.set(path);
}

pub fn path() -> Option<&'static Path> {
    PATH.get().map(|p| p.as_path())
}

/// Append one timestamped line. Local time, to match what the user sees.
pub fn write(msg: &str) {
    let Some(path) = PATH.get() else { return };
    let stamp = timestamp();
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{stamp}  {msg}");
    }
}

/// `yyyy-MM-dd HH:mm:ss.fff` without pulling in a date library.
fn timestamp() -> String {
    use windows::Win32::System::SystemInformation::GetLocalTime;
    let t = unsafe { GetLocalTime() };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}

/// Reveal a file in Explorer, used by the tray menu.
pub fn reveal(path: &Path) {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let file = crate::win::wide(&path.to_string_lossy());
    let verb = crate::win::wide("open");
    unsafe {
        ShellExecuteW(
            None,
            crate::win::pcw(&verb),
            crate::win::pcw(&file),
            None,
            None,
            SW_SHOWNORMAL,
        );
    }
}
