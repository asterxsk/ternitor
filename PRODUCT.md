# Product

<!-- impeccable:product-schema 1 -->

## Platform

windows

(Native Win32 desktop app. Not the `web | ios | android | adaptive` enum: it is a
single-OS native surface, not a website and not one product adapting per OS.)

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

Keep agent-spawned console windows from ever appearing, so they cannot steal
focus or break concentration. Success is the user forgetting it is running: no
window ever appears, no notification, no required configuration. The app exists
because the fix cannot live in the spawner -- window visibility is the spawner's
`CREATE_NO_WINDOW` decision, and the spawners are vendored hooks that cannot be
edited.

## Positioning

It hides only the windows that are structural artefacts of console-less spawners,
identified by the default-terminal broker's `-Embedding` flag on the owning
process -- not by process name, not by window title. A terminal the human opened
is never touched, and anything hidden is re-checked and restored if its title
reveals it was a real shell. A tool that closed or hid "all terminal windows"
could not truthfully copy that claim.

## Operating Context

Windows 11 (build 26200), Windows Terminal set as the default terminal, Agent
hosts run as ConPTY clients. Ternitor sits in the notification area, autostarts at
logon, and must itself never flash a console -- it is the thing that hides
consoles, so nothing is hiding its own at boot.

## Capabilities and Constraints

- Single instance, enforced by a named mutex.
- Tray icon with a small menu; one settings window, opened from the tray.
- The settings window carries exactly three things: a "Start with Windows" toggle,
  a live count of windows hidden this session, and app info describing what the
  app does. Behaviour beyond that is not configurable (confirmed: no pause
  toggle, no restore-shells toggle, no log toggle on the screen).
- Event-driven (`SetWinEventHook`), never polling.
- Portable: no installer, no runtime dependency, config in the registry, log next
  to the exe.
- Behaviour parity with the PowerShell implementation is verified and the
  PowerShell version (`ternitor.ps1`, `install.ps1`, the Startup `.vbs`) is
  deleted, per the user's decision. `src/` is now the only implementation; the
  tray icon it produces is byte-identical to the original at 16px and 32px.

## Evidence on Hand

- `src/` -- the only implementation. `janitor.rs` holds the detection and
  deferred re-show, `ui.rs` the settings surface, `icon.rs` the tray mark
  (Node hexagon + red X, ported from the old rasteriser).
- `ternitor.log` -- real hide/re-show history from this machine, still written by
  the Rust build.
- Measurements on this machine. PowerShell version: 160 MB working set, 72 MB
  private bytes, 986 ms start, 0 ms idle CPU over 5 s. Rust build: 17.4 MB working
  set, 2.8 MB private, ~10 ms start, 0.06 s CPU, and 0% idle CPU -- the repaint
  timer stops itself the moment the cut animation lands.

## Product Principles

1. Never interrupt. Refusing to hide a window is a smaller failure than hiding one
   the user needed, but appearing on screen unbidden is the failure this app exists
   to prevent.
2. Invisible by default. Every setting is a cost; only add one that changes what
   the app does in a way the user cannot get another way.
3. Never touch a terminal a human opened.
4. Zero ceremony: one file, no installer, no runtime, no first-run flow.
