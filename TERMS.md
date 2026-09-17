# Terms of use

Ternitor is free software under the [MIT License](LICENSE). That licence is the
whole of the grant, the whole of the warranty position, and the whole of the
liability position -- nothing in this file adds to it, takes from it, or
overrides it. If anything here appears to conflict with `LICENSE`, the licence
wins.

## What you are agreeing to by running it

- **It modifies your system, by design.** It writes one value under
  `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` when you turn on
  *Start with Windows*, it changes the visibility of windows on your desktop, and
  pressing *Check for updates* asks github for one public file. Full list in
  [PRIVACY.md](PRIVACY.md).
- **Hiding a window is best effort.** Ternitor hides console windows it decides
  were spawned by a console-less process, and it can be wrong in both
  directions: it may miss a window, or hide one you wanted. It re-checks every
  window it hides and gives back only those whose title is a shell's own -- a
  directory, a prompt, an elevated console, or a shell by name -- and it hides
  again anything that activates itself while hidden, returning the foreground to
  the window that last had it, but there is no guarantee. If a window you need
  disappears, use *Pause hiding* on the tray menu.
- **It is not a security product.** It does not sandbox anything, does not
  inspect what other processes do, and is not a substitute for antivirus,
  endpoint protection, or an approval step.

## What you are not getting

No warranty of any kind, express or implied, including merchantability or
fitness for a particular purpose. No guarantee of support, updates, or that any
particular version keeps working on any particular build of Windows. The
authors are not liable for any damages arising from the use of this software.
That is what the MIT License says, in the standard words, and it is the whole
answer.

## This file

This is a plain-language statement, not a contract drafted by a lawyer, and
nothing in it is legal advice. It deliberately names no jurisdiction and no
governing law, because inventing one would suggest a formality this project does
not have: Ternitor is a free tool from an individual, distributed as source, and
the licence it ships under is the document that matters.

Questions: <https://github.com/asterxsk/ternitor/issues>
