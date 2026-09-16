# Surface brief — Ternitor settings

Mode: **Operate**. The visitor completes a task: confirm the janitor is running,
read what it has done, and decide whether it starts with Windows.

## Direction contract

**THESIS.** A bench instrument panel: a mechanical counter, a switch, a grease
pencil, on a sheet of film base. What it refuses is the category default —
a Fluent settings page of rounded cards and one toggle row per setting, which is
also what the product's own host OS would ship.

**OWN-WORLD.** Black base, white ink, one rationed orange, film grey between them.
A bordered counter housing of 40×58 wheels; a 64×36 switch; a hairline
rule dividing what the app *is* from what it has *done*. Strip it of all content
and it is still unmistakably a bench instrument, not a preferences dialog.

**STORY.** The person understands that this counts blank console windows and has
been counting them; believes it is running right now because the counter reads
live; and either arms Start with Windows or closes the window, in one look,
without reading a paragraph.

**FIRST VIEWPORT.** 720×344 fixed. Name and one paragraph upper-left. Counter
wheels upper-right, 46–104, under a right-aligned label. Rule at 128. Two field
rows mid-left: last cut, session clock. The switch and its small print mid-right.
Footer rule, a data line, and the quit button in its right corner. The primary
action — the switch — sits in the right column at 452,164, unmissable because it
is the only shape on the surface that is round in both axes.

**AMENDMENT (2026-09-16, second).** At the owner's direction the Start with
Windows punch became a **switch**: one track, one knob, the knob concentric with
the end cap it sits in, `ON` a filled track with the knob knocked out of it. The
punch's slug-and-hole metaphor was the only thing on the surface that had to be
*learned* to be read, and it stopped the control from saying "toggle" in one look.
A **Quit** button was added to the footer's right corner — 88×28, 4px radius, the
same exit as the tray menu's, for when the tray icon is buried. Reachable by
`Tab`, fired by `Enter` or `Space`, no confirmation on either path. The surface is
now four things rather than three. `src/mark.rs` also replaced the app mark: a
Node hexagon with a red X on a dark tile, the same definition baking the exe's
icon, the tray icon and `assets/icon.svg`. DESIGN.md is the current record.

**AMENDMENT (2026-09-16).** The rail was cut from the surface at the owner's
direction. The 84px band, its sprocket holes, the 13 frame cells, the cut mark and
the cut animation are all gone, and with them `colors.band`, the frame and
sprocket components, and the reduced-motion path — the surface no longer animates
at all. The instrument framing above is what survives; DESIGN.md is the current
record.

**AMENDMENT (2026-09-16).** The dark tile under the app mark was cut at the
owner's direction, so the paragraph above describes a plate that is no longer
there. The mark is now the hexagon and the badge alone: the X is separated from
the hexagon by a ring knocked straight out of it, which is transparent rather
than painted, because with no tile behind the mark there is nothing to paint a
gap with. The constants were recomputed for the mark's own bounding box so it
still fills its icon and sits centred, and the caption of the settings window
now wears it too (`ui.rs` registers the class with `WNDCLASSEXW { hIcon, hIconSm }`
where `WNDCLASSW` had left both null). Verified on the live window: the class
carries a 16px and a 32px icon, both reading green and red with transparent
corners, and a `PrintWindow` of the caption draws the hexagon and the X.
`assets/icon.svg` regenerated from the same constants.

**FORM.** Chosen: the user's pick from the served decision page, **Cutting Bench
Rail** (`challenger-cutting-bench`), a competitive challenger against the run's
assigned *Drafting Plate*, over the model pick *Windows 11 Settings Page*. The
run's risk note — *a rail implies a browsable history the app deliberately does
not keep* — is discharged structurally: the app stores no history at all, frames
are counted rather than listed, and nothing on the rail is clickable, hoverable or
scrollable. Build path: code-led, no comp.

**FINISH.** unreviewed and undocumented is unfinished; this build ends with the
finish review, the verdict, DESIGN.md, and every shipping raster carrying its
provenance

## Product truth this surface answers to

- Start with Windows (HKCU Run, one value), the live hidden count, app info, and
  Quit. Nothing else is on the screen; no log toggle, no pause toggle.
- Every number is live app state. The surface can never show a figure the app
  does not have.
- One exe, no runtime dependency; portable by copying the folder, and also
  installable per-user by `install.ps1` into `%LOCALAPPDATA%\Programs\Ternitor`,
  which is what puts it in Settings > Apps.
