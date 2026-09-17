# Product

<!-- impeccable:product-schema 1 -->

## Platform

windows

(Native Win32 desktop app. Not the `web | ios | android | adaptive` enum: it is a
single-OS native surface, not a website and not one product adapting per OS.)

The crate builds on Windows only: there is no platform abstraction over the
Win32 modules and `windows` is a plain dependency. The defect it fixes is
Windows': a ConPTY client with no console spawns a child, the child gets a brand
new console, and the default-terminal broker hands that console to Windows
Terminal as a visible window. Nothing on Linux or macOS opens a terminal behind
the user's back, so there is no `janitor` to port and no honest resident process
to leave running. Decided 2026-09-16, over porting or doing nothing; the Linux and
macOS stub was deleted 2026-09-17 rather than kept as a build that pretends.

## Stack

user-specified: Rust, `windows` 0.62 crate for raw Win32 bindings, no GUI
framework or runtime dependency. Ships as one portable `.exe`. Autostart is an
`HKCU\...\Run` entry the app writes itself.

Shipped: compiled as a GUI-subsystem binary (`windows_subsystem = "windows"`,
PE subsystem 2), so it can never allocate a console and therefore cannot flash
one at logon -- the problem the PowerShell launcher solved with a `wscript` shim
and a headless `conhost`. The settings surface is raw GDI drawn into a double
buffer, with no toolkit; there is no HTML, CSS or web view anywhere in the app.
The surface is **dark-only** -- the user declined a light theme rather than
having one shipped unverified, so no light palette exists in the code.

## Users

One user: a developer on this Windows 11 machine, working in a terminal-hosted
coding agent (T3 Code). Their agents (Claude Code, opencode, codex) run as ConPTY
clients with no console, so every child they spawn -- `node`, `wslhost`, hooks,
MCP servers -- gets a brand-new console that the default-terminal broker hands to
Windows Terminal as a blank window. The job being done when Ternitor matters is
"concentrating on a task the agent is running"; the interruption is the failure.

## Product Purpose

Keep agent-spawned console windows from ever appearing, and take the focus back
when one of them activates itself anyway, so neither can break concentration.
Success is the user forgetting it is running: no window ever appears, no
notification, no required configuration. The app exists because the fix cannot
live in the spawner -- window visibility is the spawner's `CREATE_NO_WINDOW`
decision, and the spawners are vendored hooks that cannot be edited.

## Positioning

It hides only the windows that are structural artefacts of console-less spawners,
identified by the default-terminal broker's `-Embedding` flag on the owning
process -- not by process name, not by window title. A terminal the human opened
is never touched, and anything hidden is re-checked and restored if its title
reveals it was a real shell. The same holds the other way: a hidden window that
activates itself is hidden again, and the foreground goes back to the window that
last really had it, because a hidden window holding focus leaves the user typing
into nothing. A tool that closed or hid "all terminal windows" could not
truthfully copy that claim.

## Operating Context

Windows 11 (build 26200), Windows Terminal set as the default terminal, Agent
hosts run as ConPTY clients. Ternitor sits in the notification area, autostarts at
logon, and must itself never flash a console -- it is the thing that hides
consoles, so nothing is hiding its own at boot.

## Capabilities and Constraints

- Single instance, enforced by a named mutex.
- Tray icon with a small menu; one settings window, opened from the tray.
- The settings window carries exactly four things: a "Start with Windows" toggle,
  a live count of windows hidden this session, app info describing what the app
  does, and Quit, which ends the process outright -- the same thing the tray menu
  offers, for when the tray icon is buried. Behaviour beyond that is not
  configurable (confirmed: no pause toggle, no restore-shells toggle, no log
  toggle on the screen). Quit added 2026-09-16, the only control added since the
  Rust build.
- Event-driven (`SetWinEventHook`), never polling.
- No runtime dependency, config in the registry, log next to the exe. Still
  portable -- copy the folder anywhere -- and now also installable:
  `Ternitor-Setup.exe` (Inno Setup, built by `installer\build.ps1`) or
  `install.ps1` puts it in `%LOCALAPPDATA%\Programs\Ternitor` with a Start Menu
  shortcut, the `Run` value, and an entry in Settings > Apps, where the
  installer's own uninstaller -- or `uninstall.ps1` -- removes all of it.
- The app mark (a Node hexagon with a red X, on a dark tile) is defined once in
  `src/mark.rs`; `build.rs` bakes the exe's `.ico` from it at build time and
  `assets/icon.svg` is generated from the same constants, so the tray, the exe
  icon and the SVG cannot drift apart. Replaced the ported PowerShell mark on
  2026-09-16, which retires the old byte-identical-at-16px parity claim.
- Behaviour parity with the PowerShell implementation is otherwise verified, and
  the PowerShell version (`ternitor.ps1`, its installer, the Startup `.vbs`) is
  deleted, per the user's decision. The `install.ps1` in the tree today is the
  Rust build's installer, not the deleted one. `src/` is the only implementation.
  The stale Startup `.vbs` the old installer left on this machine was removed on
  2026-09-16; only the `Run` value remains.

## Evidence on Hand

- `src/` -- the only implementation. `janitor.rs` holds the detection, the
  deferred re-show, and the foreground hook that takes focus back, `ui.rs` the
  settings surface, `mark.rs` the app mark (Node hexagon + red X on a dark tile),
  `icon.rs` the Win32 handle over it.
- `PRIVACY.md` and `TERMS.md` -- added 2026-09-16 at the user's request. The
  privacy statement is factual (no network code, two registry values, one local
  log); the terms defer to MIT rather than manufacturing a contract, and say so.
- `ternitor.log` -- real hide/re-show/focus-taken-back history from this machine,
  still written by the Rust build.
- Measurements on this machine. PowerShell version: 160 MB working set, 72 MB
  private bytes, 986 ms start, 0 ms idle CPU over 5 s. Rust build: 17.4 MB working
  set, 2.8 MB private, ~10 ms start, 0.06 s CPU, and 0% idle CPU -- the surface
  does not animate and runs no repaint timer, so an idle Ternitor has nothing to
  spend CPU on at all.

## Product Principles

1. Never interrupt. Refusing to hide a window is a smaller failure than hiding one
   the user needed, but appearing on screen unbidden is the failure this app exists
   to prevent.
2. Invisible by default. Every setting is a cost; only add one that changes what
   the app does in a way the user cannot get another way.
3. Never touch a terminal a human opened.
4. Zero ceremony: one file, no runtime, no first-run flow. Installing is optional
   — the exe runs where it stands, and `Ternitor-Setup.exe` (or `install.ps1`)
   adds only the shell integration that Settings > Apps and autostart need.
