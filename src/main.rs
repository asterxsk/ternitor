// A console only while developing; the shipped binary is a GUI-subsystem app so
// it can never allocate a console -- Ternitor's whole job is hiding consoles,
// and it must not be the one app that still flashes one.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Every module below is Win32. There is no platform abstraction over them and
// there is not meant to be one: the windows these hide are made by Windows, and
// nothing else has the mechanism that makes them.
mod app;
mod autostart;
mod icon;
mod janitor;
mod log;
mod mark;
mod theme;
mod tray;
mod ui;
mod update;
mod win;

fn main() {
    std::process::exit(app::run());
}
