//! The notification-area icon and its menu.
//!
//! The icon is the app's home: the settings window is only ever opened from
//! here, and the app is expected to sit in the notification area for days. The
//! icon handle is owned by this module and by nothing else -- `remove` frees
//! it, and `Drop` routes through `remove`, so an early exit cannot leak it or
//! leave a ghost icon in the notification area.

use std::mem::size_of;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, GetSystemMetrics, PostMessageW,
    SetForegroundWindow, TrackPopupMenu, HICON, MF_SEPARATOR, MF_STRING, SM_CXSMICON,
    TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_NULL,
};

/// What the user picked from the tray menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    OpenSettings,
    TogglePause,
    OpenLog,
    Exit,
}

/// The icon's id within the window. One icon per process, as in the original.
const ID: u32 = 1;

/// Menu command ids. They are what `TrackPopupMenu` returns under
/// `TPM_RETURNCMD`, not messages, so the values only have to be distinct.
const CMD_SETTINGS: u32 = 1;
const CMD_TOGGLE_PAUSE: u32 = 2;
const CMD_OPEN_LOG: u32 = 3;
const CMD_EXIT: u32 = 4;

/// `szTip` is a fixed 128-unit field the shell reads as NUL-terminated, so the
/// usable limit is one unit less.
const TIP_UNITS: usize = 128;

pub struct Tray {
    hwnd: HWND,
    id: u32,
    /// The icon the shell is displaying. `remove` takes it, which is what makes
    /// removal idempotent and the destroy happen exactly once.
    icon: Option<HICON>,
}

impl Tray {
    /// Add the icon. `callback_msg` is the app's own message number: the shell
    /// posts it to `hwnd` with the mouse event in `lparam`.
    ///
    /// Classic callback semantics, deliberately: this module never calls
    /// `NIM_SETVERSION`, so the shell posts `callback_msg` with `wparam` = the
    /// icon id and `lparam` = the raw mouse message (`WM_LBUTTONUP`,
    /// `WM_RBUTTONUP`, `WM_LBUTTONDBLCLK`). The app's window procedure is
    /// written against exactly that convention -- see `app::on_tray_message`.
    /// `NIM_SETVERSION` with `NOTIFYICON_VERSION_4` would move the event into
    /// the high word of `lparam` and silence the menu, so do not add it.
    pub fn add(hwnd: HWND, callback_msg: u32, tooltip: &str) -> Result<Tray, String> {
        if hwnd.is_invalid() {
            return Err("tray icon needs a window to report to".into());
        }

        let icon = crate::icon::hicon(unsafe { GetSystemMetrics(SM_CXSMICON) });
        if icon.is_invalid() {
            return Err("could not create the tray icon".into());
        }

        let mut data = NOTIFYICONDATAW {
            // The whole struct, as the original passed. Version 0 (classic) is
            // what the shell assumes while `NIM_SETVERSION` is not called, and
            // it accepts any `cbSize` at least as large as the version 1 one.
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: ID,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: callback_msg,
            hIcon: icon,
            ..Default::default()
        };
        write_tip(&mut data.szTip, tooltip);

        let added = unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool();
        crate::log::write(&format!("tray icon added={added}"));

        if !added {
            // The shell is not holding the icon, so this module owns it still.
            crate::icon::destroy(icon);
            return Err("Shell_NotifyIcon(NIM_ADD) failed".into());
        }

        Ok(Tray {
            hwnd,
            id: ID,
            icon: Some(icon),
        })
    }

    /// Replace the hover text. Failures are not worth reporting: the icon is
    /// still up and still functional, only its label is stale.
    pub fn set_tooltip(&self, text: &str) {
        let mut data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: self.id,
            uFlags: NIF_TIP,
            ..Default::default()
        };
        write_tip(&mut data.szTip, text);
        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
        }
    }

    /// Show the context menu at the cursor and wait for a choice.
    pub fn show_menu(&self, paused: bool) -> Option<Choice> {
        let menu = unsafe { CreatePopupMenu() }.ok()?;

        let items = [
            (CMD_SETTINGS, "Settings..."),
            (
                CMD_TOGGLE_PAUSE,
                if paused { "Resume hiding" } else { "Pause hiding" },
            ),
            (CMD_OPEN_LOG, "Open log"),
        ];
        for (id, label) in items {
            let text = crate::win::wide(label);
            let _ = unsafe { AppendMenuW(menu, MF_STRING, id as usize, crate::win::pcw(&text)) };
        }
        let _ = unsafe { AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null()) };
        let exit = crate::win::wide("Exit");
        let _ = unsafe { AppendMenuW(menu, MF_STRING, CMD_EXIT as usize, crate::win::pcw(&exit)) };

        let mut at = POINT::default();
        let _ = unsafe { GetCursorPos(&mut at) };

        unsafe {
            // The foreground dance. A popup menu belongs to the thread whose
            // window owns it, and this window is never activated, so without
            // taking the foreground the menu would not dismiss when the user
            // clicks elsewhere; the posted WM_NULL is what closes that gap.
            // Both halves are required -- this is the documented workaround,
            // not tidiness.
            let _ = SetForegroundWindow(self.hwnd);
            let cmd = TrackPopupMenu(
                menu,
                TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
                at.x,
                at.y,
                None,
                self.hwnd,
                None,
            );
            let _ = PostMessageW(Some(self.hwnd), WM_NULL, WPARAM(0), LPARAM(0));
            let _ = DestroyMenu(menu);
            choice(cmd.0)
        }
    }

    /// Take the icon down. Idempotent, so `Drop` can call it unconditionally.
    pub fn remove(&mut self) {
        let Some(icon) = self.icon.take() else {
            return;
        };
        let data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: self.id,
            ..Default::default()
        };
        let removed = unsafe { Shell_NotifyIconW(NIM_DELETE, &data) }.as_bool();
        crate::icon::destroy(icon);
        crate::log::write(&format!("tray icon removed={removed}"));
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        self.remove();
    }
}

/// Translate a command id into a choice. Zero is "the menu was dismissed",
/// which is not a choice.
fn choice(cmd: i32) -> Option<Choice> {
    match cmd as u32 {
        CMD_SETTINGS => Some(Choice::OpenSettings),
        CMD_TOGGLE_PAUSE => Some(Choice::TogglePause),
        CMD_OPEN_LOG => Some(Choice::OpenLog),
        CMD_EXIT => Some(Choice::Exit),
        _ => None,
    }
}

/// Copy a tooltip into the fixed field, NUL-terminated.
///
/// The field lives inside the struct, so an over-long tooltip cannot be an
/// error and must not panic; it is cut at the last whole character that fits.
/// Cutting per `char` rather than per UTF-16 unit keeps a surrogate pair from
/// being split in half, which would leave a lone surrogate in the field.
fn write_tip(field: &mut [u16; TIP_UNITS], text: &str) {
    let mut n = 0;
    for c in text.chars() {
        let mut buf = [0u16; 2];
        let units = c.encode_utf16(&mut buf);
        if n + units.len() >= TIP_UNITS {
            break;
        }
        field[n..n + units.len()].copy_from_slice(units);
        n += units.len();
    }
    field[n] = 0;
}
