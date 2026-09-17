//! The one place this app asks the network anything.
//!
//! Pressed, never polled: the check runs when the settings surface's button is
//! clicked and at no other time, on its own thread, because the loop that hides
//! windows must never wait on a socket -- a blocked pump is a console that gets
//! to paint.
//!
//! The answer is the version in `Cargo.toml` on the repo's `main`: this app ships
//! from that tree, so that file is the release, and there is no separate release
//! process to fall out of step with it.

use std::ffi::c_void;
use std::sync::Mutex;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Networking::WinHttp::{
    INTERNET_DEFAULT_HTTPS_PORT, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_FLAG_SECURE,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE, WinHttpCloseHandle, WinHttpConnect,
    WinHttpOpen, WinHttpOpenRequest, WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse,
    WinHttpSendRequest, WinHttpSetTimeouts,
};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

use crate::win;

/// Where the answer lives. A raw file on the repo's own branch, so the check
/// needs no API token, no release, and no JSON.
const HOST: &str = "raw.githubusercontent.com";
const PATH: &str = "/asterxsk/ternitor/main/Cargo.toml";

/// Long enough for a slow connection, short enough that a dead one is over with
/// while the user is still looking at the button.
const TIMEOUT_MS: i32 = 5_000;

/// What the last check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    UpToDate,
    Newer(String),
    /// github did not answer, or answered with something unreadable.
    Unreachable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Idle,
    Checking,
    Done(Outcome),
}

static STATE: Mutex<State> = Mutex::new(State::Idle);

pub fn state() -> State {
    STATE.lock().map(|s| s.clone()).unwrap_or(State::Idle)
}

/// Start a check unless one is already in the air. `wake` is the app's hidden
/// window, posted to when the answer lands; the thread never touches the surface.
pub fn start(wake: isize) {
    match STATE.lock() {
        Ok(mut s) => {
            if *s == State::Checking {
                return;
            }
            *s = State::Checking;
        }
        Err(_) => return,
    }

    std::thread::spawn(move || {
        let outcome = match newest_version() {
            Some(newest) if is_newer(&newest, env!("CARGO_PKG_VERSION")) => {
                Outcome::Newer(newest)
            }
            Some(_) => Outcome::UpToDate,
            None => Outcome::Unreachable,
        };

        crate::log::write(&format!("update check: {}", said(&outcome)));
        if let Ok(mut s) = STATE.lock() {
            *s = State::Done(outcome);
        }
        unsafe {
            let _ = PostMessageW(
                Some(HWND(wake as *mut c_void)),
                crate::app::MSG_UPDATE,
                WPARAM(0),
                LPARAM(0),
            );
        }
    });
}

/// What the log says about a check, so the one network event this app has is
/// written down where the user can read it back.
pub fn said(outcome: &Outcome) -> String {
    match outcome {
        Outcome::UpToDate => format!("{} is the newest", env!("CARGO_PKG_VERSION")),
        Outcome::Newer(version) => format!("{version} is available"),
        Outcome::Unreachable => "no answer from github".into(),
    }
}

/// Where a newer build is: the repo root, because that is where the setup file and
/// the notes live. Opened only from the button, and only once the button is
/// already saying a newer version exists.
pub fn open_downloads() {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let verb = win::wide("open");
    let page = win::wide("https://github.com/asterxsk/ternitor");
    unsafe {
        ShellExecuteW(
            None,
            win::pcw(&verb),
            win::pcw(&page),
            None,
            None,
            SW_SHOWNORMAL,
        );
    }
}

fn newest_version() -> Option<String> {
    version_in(&get()?)
}

/// A plain GET. Nothing about the machine goes with it: the request is the
/// version of this app and the path of a public file.
fn get() -> Option<String> {
    let agent = win::wide(&format!("Ternitor/{}", env!("CARGO_PKG_VERSION")));
    let host = win::wide(HOST);
    let path = win::wide(PATH);
    let verb = win::wide("GET");

    unsafe {
        let session = WinHttpOpen(
            win::pcw(&agent),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            PCWSTR::null(),
            PCWSTR::null(),
            0,
        );
        if session.is_null() {
            return None;
        }
        let _ = WinHttpSetTimeouts(session, TIMEOUT_MS, TIMEOUT_MS, TIMEOUT_MS, TIMEOUT_MS);

        let connection = WinHttpConnect(session, win::pcw(&host), INTERNET_DEFAULT_HTTPS_PORT, 0);
        let request = if connection.is_null() {
            std::ptr::null_mut()
        } else {
            WinHttpOpenRequest(
                connection,
                win::pcw(&verb),
                win::pcw(&path),
                PCWSTR::null(),
                PCWSTR::null(),
                std::ptr::null(),
                WINHTTP_FLAG_SECURE,
            )
        };

        let body = if request.is_null() {
            None
        } else {
            let asked = WinHttpSendRequest(request, None, None, 0, 0, 0).is_ok()
                && WinHttpReceiveResponse(request, std::ptr::null_mut()).is_ok()
                && status(request) == 200;
            asked.then(|| read(request))
        };

        if !request.is_null() {
            let _ = WinHttpCloseHandle(request);
        }
        if !connection.is_null() {
            let _ = WinHttpCloseHandle(connection);
        }
        let _ = WinHttpCloseHandle(session);
        body
    }
}

fn status(request: *mut c_void) -> u32 {
    let mut code = 0u32;
    let mut length = std::mem::size_of::<u32>() as u32;
    unsafe {
        let _ = WinHttpQueryHeaders(
            request,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            PCWSTR::null(),
            Some(&mut code as *mut u32 as *mut c_void),
            &mut length,
            std::ptr::null_mut(),
        );
    }
    code
}

/// The body, in as few reads as the answer needs. A manifest is a few hundred
/// bytes; the cap is there so a wrong answer cannot grow without end.
fn read(request: *mut c_void) -> String {
    let mut out = Vec::new();
    let mut buf = [0u8; 1024];
    loop {
        let mut got = 0u32;
        let ok = unsafe {
            WinHttpReadData(
                request,
                buf.as_mut_ptr() as *mut c_void,
                buf.len() as u32,
                &mut got,
            )
        };
        if ok.is_err() || got == 0 {
            break;
        }
        out.extend_from_slice(&buf[..got as usize]);
        if out.len() >= 64 * 1024 {
            break;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The `version = "x.y.z"` line out of the `[package]` section. Other sections
/// carry versions of their own -- `windows = { version = "0.62" }` -- so the
/// section matters.
fn version_in(manifest: &str) -> Option<String> {
    let mut in_package = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if in_package {
            if let Some(value) = quoted(line, "version") {
                return Some(value);
            }
        }
    }
    None
}

fn quoted(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(key)?.trim_start().strip_prefix('=')?.trim();
    let rest = rest.strip_prefix('"')?;
    let (value, _) = rest.split_once('"')?;
    Some(value.to_string())
}

/// Numeric, component by component: `1.10.0` is newer than `1.9.0`, which
/// comparing the strings gets wrong. Anything unreadable counts as zero, so a
/// strange version in either place compares low rather than claiming to be new.
fn is_newer(candidate: &str, running: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> {
        v.split('.').map(|p| p.trim().parse().unwrap_or(0)).collect()
    };
    let (candidate, running) = (parts(candidate), parts(running));
    for i in 0..candidate.len().max(running.len()) {
        let (a, b) = (
            candidate.get(i).copied().unwrap_or(0),
            running.get(i).copied().unwrap_or(0),
        );
        if a != b {
            return a > b;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_comes_out_of_the_package_section() {
        assert_eq!(
            version_in("[package]\nname = \"ternitor\"\nversion = \"1.1.0\"\nedition = \"2021\"\n")
                .as_deref(),
            Some("1.1.0")
        );
        // The dependency's version is not the app's.
        assert_eq!(
            version_in("[package]\nversion = \"1.1.0\"\n\n[dependencies]\nwindows = \"0.62\"\n")
                .as_deref(),
            Some("1.1.0")
        );
        assert_eq!(version_in("[dependencies.windows]\nversion = \"0.62\"\n"), None);
        assert_eq!(version_in(""), None);
        assert_eq!(version_in("[package]\nname = \"ternitor\"\n"), None);
    }

    #[test]
    fn newer_is_numeric_not_alphabetical() {
        assert!(is_newer("1.1.1", "1.1.0"));
        assert!(is_newer("1.10.0", "1.9.0"));
        assert!(is_newer("2.0", "1.9.9"));
        assert!(is_newer("1.1.0.1", "1.1.0"));
        assert!(!is_newer("1.1.0", "1.1.0"));
        assert!(!is_newer("1.0.9", "1.1.0"));
        assert!(!is_newer("", "1.1.0"));
        assert!(!is_newer("junk", "1.1.0"));
    }

    /// The one test that leaves the machine. Run it by hand:
    /// `cargo test -- --ignored --nocapture says_what_github_says`
    #[test]
    #[ignore = "talks to github"]
    fn says_what_github_says() {
        let newest = newest_version().expect("github did not answer");
        println!("running {}  newest on main {newest}", env!("CARGO_PKG_VERSION"));
        assert!(
            newest.split('.').count() >= 2,
            "unreadable version from main: {newest}"
        );
    }
}
