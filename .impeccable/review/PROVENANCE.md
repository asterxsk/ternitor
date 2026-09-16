# Raster provenance

Every image here is a grab of the running application's client area — no mockup,
no browser, no compositing. Each was taken at 100% display scale on Windows 11
build 26200, 2026-09-16, from
`ternitor.exe` SHA-256 `821dc056d37e959a…` (the build currently shipped beside
`src/`).

The rasters themselves are not versioned — `.gitignore` excludes `*.png` here.
This file is the record; the images it describes live beside the running build.

| Raster | Build | State it records |
|---|---|---|
| `desktop.png` | `821dc056d37e959a…` | A working session: 8 blank consoles hidden, counter reading `008`, **Start with Windows** armed (punched slug, grease tick, `ON`), newest frame carrying its cut mark, keyboard focus on the punch (the square ring). |
| `desktop-empty.png` | `821dc056d37e959a…` | The same surface five seconds after launch: counter `000`, rail unlit, `nothing cut yet`, **Start with Windows** off (intact strip, hollow slug, `OFF`). |

Both are 720×428 — the fixed logical client at 100% scale — so the window's own
caption and rounded frame are deliberately out of frame; the DWM caption tinting
described in `DESIGN.md` is best-effort and not visible in these two.

States verified in captures but not shipped here, kept in `D:\Apps\tmp\ternitor\`:
`rail-capped.png` (34 hidden — the rail saturated at thirteen cells with the
oldest clipped by the window edge), `r5-on-hover.png` / `r5-on-press.png` (the
armed switch growing and insetting its slug), `r5-off-hover.png` /
`r5-off-press.png`, and `armed-no-focus.png`.

One state is **not** captured: the paused indicator. Pause is reachable only
through the tray's popup menu, and a popup cannot be driven from outside without
UI automation; a keyboard harness walked the menu into its own Exit item twice
instead. The label is covered by `ui::tests::the_paused_label_fits_the_counter_column`,
which measures the real strings with the real font against the real column width,
and it reuses the same right-aligned label path the counter's own label is drawn
with. It has not been seen on screen.
