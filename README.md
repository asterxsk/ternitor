<div align="center">

<img src="assets/readme/header.svg" alt="Ternitor, animated: a blank console window opens over the Ternitor settings screen and everything dims behind it, the Start with Windows switch flips to ON, the console disappears, and the hidden counter steps from 000 to 003" width="920">

**Hides the blank console windows Windows opens for other people's processes.**
One file, no installer, no runtime, no telemetry — and nothing visible once it is running.

[![License: MIT](https://img.shields.io/badge/license-MIT-3fb950?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-windows%2011-58a6ff?style=flat-square)](#requirements)
[![Rust](https://img.shields.io/badge/rust-2021-d29922?style=flat-square&logo=rust&logoColor=white)](#build-it)
[![Telemetry](https://img.shields.io/badge/telemetry-none-7ee787?style=flat-square)](#privacy)
[![Size](https://img.shields.io/badge/size-176%20KB-8b949e?style=flat-square)](#files)

</div>

---

Your agent goes to run something, and a blank terminal window appears on the desktop, takes focus
mid-sentence, and sits there doing nothing. Dismiss it and the next tool call opens another one.
Ternitor watches for those windows and hides them before they are ever drawn — every agent, every
tool, silently, from the notification area.

## Why the windows appear

Agent hosts — Claude Code, opencode, codex under T3 Code — run as **ConPTY clients, which have no
console**. Any child they spawn with default creation flags therefore gets a brand-new console, and
the Windows default-terminal broker hands that console to Windows Terminal as a visible window.

This is structural, not a bug in any one tool, and no flag on the child can prevent it: window
visibility is the **spawner's** `CREATE_NO_WINDOW` decision, and the spawners are vendored hooks and
MCP servers that cannot be edited.

One dead end, recorded so nobody repeats it: a `node` shim on `PATH` can never shadow the real one,
because `C:\Program Files\nodejs` is on Machine `PATH` and User `PATH` is appended after it.

## Quickstart

```
ternitor.exe
```

That is the whole install. The app starts in the notification area, hides the next blank console it
sees, and stays out of the way.

**Left-click the tray icon** for Settings. **Right-click** for Settings, Pause hiding, Open log, Exit.

To have it start at logon, open Settings and flip **Start with Windows**. That writes one value under
`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`. Delete the exe and that value and nothing is
left behind.

## What you get

| | |
|---|---|
| **Hides the window** | Before it paints, not after — no flash, no focus steal, no flicker in the taskbar |
| **Knows the difference** | A terminal *you* opened from `Win+R` or a `wt` alias looks identical at that instant. Each hide is re-checked and re-shown, without activation, once its title reveals it was a real shell |
| **Counts its work** | The settings screen carries a live count of what it has hidden this session, and the title of the last one |
| **Refuses to grow** | Three things on the settings screen and nothing else. Everything that is not a setting lives on the tray menu |
| **Says nothing** | No network calls, no telemetry, no update check, no crash reporting |

## How it works

```
agent spawns a child ─▶ new console, no window yet ─▶ default-terminal broker
                                                              │
                        SetWinEventHook ◀──────────────────────┘
                                │
                class is CASCADIA_HOSTING_WINDOW_CLASS?
                                │
                        owning process command line contains -Embedding?
                                │
                                ▼
                     hidden before it is drawn
```

A `SetWinEventHook` on `EVENT_OBJECT_CREATE` and `EVENT_OBJECT_SHOW` watches for new windows. A
window is hidden only if its **class** is `CASCADIA_HOSTING_WINDOW_CLASS` **and** the command line of
the process that owns it contains `-Embedding` — a flag set by the default-terminal broker and
nothing else, read with `NtQueryInformationProcess`. Matching the window *title* does not work: at
creation time it is still the broker's placeholder, and only later becomes the client's path.

Event-driven, no polling, and a terminal you opened yourself is never touched.

## Settings

The settings window is the app's only surface, opened from the tray and perhaps once a session. It
carries three things and refuses to carry a fourth: **Start with Windows**, a live count of the
windows hidden this session, and app info.

Everything else stays on the tray menu, because a utility whose value is *not being noticed* should
not grow a control panel.

What it looks like is documented in [DESIGN.md](DESIGN.md). The surface is a film cutting bench — a
perforated rail of blank frames, a mechanical counter, a grease pencil — and every frame on it is
blank, because the window it is counting is by definition a window with nothing in it.

## Requirements

- **Windows 11, x64.** No runtime, no redistributable, no Node, no PowerShell.
- **176 KB.** One exe. Nothing is installed and nothing is written outside its own folder and one
  registry value you can see and delete.
- **Nothing at startup.** The build is a GUI-subsystem binary (PE subsystem 2), so it can never
  allocate a console — which is what stops it flashing one of its own at logon. An earlier scripting
  version needed a `wscript` shim and a headless `conhost` to achieve the same thing; a compiled app
  gets it for free.
- **No GPU, no service, no scheduled task, no driver.**

## Privacy

Nothing leaves the machine. There is no network code in the binary at all — no telemetry, no
analytics, no update check. The one file it writes is `ternitor.log` beside the exe: every hide, every
re-show, every toggle. Read it, or delete it while the app is running.

## Build it

```
cargo build --release
```

Rust 2021, one dependency (the `windows` crate). The binary lands at `target\release\ternitor.exe`.

The `ternitor.exe` committed here is that build copied up, so it goes stale whenever `src/` moves.
Rebuild before trusting it.

## Files

| File | |
|---|---|
| `ternitor.exe` | the app; run it directly |
| `ternitor.log` | created next to the exe on first run |
| `src/` | `janitor.rs` detection, `ui.rs` the settings surface, `tray.rs` the icon |
| `assets/readme/` | the animated header on this page, as SVG |

## Moving it

The folder is self-contained and location-independent — the log is written beside the exe rather than
under AppData, and nothing stores an absolute path. Copy it anywhere.

If **Start with Windows** is on, flip it off and on again from the new location so the `Run` value
points at the copy you kept.

## Documentation

- [DESIGN.md](DESIGN.md) — the settings surface: palette, type, layout, and the rules behind them
- [PRODUCT.md](PRODUCT.md) — what the product is for, and what it deliberately does not do

## License

MIT. See [LICENSE](LICENSE).

<div align="center">
<sub>Every frame on the rail is blank. That is the point.</sub>
</div>
