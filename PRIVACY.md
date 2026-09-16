# Privacy

Ternitor collects nothing, sends nothing, and has no network code in it at all.
There is no telemetry, no analytics, no update check, no crash reporting, no
account, and no third party of any kind. Nothing about you or your machine
leaves your machine, because there is nowhere in this program for it to go.

That is not a policy promise, it is the shape of the program: `ternitor.exe` is
one binary with no HTTP client and no URLs in it.

## What it reads

To do its job, Ternitor has to look at what is happening on your own desktop:

- **Window class names and process command lines**, to tell a blank console that
  a console-less process spawned (which it hides) from a terminal you opened
  yourself (which it must never hide). The command line is how it checks for the
  default-terminal broker's `-Embedding` flag.
- **Window titles**, after the fact, to re-check a window it hid -- a title that
  turns out to be a real shell gets the window back.

All of that is examined in memory, on your machine. None of it is transmitted.
The only thing that can ever be written down is the log below, and that file
stays on your disk.

## What it writes

| Where | What | When |
|---|---|---|
| `ternitor.log`, beside the exe | One line per event: a window hidden, a window given back, a setting changed. Hidden-window titles appear here. | While the app runs |
| `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` -> `Ternitor` | The path to the exe, so it starts at sign-in | Only while **Start with Windows** is on |
| `HKCU\...\Uninstall\Ternitor` | Name, version and uninstall command, so Windows can list it in Settings > Apps | Written by `install.ps1` |
| `...\Start Menu\Programs\Ternitor.lnk` | A shortcut | Written by `install.ps1` |

Nothing is written anywhere else: not in `Documents`, not in `ProgramData`, not
in the registry outside those two keys. There is no cache and no database.

## Removing it

`uninstall.ps1` stops the app and deletes the exe, the folder, the shortcut, the
Run entry and the Settings > Apps entry, and asks whether to delete the log. The
same is true by hand: delete the exe, the `Ternitor` Run value, and the folder --
there is nothing else to clean up.

## Data requests

Because nothing is collected and nothing is transmitted, there is no data held
about you to access, correct, export or delete. If you believe that is wrong,
open an issue at <https://github.com/asterxsk/ternitor/issues> and say what you
are seeing.
