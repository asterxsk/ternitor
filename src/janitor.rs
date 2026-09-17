//! The janitor: watching for new console windows and hiding the blank ones.
//!
//! Ported from the PowerShell implementation, which is the behavioural spec.
//!
//! Why the windows appear: an agent runs as a ConPTY client, which has no
//! console, so any child it spawns with default creation flags gets a brand-new
//! console, and the default-terminal broker hands that console to Windows
//! Terminal as a visible window. Nothing the child is passed can prevent it --
//! creation flags belong to the spawner.
//!
//! Detection is two conditions, never one: the window class must be
//! `CASCADIA_HOSTING_WINDOW_CLASS` *and* the owning process's command line must
//! contain `-Embedding`. Only the broker passes that flag, so a terminal a human
//! opened is never touched. The title cannot stand in for it: at create/show
//! time it is still Windows Terminal's placeholder, becoming the client's path
//! only later, which is far too late to prevent the flash.
//!
//! Steady state is event-driven -- no polling of windows. Everything runs on the
//! installing thread: the hook callbacks are out-of-context and the deferred
//! re-checks ride a timer on this module's own message-only window, so the app's
//! existing message loop drives all of it.

// Everything here is reachable from `Janitor::install`, which the app does not
// call yet, so the whole module reads as unused to the compiler. Delete this
// once main.rs wires the janitor in -- a warning in this file should mean
// something then.
#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::ffi::c_void;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{
    CloseHandle, GetLastError, HANDLE, HWND, LPARAM, LRESULT, WPARAM,
};
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetClassNameW, GetForegroundWindow, GetWindowTextW,
    GetWindowThreadProcessId, IsWindow, RegisterClassW, SetForegroundWindow, SetTimer, ShowWindow,
    EVENT_OBJECT_CREATE, EVENT_OBJECT_SHOW, EVENT_SYSTEM_FOREGROUND, HWND_MESSAGE, OBJID_WINDOW,
    SW_HIDE, SW_SHOWNOACTIVATE, WINEVENT_OUTOFCONTEXT, WM_TIMER, WNDCLASSW, WS_EX_TOOLWINDOW,
    WS_POPUP,
};

use crate::log;
use crate::win;

/// The class Windows Terminal's window is created with.
const CASCADIA_CLASS: &str = "CASCADIA_HOSTING_WINDOW_CLASS";
/// The legacy console host, which has no broker flag to look for.
const CONSOLE_CLASS: &str = "ConsoleWindowClass";
/// Set by the default-terminal broker and nothing else.
const EMBEDDING_FLAG: &[u8] = b"-Embedding";

const CLASS_NAME: &str = "TernitorJanitorWnd";
const WINDOW_NAME: &str = "Ternitor";

const PENDING_TIMER_ID: usize = 1;
const PENDING_INTERVAL_MS: u32 = 200;
/// How long a hidden window's title is given to declare what it really is. Long
/// on purpose: an agent-spawned blank resolves to a bare executable path within
/// ~120ms, while a real shell can take seconds to set its title -- and a shell
/// wrongly hidden is worse than a slow reveal.
const RECHECK_DELAY: Duration = Duration::from_millis(600);
/// How many times an undecided title is looked at again before it is written off
/// as a blank.
const MAX_RECHECK_TRIES: u32 = 10;
/// Windows already given back are remembered, but not without limit.
const RELEASED_MEMORY: usize = 512;

const PROCESS_COMMAND_LINE_INFORMATION: u32 = 60;

#[link(name = "ntdll")]
unsafe extern "system" {
    /// Declared through ntdll directly: the command line is the whole
    /// discriminator, and it is not otherwise reachable without pulling in a
    /// toolhelp snapshot.
    fn NtQueryInformationProcess(
        handle: HANDLE,
        class: u32,
        info: *mut c_void,
        len: u32,
        ret_len: *mut u32,
    ) -> i32;
}

/// `UNICODE_STRING`: a length in bytes and a pointer to the characters.
#[repr(C)]
struct UnicodeString {
    length: u16,
    maximum_length: u16,
    buffer: *const u16,
}

/// What the janitor reports, so the settings screen can show real activity.
pub enum Event {
    Hidden { title: String },
    Restored { title: String },
}

/// A window that has been hidden and must be re-checked, because a real shell
/// launched from a console-less context looks identical at the instant it
/// appears.
struct Pending {
    hwnd: HWND,
    at: Instant,
    tries: u32,
}

struct State {
    paused: bool,
    hidden: u64,
    /// The last foreground window that was not one we would hide, so focus can
    /// be handed back instead of left nowhere after a hide.
    last_foreground: HWND,
    pending: Vec<Pending>,
    /// Windows already given back. A shell the user can see stays visible, no
    /// matter how many create/show events it goes on to produce.
    released: HashSet<usize>,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    /// The app's callback. Win32 hands the hook callbacks no context pointer, so
    /// the callback is kept here and looked up on the installing thread.
    static CALLBACK: Cell<Option<&'static dyn Fn(Event)>> = const { Cell::new(None) };
}

/// A window handle as a hashable key. Never dereferenced -- handles are opaque.
fn key(hwnd: HWND) -> usize {
    hwnd.0 as usize
}

/// Report a failure through the app's only diagnostic surface and hand it back.
fn fail(msg: String) -> Result<Janitor, String> {
    log::write(&format!("janitor: {msg}"));
    Err(msg)
}

/// The app's handle on the running janitor. The counters themselves live in the
/// thread-local state, which is the same thread the app runs on.
pub struct Janitor {
    paused: bool,
}

impl Janitor {
    /// Install the watch. Must be called on the thread that pumps messages:
    /// events are delivered through that thread's queue. The callback runs on
    /// that same thread, so it may touch app state directly.
    pub fn install(on_event: Box<dyn Fn(Event)>) -> Result<Janitor, String> {
        if STATE.with(|cell| cell.borrow().is_some()) {
            return fail("already installed".into());
        }

        // Called for the life of the process and reached without a context
        // pointer, so it is handed over rather than owned.
        let callback: &'static dyn Fn(Event) = Box::leak(on_event);
        CALLBACK.with(|slot| slot.set(Some(callback)));
        STATE.with(|cell| {
            *cell.borrow_mut() = Some(State {
                paused: false,
                hidden: 0,
                last_foreground: HWND::default(),
                pending: Vec::new(),
                released: HashSet::new(),
            });
        });

        let class_name = win::wide(CLASS_NAME);
        let window_name = win::wide(WINDOW_NAME);
        let instance = win::instance();

        unsafe {
            let wc = WNDCLASSW {
                lpfnWndProc: Some(wnd_proc),
                hInstance: instance,
                lpszClassName: win::pcw(&class_name),
                ..Default::default()
            };
            if RegisterClassW(&wc) == 0 {
                // A second install in the same process is the only benign
                // reason for this, and it is harmless: the class is ours.
                let err = GetLastError().0;
                if !win::already_exists() {
                    return fail(format!("RegisterClassW failed (error {err})"));
                }
            }

            // Message-only: it exists to own the re-check timer and must never
            // be a window on screen in its own right.
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                win::pcw(&class_name),
                win::pcw(&window_name),
                WS_POPUP,
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(instance),
                None,
            )
            .map_err(|e| format!("CreateWindowExW failed ({e})"))?;

            let foreground = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(on_foreground),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
            let objects = SetWinEventHook(
                EVENT_OBJECT_CREATE,
                EVENT_OBJECT_SHOW,
                None,
                Some(on_object_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
            if foreground.is_invalid() || objects.is_invalid() {
                return fail(format!(
                    "SetWinEventHook failed (error {})",
                    GetLastError().0
                ));
            }

            let timer = SetTimer(Some(hwnd), PENDING_TIMER_ID, PENDING_INTERVAL_MS, None);
            if timer == 0 {
                return fail(format!("SetTimer failed (error {})", GetLastError().0));
            }
        }

        Ok(Janitor { paused: false })
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
        STATE.with(|cell| {
            if let Some(st) = cell.borrow_mut().as_mut() {
                st.paused = paused;
            }
        });
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn hidden_count(&self) -> u64 {
        STATE.with(|cell| cell.borrow().as_ref().map_or(0, |st| st.hidden))
    }
}

/// Hand an event to the app. Called with no borrow held, so the callback is free
/// to call back into `Janitor`.
fn emit(event: Event) {
    if let Some(callback) = CALLBACK.with(|slot| slot.get()) {
        callback(event);
    }
}

/// The janitor's own window: nothing but the re-check timer arrives here. The
/// tray owns its own window, since the shell will not post to a message-only one.
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_TIMER && wparam.0 == PENDING_TIMER_ID {
        process_pending();
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// Keeps track of what was in front, so a hide can put focus back rather than
/// leaving the user on the desktop -- and takes focus back when a window we hid
/// takes it instead.
///
/// A hidden console can still be activated. The broker wakes the client's window
/// again as the console attaches, seconds after the hide, and an activation moves
/// focus whether or not anything is on screen. The object events do not cover
/// that: Windows Terminal re-activates without re-showing, so nothing else here
/// would notice, and the user is left typing into a window they cannot see.
unsafe extern "system" fn on_foreground(
    _hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time: u32,
) {
    if hwnd.is_invalid() {
        return;
    }

    // Borrowed across the Win32 calls, as in `on_object_event`: nothing reached
    // from here touches the state again.
    STATE.with(|cell| {
        let mut guard = cell.borrow_mut();
        let Some(st) = guard.as_mut() else { return };

        if !st.paused && !st.released.contains(&key(hwnd)) && is_unwanted(hwnd) {
            unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
            restore_focus(st.last_foreground, hwnd);
            log::write(&format!("focus taken back title=[{}]", title_of(hwnd)));
            return;
        }

        st.last_foreground = hwnd;
    });
}

unsafe extern "system" fn on_object_event(
    _hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    _thread: u32,
    _time: u32,
) {
    if id_object != OBJID_WINDOW.0 || id_child != 0 || hwnd.is_invalid() {
        return;
    }

    // The event is handled with the state borrowed; the callback that tells the
    // app about it is not, so it can read `paused`/`hidden_count` safely.
    let event = STATE.with(|cell| {
        let mut guard = cell.borrow_mut();
        let st = guard.as_mut()?;

        if st.paused || st.released.contains(&key(hwnd)) || !is_unwanted(hwnd) {
            return None;
        }

        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
        restore_focus(st.last_foreground, hwnd);
        st.hidden += 1;

        if !st.pending.iter().any(|p| p.hwnd == hwnd) {
            st.pending.push(Pending {
                hwnd,
                at: Instant::now(),
                tries: 0,
            });
        }

        let title = title_of(hwnd);
        log::write(&format!(
            "hid {} (defterm handoff, client title {title})",
            class_of(hwnd)
        ));
        Some(Event::Hidden { title })
    });

    if let Some(event) = event {
        emit(event);
    }
}

/// Give back anything that turns out to have been a real shell.
fn process_pending() {
    let mut events = Vec::new();

    STATE.with(|cell| {
        let mut guard = cell.borrow_mut();
        let Some(st) = guard.as_mut() else { return };

        // Backwards, because entries are removed as they are decided.
        let mut i = st.pending.len();
        while i > 0 {
            i -= 1;
            if st.pending[i].at.elapsed() < RECHECK_DELAY {
                continue;
            }

            let hwnd = st.pending[i].hwnd;
            if !unsafe { IsWindow(Some(hwnd)).as_bool() } {
                st.pending.remove(i);
                log::write("pending: window gone");
                continue;
            }

            let title = title_of(hwnd).trim().to_string();
            match classify(&title) {
                // Still no title: the client has not attached yet, so wait.
                Verdict::Undecided if st.pending[i].tries < MAX_RECHECK_TRIES => {
                    st.pending[i].tries += 1;
                    st.pending[i].at = Instant::now();
                }
                Verdict::Undecided | Verdict::StayHidden => {
                    st.pending.remove(i);
                    log::write(&format!("pending: stay hidden title=[{title}]"));
                }
                Verdict::Restore => {
                    st.pending.remove(i);
                    if st.released.len() >= RELEASED_MEMORY {
                        st.released.clear();
                    }
                    st.released.insert(key(hwnd));
                    unsafe {
                        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                    }
                    log::write(&format!("re-shown, real shell title=[{title}]"));
                    events.push(Event::Restored { title });
                }
            }
        }
    });

    for event in events {
        emit(event);
    }
}

/// True when this window is one of the blank consoles we exist to hide.
fn is_unwanted(hwnd: HWND) -> bool {
    let class = class_of(hwnd);

    if class == CASCADIA_CLASS {
        let mut pid = 0u32;
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
        }
        return command_line_of(pid).is_some_and(|cmd| command_line_has_embedding(&cmd));
    }

    // Legacy conhost window: no broker flag to look for, so fall back to the
    // placeholder title -- a bare executable path -- that only the OS assigns.
    if class == CONSOLE_CLASS {
        let title = title_of(hwnd);
        let title = title.trim();
        return title.is_empty() || is_bare_exe_path(title);
    }

    false
}

/// What a pending window's title says it is.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// No title yet, or still Windows Terminal's placeholder.
    Undecided,
    /// A blank console: the OS-assigned name of the executable and nothing more.
    StayHidden,
    /// Something a person asked for, which has to be given back.
    Restore,
}

fn classify(title: &str) -> Verdict {
    if is_undecided_title(title) {
        Verdict::Undecided
    } else if is_bare_exe_path(title) {
        Verdict::StayHidden
    } else {
        Verdict::Restore
    }
}

/// Windows Terminal's own title, before it has been replaced by the client's.
fn is_undecided_title(title: &str) -> bool {
    title.is_empty() || title == "Windows Terminal" || title == "Terminal"
}

/// `C:\Windows\System32\cmd.exe` and nothing besides: the OS's placeholder for a
/// console whose client never named it.
fn is_bare_exe_path(title: &str) -> bool {
    let b = title.as_bytes();
    b.len() > 3
        && b[b.len() - 4..].eq_ignore_ascii_case(b".exe")
        && b[0].is_ascii_alphabetic()
        && b[1] == b':'
        && (b[2] == b'\\' || b[2] == b'/')
}

/// Case-insensitive, and allocation-free: this runs for every window that is
/// created or shown anywhere on the desktop.
fn command_line_has_embedding(cmd: &str) -> bool {
    let haystack = cmd.as_bytes();
    let needle = EMBEDDING_FLAG;
    haystack.len() >= needle.len()
        && haystack
            .windows(needle.len())
            .any(|w| w.eq_ignore_ascii_case(needle))
}

/// The command line of a process, or `None` when it cannot be read.
fn command_line_of(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let result = query_command_line(handle);
    unsafe {
        let _ = CloseHandle(handle);
    }
    result
}

fn query_command_line(handle: HANDLE) -> Option<String> {
    // First a length probe: the call fails with a length mismatch and reports
    // what it would have needed.
    let mut needed = 0u32;
    unsafe {
        NtQueryInformationProcess(
            handle,
            PROCESS_COMMAND_LINE_INFORMATION,
            std::ptr::null_mut(),
            0,
            &mut needed,
        );
    }
    if needed == 0 {
        return None;
    }

    // A `UNICODE_STRING` followed by the characters, so the buffer has to be
    // pointer-aligned: words, not bytes.
    let mut buf = vec![0u64; needed.div_ceil(8) as usize];
    let mut written = 0u32;
    let status = unsafe {
        NtQueryInformationProcess(
            handle,
            PROCESS_COMMAND_LINE_INFORMATION,
            buf.as_mut_ptr() as *mut c_void,
            (buf.len() * size_of::<u64>()) as u32,
            &mut written,
        )
    };
    if status != 0 {
        return None;
    }

    let us = unsafe { std::ptr::read_unaligned(buf.as_ptr() as *const UnicodeString) };
    if us.buffer.is_null() || us.length < 2 {
        return None;
    }
    let chars = unsafe { std::slice::from_raw_parts(us.buffer, (us.length / 2) as usize) };
    Some(String::from_utf16_lossy(chars))
}

fn class_of(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let n = unsafe { GetClassNameW(hwnd, &mut buf) };
    let n = n.clamp(0, buf.len() as i32) as usize;
    String::from_utf16_lossy(&buf[..n])
}

fn title_of(hwnd: HWND) -> String {
    let mut buf = [0u16; 512];
    let n = unsafe { GetWindowTextW(hwnd, &mut buf) };
    let n = n.clamp(0, buf.len() as i32) as usize;
    String::from_utf16_lossy(&buf[..n])
}

/// Put focus back where it was, instead of leaving it nowhere after a hide.
///
/// `SetForegroundWindow` only works for a thread that is already attached to the
/// foreground thread's input queue, hence the attach around it.
fn restore_focus(previous: HWND, hidden: HWND) {
    if previous.is_invalid() || previous == hidden || !unsafe { IsWindow(Some(previous)).as_bool() }
    {
        return;
    }

    let foreground_thread = unsafe { GetWindowThreadProcessId(GetForegroundWindow(), None) };
    let this_thread = unsafe { GetCurrentThreadId() };
    if foreground_thread != 0 {
        unsafe {
            let _ = AttachThreadInput(this_thread, foreground_thread, true);
        }
    }
    unsafe {
        let _ = SetForegroundWindow(previous);
    }
    if foreground_thread != 0 {
        unsafe {
            let _ = AttachThreadInput(this_thread, foreground_thread, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The classification the deferred re-check turns on: hidden windows are
    /// given back unless their title says they were never a real shell.
    #[test]
    fn blank_titles_stay_hidden() {
        for title in [
            r"C:\Windows\System32\cmd.exe",
            r"C:\Windows\System32\OpenSSH\ssh.EXE",
            r"d:/tools/node.exe",
        ] {
            assert_eq!(classify(title), Verdict::StayHidden, "{title}");
        }
        assert_eq!(classify(r"C:\a.exe"), Verdict::StayHidden);
    }

    #[test]
    fn placeholder_titles_are_undecided() {
        for title in ["", "Terminal", "Windows Terminal"] {
            assert_eq!(classify(title), Verdict::Undecided, "[{title}]");
        }
    }

    #[test]
    fn real_shell_titles_are_given_back() {
        for title in [
            "Command Prompt",
            r"Administrator: C:\Windows\System32\cmd.exe",
            r"pwsh - node",
            r"C:\Windows\System32",
            "notepad.exe",
            "C:",
        ] {
            assert_eq!(classify(title), Verdict::Restore, "{title}");
        }
    }

    /// A title is only a placeholder if it is nothing *but* a path to an exe.
    #[test]
    fn bare_exe_path_needs_a_drive_and_the_suffix() {
        assert!(is_bare_exe_path(r"C:\Windows\notepad.exe"));
        assert!(!is_bare_exe_path(r"C:\Windows\notepad.exe - Notepad"));
        assert!(!is_bare_exe_path(r"\Windows\notepad.exe"));
        assert!(!is_bare_exe_path("notepad.exe"));
        assert!(!is_bare_exe_path("2:\\x.exe"));
        assert!(!is_bare_exe_path("C:"));
        assert!(!is_bare_exe_path(""));
        assert!(!is_bare_exe_path(".exe"));
    }

    #[test]
    fn embedding_flag_decides_on_the_command_line() {
        assert!(command_line_has_embedding(
            r#""C:\Program Files\WindowsApps\Microsoft.WindowsTerminal\wt.exe" -Embedding"#
        ));
        assert!(command_line_has_embedding("wt.exe -embedding"));
        assert!(command_line_has_embedding(r"C:\wt.exe --Embedding Thing"));
        // Not the flag: a shell that merely mentions something like it.
        assert!(!command_line_has_embedding(""));
        assert!(!command_line_has_embedding("a"));
        assert!(!command_line_has_embedding(
            r"C:\Windows\System32\cmd.exe /c echo Embedding"
        ));
        assert!(!command_line_has_embedding(r"C:\Windows\System32\cmd.exe"));
    }
}
