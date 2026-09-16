// A console only while developing; the shipped binary is a GUI-subsystem app so
// it can never allocate a console -- Ternitor's whole job is hiding consoles,
// and it must not be the one app that still flashes one.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

// Every module below is Win32. There is no platform abstraction over them and
// there is not meant to be one: see `elsewhere` for why the other platforms get
// an honest answer instead of a hollow port.
#[cfg(windows)]
mod app;
#[cfg(windows)]
mod autostart;
#[cfg(windows)]
mod icon;
#[cfg(windows)]
mod janitor;
#[cfg(windows)]
mod log;
#[cfg(windows)]
mod mark;
#[cfg(windows)]
mod theme;
#[cfg(windows)]
mod tray;
#[cfg(windows)]
mod ui;
#[cfg(windows)]
mod win;

#[cfg(windows)]
fn main() {
    std::process::exit(app::run());
}

#[cfg(not(windows))]
fn main() {
    std::process::exit(elsewhere());
}

/// What this program is on a system whose terminal windows are not opened behind
/// the user's back.
///
/// Ternitor exists because of one Windows mechanism: an agent host runs as a
/// ConPTY client with no console, so every process it spawns gets a brand-new
/// console, and the default-terminal broker hands that console to Windows
/// Terminal as a visible window. Nothing on Linux or macOS does that -- a
/// terminal appears there because a person opened one -- so there is nothing to
/// hide, no `janitor` equivalent to port, and a resident process pretending
/// otherwise would be a lie with a tray icon attached.
#[cfg(not(windows))]
fn elsewhere() -> i32 {
    // `args_os`, not `args`: on Unix a non-UTF-8 argument makes `args` panic, and
    // a binary whose whole job here is to do nothing should not be able to abort
    // with a backtrace for being handed a filename it cannot spell.
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    let has = |flag: &str| args.iter().any(|a| a.to_str() == Some(flag));

    if has("--version") || has("-V") {
        println!("ternitor {} ({})", env!("CARGO_PKG_VERSION"), std::env::consts::OS);
        return 0;
    }
    if has("--help") || has("-h") {
        println!("Usage: ternitor");
        println!();
        println!("Ternitor hides the blank console windows Windows opens for processes that");
        println!("have no console of their own. This build runs on {}, where Windows does", std::env::consts::OS);
        println!("not open those windows, so it has nothing to do. Run it with --why.");
        return 0;
    }
    if has("--why") {
        print_why();
        return 0;
    }

    print_why();
    0
}

#[cfg(not(windows))]
fn print_why() {
    println!(
        "Ternitor does nothing on {os}: there is nothing here to hide.

The windows it hides are made by Windows itself. An agent host -- Claude Code,
opencode, codex -- runs as a ConPTY client with no console, so any process it
spawns gets a brand-new console, and the default-terminal broker hands that
console to Windows Terminal as a visible window that steals focus. No flag on
the child can prevent it, because window visibility is the spawner's decision.

{os} has no equivalent mechanism: a terminal opens there because a person opened
one. So this build is not a port of the Windows one and does not pretend to be.
There is no background process, no tray icon and no autostart entry here, and
nothing is written to disk.

The Windows build is the app. See README.md.",
        os = std::env::consts::OS
    );
}
