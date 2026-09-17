# Privacy

Ternitor collects nothing, sends nothing on its own, and has no accounts, no
telemetry, no analytics, no crash reporting and no usage reporting of any kind.
Nothing about you or your machine leaves your machine by itself.

There is exactly one request in the program, and it happens only when you press
**Check for updates**: a `GET` of
`https://raw.githubusercontent.com/asterxsk/ternitor/main/Cargo.toml`, to read the
version number in the repo. Nothing about the machine goes with it beyond the
app's own version in the HTTP user agent -- no identifier, no cookie, no query
string. No other screen, no tray action and no startup path can open a socket,
and nothing is ever checked on a schedule.

## What it reads

To do its job, Ternitor has to look at what is happening on your own desktop:

- **Window class names and process command lines**, to tell a blank console that
  a console-less process spawned (which it hides) from a terminal you opened
  yourself (which it must never hide). The command line is how it checks for the
  default-terminal broker's `-Embedding` flag, and how it recognises a console
  host whose window class it does not know.
- **Window titles**, after the fact, to re-check a window it hid -- the window
  comes back only if the title is a shell's own: a directory it is sitting in, a
  prompt, an elevated console, or a shell by name.
- **Which window has the foreground**, so it can be given back: a hidden window
  that activates itself is hidden again, and the foreground returns to the window
  that last really had it.

All of that is examined in memory, on your machine. None of it is transmitted.
The only thing that can ever be written down is the log below, and that file
stays on your disk.

## What it writes

| Where | What | When |
|---|---|---|
| `ternitor.log`, beside the exe | One line per event: a window hidden, a window given back, a focus taken back, every window the gate looked at (`gate ...`, hidden or left alone), a check for updates and what it found, a setting changed. Window titles and the pid that owned them appear here. | While the app runs |
| `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` -> `Ternitor` | The path to the exe, so it starts at sign-in | Only while **Start with Windows** is on |
| `HKCU\...\Uninstall\` | Name, version and uninstall command, so Windows can list it in Settings > Apps | Written by the installer, or by `install.ps1`, which uses the key `...\Uninstall\Ternitor` |
| `...\Start Menu\Programs\Ternitor.lnk` | A shortcut | Written by the installer or by `install.ps1` |
| `%LOCALAPPDATA%\Programs\Ternitor\` | `Ternitor.exe`, and the uninstaller that removes it | Written by the installer, or by `install.ps1` |

Nothing is written anywhere else: not in `Documents`, not in `ProgramData`, not
in the registry outside those two keys. There is no cache and no database.

## Removing it

**Uninstall** in Settings > Apps stops the app and deletes the exe, the folder,
the shortcut, the Run entry and the Settings > Apps entry. `uninstall.ps1` asks
whether to keep the log on the way out; `Ternitor-Setup.exe`'s own uninstaller
takes it with everything else. The same is true by hand: delete the exe, the
`Ternitor` Run value, and the folder -- there is nothing else to clean up.

## Data requests

Because nothing is collected and nothing is transmitted, there is no data held
about you to access, correct, export or delete. If you believe that is wrong,
open an issue at <https://github.com/asterxsk/ternitor/issues> and say what you
are seeing.
