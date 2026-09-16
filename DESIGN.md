---
name: Ternitor
description: A bench instrument panel: a mechanical counter, a punch, and a grease pencil.
colors:
  ground: "#000000"
  speck: "#121212"
  dust: "#2C2C2C"
  edge: "#2E2E2E"
  rule: "#262626"
  dim: "#8A8A8A"
  body: "#C0C0C0"
  ink: "#FFFFFF"
  grease: "#FF6A13"
typography:
  title:
    fontFamily: "Franklin Gothic Medium, Segoe UI Variable Display, Segoe UI, sans-serif"
    fontSize: "21px"
    fontWeight: 400
    lineHeight: 1.1
    letterSpacing: "normal"
  label:
    fontFamily: "Arial Narrow, Segoe UI, sans-serif"
    fontSize: "11.5px"
    fontWeight: 700
    lineHeight: 1
    letterSpacing: "1.4px"
  body:
    fontFamily: "Franklin Gothic Book, Segoe UI Variable Text, Segoe UI, sans-serif"
    fontSize: "13.5px"
    fontWeight: 400
    lineHeight: 1.48
    letterSpacing: "normal"
  small:
    fontFamily: "Arial Narrow, Segoe UI, sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.42
    letterSpacing: "normal"
  mono:
    fontFamily: "Cascadia Mono, Consolas, monospace"
    fontSize: "12.5px"
    fontWeight: 400
    lineHeight: 1.2
    letterSpacing: "normal"
  counter:
    fontFamily: "Cascadia Mono, Consolas, monospace"
    fontSize: "34px"
    fontWeight: 600
    lineHeight: 1
    letterSpacing: "normal"
rounded:
  punch: "4px"
  slug: "2px"
spacing:
  pad: "44px"
  gutter: "24px"
components:
  counter-wheel:
    backgroundColor: "{colors.ground}"
    textColor: "{colors.ink}"
    typography: "{typography.counter}"
    size: "40px"
    height: "58px"
  counter-housing:
    backgroundColor: "{colors.ground}"
    rounded: "0"
  start-with-windows-off:
    backgroundColor: "{colors.ground}"
    textColor: "{colors.body}"
    rounded: "{rounded.punch}"
    padding: "0"
    width: "64px"
    height: "36px"
  start-with-windows-on:
    backgroundColor: "{colors.ground}"
    textColor: "{colors.ink}"
    rounded: "{rounded.punch}"
    width: "64px"
    height: "36px"
---

# Ternitor — design system

## Overview

Ternitor hides the blank console windows Windows opens on behalf of processes that
have no console of their own. It lives in the notification area and does nothing
visible; the settings window is the only surface it owns, and it is opened
perhaps once.

That constraint decides everything. A utility whose value is *not being noticed*
cannot have a surface that performs. So the surface is a **bench instrument
panel**: a mechanical counter, a punched control, and a grease pencil, on a sheet
of film base. It is short, legible in one look, and each element is a real
instrument rather than a decoration.

The figure is the app: **the counter measures work that leaves no trace**. It is
the only thing on the surface that carries a number, and the number is the whole
of what Ternitor produces — windows opened and taken away before they were drawn.
There is no history beside it because the app keeps none.

Five rules hold the surface together:

1. **Instruments, not controls.** Only two things answer the pointer: the punch
   and the window itself. Everything else is a reading.
2. **State is a mark, not a colour.** ON is a punched hole; OFF is an intact
   strip. The whole surface reads correctly in greyscale.
3. **Orange is grease pencil, and grease pencil is the operator's hand.** It
   appears in exactly one place, the tick beside an armed switch. Never as a
   fill, never as a state colour.
4. **Measuring, not performing.** The counter states a figure a person could read
   off a machine. Nothing counts up for effect; nothing animates.
5. **Nothing is stored.** The surface shows this session and only this session.
   Nothing on it implies a history, because the app keeps none.

There is no light theme. A bench is a dark room with a lit instrument, and this
surface is built around one black ground with ink lifted off it; inverting it
would be a different object rather than the same object in another theme. The
window caption is tinted to the ground so the OS frame reads as part of the plate.

## Colors

Three colours and no others: the ground, the ink, and the orange the operator
marks with. Everything between them is film grey.

- `ground` is film base. It is the entire page, and the page is pure black.
- `speck` and `dust` are grain: one deterministic scatter of single pixels across
  the ground. `dust` is the rare speck that catches light. Both are baked, never
  animated, so the sheet does not crawl between repaints.
- `edge` is one-pixel structure: the counter housing and its wheel dividers. It is
  deliberately *below* body contrast — structure should be felt before it is seen.
- `rule` is the section rules. It is dimmer than `edge`; a rule divides, it does
  not attract.
- `dim` carries small-caps labels and the footer. 6.1:1 on the ground.
- `body` carries running copy — the paragraph that says what the app does, and the
  small print under the control. 11.9:1 on the ground. A step above `dim` so that
  explanation never reads as annotation.
- `ink` is primary copy, values, the counter's digits, and the punched state of a
  control.
- `grease` is the one saturated value on the surface, and it is rationed. It means
  *the operator's hand was here*: the tick beside an armed switch, and nothing
  else.

## Typography

Four families, all of them Windows stock, chosen because they belong to the world
rather than because they are available.

- **title — Franklin Gothic Medium.** The American industrial grotesque that ended
  up on film can labels and bench signage. It appears once, on the app's name.
- **label — Arial Narrow Bold, uppercase, tracked 1.4px.** Film edge printing is
  set in exactly this: tiny condensed caps running along the edge of the strip.
  Every field label uses it, which is what gives the sheet its rhythm.
- **body — Franklin Gothic Book.** The title's family at reading size, for the one
  paragraph on the surface.
- **small — Arial Narrow regular.** The dense explanation under the control.
- **mono — Cascadia Mono.** Data: the hidden count, window titles, clock times,
  the version string. The terminal face of the audience this app serves.
- **counter — Cascadia Mono semibold at 34px.** The counter's wheels only.

Fonts render with greyscale antialiasing (`ANTIALIASED_QUALITY`), **never
ClearType**. On a black ground subpixel rendering fringes every glyph with orange
and blue, and on a surface whose single permitted colour is a grease mark that
reads as a bug, not as a compromise.

## Layout

A fixed 720×344 logical client at 96dpi, scaled as a whole by the monitor's DPI
factor — never reflowed. Every measurement is logical and snapped to whole device
pixels through one `px()` helper, which is what keeps a one-pixel rule exactly one
pixel wide instead of blurring across two.

Bands, top to bottom:

| Band | Logical | Contents |
|---|---|---|
| Head | 20–128 | Name and description left; counter right |
| Fields | 144–241 | Last cut, session clock left; the switch and its small print right |
| Footer | 284–314 | Version, target, log location |

The two columns are 44–474 and 452–676, and they overlap because the paragraph
and the counter never share a line. The paragraph's three lines are held to a
430px measure through the same `fit()` the window titles use, so if the display
face ever substitutes for a wider one the text ellipsizes rather than growing
into the counter's housing. Nothing is centred; the surface is read top-left to
bottom-right, like a workbench.

The head rule at 128 is the only full-width division above the footer, and it is
what separates the instrument from the reading: the name and the paragraph say
what the app is, and below the rule is everything that changes.

## Elevation & Depth

**There is none, and that is a rule.** No shadow, no gradient, no blur, no
translucency anywhere on the surface. Separation is carried entirely by one-pixel
lines at two weights: `edge` for structure, `rule` for division.

The only depth-like behaviour is the window's own: the OS caption is tinted to
`ground` with `ink` text so the frame reads as part of the plate, and the frame's
drop shadow is Windows' own, left alone.

## Shapes

Rectangles and one round. The counter housing, its wheels and the rules are all
rectangular with square corners: this is cut metal.

The only rounded form is the **punch control** (4px, with a 2px slug inside),
because it is the one shape that is literally *punched* rather than cut. That
distinction is the shape system: cut is square, punched is round.

The grease mark is the exception to all of it — two strokes, one 3px and one 1px
overshooting it at both ends, drawn to look waxy and by hand rather than as a
vector line.

## Components

**The counter.** A mechanical counter's wheels: a bordered housing divided into
40×58 cells, one digit each in `counter` type, zero-padded to at least three.
`000` is the honest reading for a session that has hidden nothing yet. This is
deliberately *not* a hero metric — wheels are an instrument, and an instrument
states a measurement rather than performing one.

**The punch.** The Start with Windows control, 64×36. OFF is an intact strip: a
`body`-coloured outline with a hollow slug. ON is a punched hole: an `ink` outline
with the slug filled solid, the material removed and light coming through — the
same weight as OFF's, so an off switch never reads as a disabled one. A grease
tick appears beside the state word when armed. The keyboard shows a square focus
ring at a 4px offset.

An armed switch is already at `ink` and so cannot answer the pointer with a
colour; the **slug** answers instead. Pressing takes a pixel of material off it
in either state, and hovering over an armed switch gives one back — 18px at rest,
20px hovered, 16px held. Feedback stays a mark. The control snaps between states;
there is no slide.

When the write behind it fails the punch simply stays where it was, and the small
print below it stops describing the registry and states the reason and the
state it is still in. There is one control on this surface; it does not get to
refuse silently.

**The paused indicator.** Pause lives on the tray menu, but it changes what the
counter means — a frozen figure with no explanation is the one lie this surface
could tell. While paused, the counter's label reads `PAUSED` in `body` followed
by `NOT COUNTING` in `dim`, in place of `HIDDEN THIS SESSION`. It is an indicator,
not a control: there is still nothing to click.

**The footer.** Version and target left; log location right. Static, `mono`,
`dim`.

## Do's and Don'ts

**Do**

- Drive every number on the surface from live state. The design's failure mode is
  a figure that keeps showing something the app no longer has.
- Put each fact in the instrument that fits it: counts in wheels, states as marks,
  time as clock readings.
- Keep orange to one mark. If two orange things are on screen at once, one of them
  is wrong.
- Snap every measurement through `px()`, including hairlines and type.
- Check that a new element still reads in greyscale before checking it in colour.

**Don't**

- Don't add a scrollbar, a hover tooltip, a list of what was hidden, or anything
  else that implies a browsable history. The app keeps no history.
- Don't use orange as a background, a fill, an error state, or a hover state.
- Don't add elevation, a card, a gradient, or a soft edge. Separation is a line.
- Don't render text with ClearType, and don't add a font that isn't stock Windows.
- Don't add a light theme without redrawing the object; inverting the palette
  produces a lit instrument on white paper, which is a different thing.
- Don't let the surface grow. If a setting is added, something else comes off.
