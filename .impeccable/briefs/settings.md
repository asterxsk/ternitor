# Surface brief — Ternitor settings

Mode: **Operate**. The visitor completes a task: confirm the janitor is running,
read what it has done, and decide whether it starts with Windows.

## Direction contract

**THESIS.** A bench instrument panel: a mechanical counter, a punched switch, a
grease pencil, on a sheet of film base. What it refuses is the category default —
a Fluent settings page of rounded cards and one toggle row per setting, which is
also what the product's own host OS would ship.

**OWN-WORLD.** Black base, white ink, one rationed orange, film grey between them.
A bordered counter housing of 40×58 wheels; a 64×36 punched switch; a hairline
rule dividing what the app *is* from what it has *done*. Strip it of all content
and it is still unmistakably a bench instrument, not a preferences dialog.

**STORY.** The person understands that this counts blank console windows and has
been counting them; believes it is running right now because the counter reads
live; and either arms Start with Windows or closes the window, in one look,
without reading a paragraph.

**FIRST VIEWPORT.** 720×344 fixed. Name and one paragraph upper-left. Counter
wheels upper-right, 46–104, under a right-aligned label. Rule at 128. Two field
rows mid-left: last cut, session clock. The switch and its small print mid-right.
Footer rule and a single data line below. The primary action — the punch — sits in
the right column at 452,164, unmissable because it is the only rounded shape on
the surface.

**AMENDMENT (2026-09-16).** The rail was cut from the surface at the owner's
direction. The 84px band, its sprocket holes, the 13 frame cells, the cut mark and
the cut animation are all gone, and with them `colors.band`, the frame and
sprocket components, and the reduced-motion path — the surface no longer animates
at all. The instrument framing above is what survives; DESIGN.md is the current
record.

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

- Start with Windows (HKCU Run, one value), the live hidden count, and app info.
  Nothing else is on the screen; no log toggle, no pause toggle.
- Every number is live app state. The surface can never show a figure the app
  does not have.
- Portable single exe, no installer.
