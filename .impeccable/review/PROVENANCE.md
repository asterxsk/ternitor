# Raster provenance

Every image here is a grab of the running application's client area — no mockup,
no browser, no compositing. Each was taken at 100% display scale on Windows 11
build 26200, 2026-09-16, from
`ternitor.exe` SHA-256 `6e51439161e41572…` (the build currently shipped beside
`src/`).

The rasters themselves are not versioned — `.gitignore` excludes `*.png` here.
This file is the record; the images it describes live beside the running build.

| Raster | Build | State it records |
|---|---|---|
| `desktop.png` | `6e51439161e41572…` | A working session: 18 blank consoles hidden, counter reading `018`, **Start with Windows** armed (punched slug, grease tick, `ON`), keyboard focus on the punch (the square ring). |
| `desktop-empty.png` | `6e51439161e41572…` | The same surface five seconds after launch: counter `000`, `nothing cut yet`, **Start with Windows** off (intact strip, hollow slug, `OFF`). |

Both are 720×344 — the fixed logical client at 100% scale, and these two are
grabs of the client area only, so the window's own caption and rounded frame are
deliberately out of frame; the DWM caption tinting described in `DESIGN.md` is
best-effort and not visible in these two.

An earlier pair recorded the same two states at 720×428, when the surface still
carried the film rail; that rail was removed on 2026-09-16 and those captures no
longer describe the app.

States verified in captures but not shipped here, kept in `D:\Apps\tmp\ternitor\`:
`norail-on.png` and `norail-counted.png` (the armed switch and a live count on the
current build), and from the rail era, `rail-capped.png`, `r5-on-hover.png` /
`r5-on-press.png`, `r5-off-hover.png` / `r5-off-press.png`, and
`armed-no-focus.png`.

One state is **not** captured: the paused indicator. Pause is reachable only
through the tray's popup menu, and a popup cannot be driven from outside without
UI automation; a keyboard harness walked the menu into its own Exit item twice
instead. The label is covered by `ui::tests::the_paused_label_fits_the_counter_column`,
which measures the real strings with the real font against the real column width,
and it reuses the same right-aligned label path the counter's own label is drawn
with. It has not been seen on screen.
