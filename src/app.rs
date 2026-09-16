//! Application state, the hidden window that receives everything, and the loop.
//!
//! The app is deliberately single-threaded: the window-event hook, the tray icon
//! and the settings window all live on one thread, so the hook callback can touch
//! state directly and nothing needs a lock or a channel. Everything is reached
//! through `APP`, a thread-local; the Win32 callbacks are free functions that
//! borrow it for the duration of one call.

use std::cell::RefCell;
use std::time::Instant;

use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, HWND_MESSAGE, MSG,
    PostQuitMessage, RegisterClassW, TranslateMessage, WM_APP, WM_DESTROY, WM_LBUTTONDBLCLK,
    WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSW, WINDOW_EX_STYLE, WINDOW_STYLE,
};

use crate::win::{already_exists, instance, pcw, wide};
use crate::{autostart, janitor, log, tray, ui};

/// Tray icon notifications arrive on the app's hidden window under this message
/// number, with the mouse event in `lparam` -- classic semantics, see
/// `tray::Tray::add`.
pub const MSG_TRAY: u32 = WM_APP + 1;

/// Everything the settings screen renders, read in one go so the UI never holds
/// a borrow of live state while it draws.
pub struct Snapshot {
    pub paused: bool,
    pub hidden: u64,
    pub autostart: bool,
    pub last_title: String,
    pub session_start: Instant,
}

pub struct App {
    janitor: Option<janitor::Janitor>,
    tray: Option<tray::Tray>,
    session_start: Instant,
    /// The title of the last window hidden. The app tracks it from the events it
    /// receives rather than reaching into the janitor for it.
    last_title: String,
}

impl App {
    fn new() -> Self {
        App {
            janitor: None,
            tray: None,
            session_start: Instant::now(),
            last_title: String::new(),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let (paused, hidden) = match &self.janitor {
            Some(j) => (j.paused(), j.hidden_count()),
            None => (false, 0),
        };
        Snapshot {
            paused,
            hidden,
            autostart: autostart::is_enabled(),
            last_title: self.last_title.clone(),
            session_start: self.session_start,
        }
    }

    /// The janitor owns the detection and its own log lines. All the app does
    /// here is remember what happened and repaint, because the count and the
    /// figure both just changed.
    fn on_event(&mut self, ev: janitor::Event) {
        if let janitor::Event::Hidden { title } = ev {
            self.last_title = title;
        }
        ui::refresh();
    }

    pub fn toggle_pause(&mut self) {
        if let Some(j) = &mut self.janitor {
            let now = !j.paused();
            j.set_paused(now);
            log::write(if now { "paused" } else { "resumed" });
            self.refresh_tooltip();
            ui::refresh();
        }
    }

    /// Returns the reason it did not take, for the surface to show. The window is
    /// only repainted -- nothing animates, so a repaint is the whole update.
    pub fn set_autostart(&mut self, on: bool) -> Result<(), String> {
        let r = autostart::set(on);
        match &r {
            Ok(()) => log::write(if on {
                "autostart enabled"
            } else {
                "autostart disabled"
            }),
            Err(e) => log::write(&format!("autostart toggle failed: {e}")),
        }
        ui::refresh();
        r
    }

    /// The tooltip carries the two facts worth knowing without opening anything.
    pub fn refresh_tooltip(&self) {
        let Some(t) = &self.tray else { return };
        let s = self.snapshot();
        if s.paused {
            t.set_tooltip("Ternitor - paused");
        } else {
            t.set_tooltip(&format!("Ternitor - {} hidden this session", s.hidden));
        }
    }

    pub fn quit(&mut self) {
        if let Some(t) = &mut self.tray {
            t.remove();
        }
        ui::close();
        unsafe { PostQuitMessage(0) };
    }

    pub fn open_log(&self) {
        if let Some(p) = log::path() {
            log::reveal(p);
        }
    }
}

thread_local! {
    static APP: RefCell<App> = RefCell::new(App::new());
}

/// Run a closure with mutable access to the app state.
pub fn with<R>(f: impl FnOnce(&mut App) -> R) -> R {
    APP.with(|a| f(&mut a.borrow_mut()))
}

pub fn snapshot() -> Snapshot {
    with(|a| a.snapshot())
}

pub fn toggle_pause() {
    with(|a| a.toggle_pause())
}

pub fn set_autostart(on: bool) -> Result<(), String> {
    with(|a| a.set_autostart(on))
}

pub fn quit() {
    with(|a| a.quit())
}

pub fn open_log() {
    with(|a| a.open_log())
}

/// The tray notification handler: the shell posts `MSG_TRAY` to our hidden
/// window with the mouse event in `lparam`.
fn on_tray_message(lparam: LPARAM) {
    match lparam.0 as u32 {
        WM_LBUTTONUP | WM_LBUTTONDBLCLK => ui::open(),
        WM_RBUTTONUP => {
            let paused = snapshot().paused;
            let choice = with(|a| a.tray.as_ref().and_then(|t| t.show_menu(paused)));
            match choice {
                Some(tray::Choice::OpenSettings) => ui::open(),
                Some(tray::Choice::TogglePause) => toggle_pause(),
                Some(tray::Choice::OpenLog) => open_log(),
                Some(tray::Choice::Exit) => quit(),
                None => {}
            }
        }
        _ => {}
    }
}

unsafe extern "system" fn message_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        MSG_TRAY => {
            on_tray_message(lparam);
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// A message-only window: invisible, never painted, just somewhere for the shell
/// and the hook to post to.
fn create_hidden_window() -> Result<HWND, String> {
    let class = wide("TernitorMessageWindow");
    let wc = WNDCLASSW {
        lpfnWndProc: Some(message_proc),
        hInstance: instance(),
        lpszClassName: pcw(&class),
        ..Default::default()
    };
    if unsafe { RegisterClassW(&wc) } == 0 {
        return Err("RegisterClassW failed".into());
    }
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            pcw(&class),
            pcw(&wide("Ternitor")),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(instance()),
            None,
        )
    }
    .map_err(|e| format!("CreateWindowExW failed: {e}"))
}

/// Start everything and run until the user exits. Returns a process exit code.
pub fn run() -> i32 {
    unsafe {
        // Per-monitor v2 before any window exists, or the settings sheet is
        // bitmap-stretched on a scaled display and every hairline goes soft.
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    let exe = std::env::current_exe().unwrap_or_default();
    let dir = exe.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    log::init(dir.join("ternitor.log"));

    // One instance: a second Ternitor would double-hide and fight over the tray.
    let name = wide(r"Local\Ternitor.SingleInstance");
    let _mutex: HANDLE = unsafe { CreateMutexW(None, true, pcw(&name)) }.unwrap_or_default();
    if already_exists() {
        log::write("another instance is already running; exiting");
        return 0;
    }

    let hwnd = match create_hidden_window() {
        Ok(h) => h,
        Err(e) => {
            log::write(&format!("startup failed: {e}"));
            return 1;
        }
    };

    // The hook goes in first: a console spawned between now and the tray icon
    // appearing is still a console the user should never have seen.
    let janitor = match janitor::Janitor::install(Box::new(|ev| with(|a| a.on_event(ev)))) {
        Ok(j) => Some(j),
        Err(e) => {
            log::write(&format!("hook failed: {e}"));
            None
        }
    };

    let tray = match tray::Tray::add(hwnd, MSG_TRAY, "Ternitor") {
        Ok(t) => Some(t),
        Err(e) => {
            log::write(&format!("tray failed: {e}"));
            None
        }
    };

    with(|a| {
        a.janitor = janitor;
        a.tray = tray;
    });
    with(|a| a.refresh_tooltip());

    log::write(&format!("Ternitor started (pid {})", std::process::id()));

    let mut msg = MSG::default();
    loop {
        // GetMessageW returns -1 on error; as_bool() would read that as "keep
        // going" and spin, so compare explicitly.
        let r = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if r.0 <= 0 {
            break;
        }
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    log::write("exiting");
    0
}
