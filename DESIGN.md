---
name: CapturePort
description: "A native ingest instrument for photographers: a verified threshold between camera and library."
colors:
  warm-paper: "#f7f7f2"
  pale-sage: "#eff2e9"
  bright-paper: "#fffef9"
  soft-fern: "#d3dfd2"
  faint-sage: "#e5ede3"
  mint-wash: "#dcefdc"
  moss-grey: "#4d6355"
  deep-ink: "#1d2f24"
  pine-signal: "#276f51"
  pale-clay: "#f5e3dc"
  cedar: "#193c34"
  paper-white: "#ffffff"
  pale-mint: "#b9dfc9"
  pine-button: "#247c66"
  pine-button-deep: "#195d4c"
  ghost-fern: "#50836b22"
  ghost-fern-hover: "#50836b44"
  ghost-fern-line: "#57856d66"
typography:
  display:
    fontSize: "1.25rem"
    fontWeight: 700
  headline:
    fontSize: "1.125rem"
    fontWeight: 600
  title:
    fontSize: "0.875rem"
    fontWeight: 600
  body:
    fontSize: "0.875rem"
    fontWeight: 400
  label:
    fontSize: "0.75rem"
    fontWeight: 400
rounded:
  sm: "4px"
  md: "6px"
  full: "9999px"
spacing:
  "1": "4px"
  "2": "8px"
  "3": "12px"
  "4": "16px"
  "5": "20px"
  "6": "24px"
components:
  button-primary:
    backgroundColor: "{colors.pine-button}"
    textColor: "{colors.paper-white}"
    rounded: "{rounded.md}"
    padding: "8px 16px"
    height: "38px"
  button-primary-hover:
    backgroundColor: "{colors.pine-button-deep}"
  button-ghost:
    backgroundColor: "{colors.ghost-fern}"
    rounded: "{rounded.md}"
    padding: "8px 12px"
    height: "36px"
  button-ghost-hover:
    backgroundColor: "{colors.ghost-fern-hover}"
  chip-active:
    backgroundColor: "{colors.pine-button}"
    textColor: "{colors.paper-white}"
    rounded: "{rounded.md}"
    padding: "4px 12px"
  chip-idle:
    backgroundColor: "{colors.ghost-fern}"
    rounded: "{rounded.md}"
    padding: "4px 12px"
  input:
    backgroundColor: "{colors.bright-paper}"
    textColor: "{colors.deep-ink}"
    rounded: "{rounded.md}"
  nav-item-active:
    backgroundColor: "{colors.mint-wash}"
    textColor: "{colors.deep-ink}"
    rounded: "{rounded.md}"
    padding: "8px 12px"
    height: "36px"
  media-card:
    backgroundColor: "{colors.bright-paper}"
    rounded: "{rounded.md}"
  media-card-selected:
    backgroundColor: "{colors.mint-wash}"
---

# Design System: CapturePort

## Overview

**Creative North Star: "The Verified Threshold"**

CapturePort's interface is a controlled threshold between a camera card and a
lasting library. Nothing crosses it unexamined, and nothing crosses it silently.
The visual system expresses that posture rather than decorating it: every screen
is arranged so the user can see the exact destination of every file before a
single byte moves, and so that after the copy the state of each file is legible
at a glance. The design is a gate, and a gate is judged by what it refuses.

The system is green-washed rather than neutral. On warm paper in light mode and
on near-black pine in dark mode, the entire interface is built from one family of
surface tones plus a single mint-to-pine signal color. Structure is carried by
1px borders and five tonal steps, never by shadow, blur, or translucency. The
result reads as an instrument: compact, quiet, and specific, with no chrome that
is not information. Density is deliberate, because the user is scanning hundreds
of frames, and the layout stays tight so that more of the decision is visible at
once.

Two looks are explicitly ruled out. This is not a neon-cyan developer-tool
dashboard, and it is not a typical SaaS page: no hero sections, no gradient
marketing panels, no oversized pastel cards, no glow, no drop shadows. Those
references fail the brief for the same reason, which is that they spend
attention on themselves instead of on the import decision.

**Key Characteristics:**
- Every surface is flat and separated by a 1px border; the interface contains no shadows, blurs, or backdrop filters at all.
- One signal color per scheme, reserved for focus, selection, and the primary action.
- Hierarchy comes from a five-step tonal ramp (canvas → panel → card → selected), not from elevation.
- Four schemes times two modes: the scheme repaints every surface, including the header and the secondary button tints.
- Compact, consistent controls: 6px radius, 36–38px heights, 20px gutters, four type steps.
- Status is read as text plus icon; color never carries meaning on its own.

## Colors

One dark-green system with a single mint-to-pine signal, sitting on warm paper in
light mode and near-black pine in dark mode; the accent is the only saturated
color anywhere in the interface.

The frontmatter records the **Pine** scheme in light mode, which is the default
configuration. Pine, Darkroom, Graphite, and Ink each define all nineteen palette
roles for both light and dark, and the complete set lives in
`.impeccable/design.json`. Every value below is Pine light; Pine dark and the
other three schemes redefine the same roles in their own hue family.

### Primary
- **Pine Signal** (#276f51): the single affirmative color. It marks focus (an input's border swaps to it), selection (the border of a selected media tile), and the primary action. In dark mode this role becomes a light mint (#a5e0ae) because the surface polarity inverts, and the button ink inverts with it (#0e1813).

### Neutral
- **Warm Paper** (#f7f7f2): the application canvas, behind everything.
- **Pale Sage** (#eff2e9): the sidebar, toolbar strips, and the expanded group panel; one step off the canvas.
- **Bright Paper** (#fffef9): cards, inputs, and the status bar; the highest surface.
- **Soft Fern** (#d3dfd2): every divider and border in the interface. Borders do the work that shadows would do elsewhere.
- **Faint Sage** (#e5ede3): the placeholder behind a thumbnail that has not loaded, and the accent for image cards' empty state.
- **Mint Wash** (#dcefdc): the selection and hover tone; a tile or nav item in this color is selected.
- **Moss Grey** (#4d6355): all secondary and metadata text; timestamps, counts, labels, helper lines.
- **Deep Ink** (#1d2f24): primary text.
- **Cedar** (#193c34): the application header band, with **Paper White** (#ffffff) for the wordmark and **Pale Mint** (#b9dfc9) for the subtitle.
- **Pale Clay** (#f5e3dc): the destructive-action fill for "Delete verified originals…" and "Clean incomplete file", always carrying normal Deep Ink text rather than a red label.
- **Ghost Fern** (`#50836b22`, a 13% tint of the signal hue) with **Ghost Fern Line** (`#57856d66`) and **Ghost Fern Hover** (`#50836b44`, 27%): the translucent fill and hairline of secondary buttons and idle chips. In dark mode the alpha steps up rather than the hue changing.

### The Four Schemes
| Scheme | Accent (light / dark) | Header (light) | Character |
|---|---|---|---|
| **Pine** | `#276f51` / `#a5e0ae` | `#193c34` | The incumbent. Green signal on warm paper and near-black pine. |
| **Darkroom** | `#a4653a` / `#e8b06a` | `#4a3320` | Warm charcoal and an amber safelight; the photography-native alternative. |
| **Graphite** | `#2f5f8f` / `#8fb4d9` | `#27313d` | Neutral cool grey with one restrained steel-blue signal and no color cast. |
| **Ink** | `#2b48b8` / `#86a6ff` | `#1c2a4a` | Paper-forward light neutral with a cobalt signal and a deep navy header. |

### Named Rules
**The One Signal Rule.** One accent per scheme, and it means exactly three things: focus, selection, the primary action. It is the only saturated color on screen; everything else is a surface tone or a text tone.

**The Scheme Parity Rule.** A scheme is not an accent swap. Header, primary button, secondary button tints, borders, and every surface must be defined for that scheme in both modes; no scheme may inherit another's chrome. A new scheme with an incomplete palette is a bug, not a partial feature.

**The Status Language Rule.** Import status is icon plus text, and color only reinforces it. `✓ Imported`, `? Unknown`, `New`, `Checking…`. A reader who cannot distinguish the greens must still be able to distinguish the states.

## Typography

**Display Font:** The platform UI font (GPUI's default; the application configures no family anywhere, so the system resolves it, typically the desktop's default sans).
**Body Font:** The same family.
**Label/Mono Font:** None. There is no monospaced or secondary face in the system.

**Character:** One family, three weights, and four sizes. Nothing is expressed by changing the typeface, because there is only one; hierarchy is entirely weight and step. Semibold is the workhorse — filenames, buttons, headings, active navigation — and Bold appears exactly once, in the wordmark. Regular weight is reserved for secondary text and body copy.

### Hierarchy
- **Display** (700, 1.25rem/20px): the "CapturePort" wordmark in the header, and nothing else.
- **Headline** (600, 1.125rem/18px): panel titles ("Sources", "Settings", "Import recovery", "Review import · N files") and empty-state headlines.
- **Title** (600, 0.875rem/14px): media filenames, primary and navigation labels, and summary lines that lead a row.
- **Body** (400, 0.875rem/14px): the default interface size; footer text, settings values, and sentence-length copy.
- **Label** (400, 0.75rem/12px): metadata, timestamps, counts, field captions, and tile overlays. Frequently paired with Moss Grey.

GPUI's 1rem base (16px) is never set explicitly; no interface text is larger than the 20px wordmark or smaller than 12px. Line height is the text-style default and is not overridden anywhere, so vertical rhythm inside a panel comes from spacing rather than leading. The system never truncates by shrinking type: long filenames are truncated with an ellipsis at Title size.

### Named Rules
**The Two-Size Rule.** Interface text is 12px or 14px. 18px and 20px belong to headings and the wordmark only; a 16px body treatment does not exist anywhere in the application, so a screen that needs to feel louder should change weight or spacing, not step size.

**The Semibold-Is-Structure Rule.** Semibold marks what the row is about; Regular carries what happened. A row may have exactly one semibold element.

## Layout

The application opens at 1180×760. A 70px header spans the full width, above a body split into a fixed sidebar and a fluid content area, above a 42px status bar. The window is the layout's only dimension of change.

The sidebar is a fixed 238px, dropping to 200px when the window is narrower than 760px; that single breakpoint is the entire responsive story for navigation. Content panels use 20px horizontal gutters and 8–12px vertical rhythm, and each panel opens with a bordered header row. The media browser is a virtualized grid whose column count is computed from the available width against a target tile size, with a fixed 12px gutter: the available width minus the sidebar and a 40px content inset, divided by the target plus the gutter. Tile targets step through 172, 210, 250, 300, and 360px, and a five-step slider sets the target while the actual height adapts to the window. Each tile reserves 82px below its image for filename, timestamp, and status; a gallery heading reserves 42px.

Interactive controls are tightly standardized: 38px minimum height for the primary button and 36px for secondary buttons, nav items, and any clickable row; 28px for slider steps and small inline targets. The status bar carries a single truncating line on the left and counts plus the primary action on the right, and stays 42px tall. The expanded group panel caps at 240px and scrolls; a single row in the review list is at least 72px tall, because it names a destination and its status.

### Named Rules
**The Window-Not-Device Rule.** There is one breakpoint, it is measured in window width, and it exists to stop the sidebar from crowding the grid. Do not add device-class breakpoints; the grid already reflows continuously between them, and a desktop import tool has no phone layout to honour.

**The Chrome-Is-Fixed Rule.** Header 70px, status bar 42px, sidebar 238/200px. Chrome does not grow, shrink, or reflow; all flexibility belongs to the content area, so the interface feels stable while the library scrolls past it.

## Elevation & Depth

This system has no shadows. Not on hover, not on menus, not on the overlay badges that sit on top of photographs, and not on the expanded group panel that overlaps the grid. There is also no opacity layer, no blur, and no backdrop filter anywhere in the interface. Depth is conveyed in exactly two ways: a 1px border for separation, and a five-step tonal ramp for hierarchy — canvas (#f7f7f2) → panel (#eff2e9) → card (#fffef9), with **Mint Wash** (#dcefdc) marking the selected state and **Cedar** (#193c34) forming the header band above everything.

Elements that would use a shadow elsewhere simply sit on a photograph with a solid translucent scrim instead: tile badges and size labels are `rgba(0, 0, 0, 0.6)` at 4px radius with white 12px text, which guarantees legibility over an arbitrary image without introducing a shadow. The one filled non-accent surface in the system is the destructive button's **Pale Clay** (#f5e3dc), which is a state signal rather than an elevation.

Because surface polarity inverts between modes, the tonal ramp is defined twice per scheme rather than derived by tinting: dark mode's ramp runs near-black to charcoal (#0e1813 → #15251d → #213329) and its borders lighten instead of darkening. Any comparison of two surfaces must therefore be re-checked in both modes; a ramp that reads correctly in light mode can collapse in dark.

### Named Rules
**The Flat-By-Default Rule.** Surfaces are flat at rest and flat in every state. Hover and focus change tone or border color, never elevation. If a design problem seems to need a shadow, it needs a border or a tonal step instead.

**The Border-Before-Background Rule.** Prefer a 1px border to a background change when separating two surfaces. The interface already uses twenty-one such borders, and they are the reason it survives having no elevation at all.

## Shapes

The form language is a single soft rectangle. A 6px radius is the near-universal form: buttons, chips, inputs, cards, tiles, nav items, panels, and the destructive action all use it. A 4px radius is reserved for the small overlay furniture that sits on photographs — the media-type badge and the size label — and for the inline hover surface behind a filename or gallery name. Full-round corners appear only on slider tracks, which are 4px tall at rest and 6px when active, drawn with `rounded_full` regardless.

Borders are always exactly 1px and always a single color from the palette. There are no double borders, no inset rings, no dashed strokes, and no colored side-tabs other than the 1px left border that indents a bundle member beneath its group. Images are clipped to the tile's 6px radius and fill their tile edge-to-edge with `Cover`, so photographs read as full-bleed rectangles rather than as framed pictures.

### Named Rules
**The One-Radius Rule.** 6px for anything interactive, 4px for overlay chips on imagery, full-round for slider tracks only. A new component picks one of those three; it does not introduce a fourth radius.

## Components

### Buttons
- **Shape:** 6px radius (rounded_md), 1px border on the secondary variant only.
- **Primary:** Pine fill with white text (#247c66 on #ffffff), 16px horizontal and 8px vertical padding, 38px minimum height, 14px semibold. Its hover deepens the fill to #195d4c. The primary button color is a distinct token from the accent, because the accent's job is borders on paper while the button must carry white text.
- **Secondary / Ghost:** A translucent green tint with a hairline of the same hue (`#50836b22` fill, `#57856d66` border), 12px/8px padding, 36px minimum height, 14px. Hover raises only the fill alpha to 27% (`#50836b44`); the border is static. This is the default button for everything that is not the one primary action — this interface has many secondary actions and exactly one primary per screen.
- **Toggle state:** the mode switch in the header is itself a ghost button with a text label ("Dark mode" / "Light mode"), so there is no switch widget in the system.
- **Destructive confirmation:** an irreversible action is never its own confirmation. A quiet secondary button opens a bordered confirmation block that states the scope in words ("Delete 47 original(s) (2.1 GB) from Card · X"), names the consequence ("This cannot be undone"), and offers a non-destructive exit — "Keep originals", "Keep file" — beside a single danger-filled confirm. Only two actions in the system are irreversible (deleting verified originals, cleaning an incomplete file), and both use this shape; the scope stated and the scope executed are the same predicate.

### Chips
- **Style:** 6px radius, 12px horizontal and 4px vertical padding, 14px text, no border, on the translucent ghost fill when idle.
- **State:** Active chips are filled with the scheme's primary color and carry white text, so a selected filter is visible at a glance; idle chips are the ghost tint. Hover promotes an idle chip along the same alpha ramp. Chips carry filters (All, Photos, Videos, New, Imported, Possible), sorts (Time, Name), preset choice (Everyday, Organized), and the scheme and mode selectors.

### Cards / Containers
- **Corner Style:** 6px radius on media tiles, inputs, and buttons; panels and the sidebar are square-edged and separated by borders.
- **Background:** Tiles and inputs sit on Bright Paper, the sidebar and toolbars on Pale Sage, the canvas on Warm Paper.
- **Shadow Strategy:** None. See Elevation & Depth.
- **Border:** 1px Soft Fern on every tile; a selected or partly-selected tile swaps that border to the accent as well as changing its background to Mint Wash, so selection survives being looked at in grayscale.
- **Internal Padding:** 8px horizontal and 4px vertical around a tile's metadata block; 20px horizontal and 12px vertical inside panels and panel headers.

### Inputs / Fields
- **Style:** 1px Soft Fern border with a 6px radius and Bright Paper fill; the field's caption is 12px Moss Grey one spacing step above it.
- **Focus:** The border color swaps to the accent. There is no glow, ring, or outer shadow; focus is a single 1px color change, and it is the only focus affordance in the system.
- **Disabled / Error:** Fields are not disabled. Invalid input is reported as a message line in the status bar rather than as a field-level error state, and typed input inherits the surrounding color, matching the rest of the palette.

### Navigation
- **Style:** The sidebar is a stack of bordered rows on Pale Sage with a 1px right border separating it from the content. Each item is 6px radius, 36px minimum height, 12px/8px padding, 14px text, and 8px gaps between items.
- **States:** The active item is filled with Mint Wash and set semibold; inactive items are Pale Sage with regular weight and promote to Mint Wash on hover. Navigation is text-only, with no icons, so the labels carry the entire navigation load.
- **Mobile treatment:** None. The sidebar narrows from 238px to 200px below a 760px window width and never collapses or becomes a drawer.

### Media Tile (signature)
The system's defining component, and the one that must survive being looked at ten thousand times. A 6px-radius bordered rectangle containing a full-bleed `Cover` thumbnail, with the media-type badge (`▶ VIDEO`, `▣ RAW`, `▣ JPEG`) in a `rgba(0, 0, 0, 0.6)` pill at the top-right and the file size in a matching pill at the bottom-left, both 4px radius with white 12px text. Below the image: the filename at 14px semibold truncated with an ellipsis, a 12px Moss Grey timestamp, and a 12px status line. A group member may instead show "View files" / "Hide files" in that status slot, which is how an expandable RAW+JPEG or video+sidecar pair advertises itself. A placeholder of Faint Sage holds the space until the thumbnail arrives.

### Status Readout (signature)
Import status is a 12px semibold line in the tile metadata block, formatted as icon plus text: `✓ Imported`, `! Possible duplicate`, `? Unknown`, `New`, `Checking…`. The two states that imply prior history — `✓ Imported` and `! Possible duplicate` — append the prior import they matched (`· session 12 · 2026-09-27`), so an uncertain duplicate is never left unexplained; the same line appears in full on an expanded bundle member row. A bundle that is partly selected darkens its status line to primary text rather than changing its wording.

### Tonal Slider (signature)
Both the thumbnail-size and session-gap controls are five-to-eight step ramps rather than drag handles: each step is a 24–30px wide, 28px tall target whose bar is 4px tall when inactive and 6px when active, drawn full-round in either the accent or the border color. The ramp accumulates — every step up to the current value is lit — so the control reads as a level, and the numeric value is printed at 12px beside it. Dragging across the steps scrubs the value.

### Status Bar (signature)
A 42px band on Bright Paper with a 1px top border, 20px gutters, and a truncating 14px Moss Grey message on the left. On the right sits the counts line ("N items · M selected · S") and, when a selection exists, the primary action. Because the bar is the only place errors appear, it always reserves the space whether or not a message is present.

## Do's and Don'ts

### Do:
- **Do** keep every surface flat and separate it with a 1px border; the system has twenty-one of them and zero shadows.
- **Do** define all nineteen palette roles — surfaces, borders, text tones, header, primary button, and ghost tints — for any new scheme, in both light and dark.
- **Do** use exactly one accent per scheme (#276f51 in Pine light) and spend it only on focus, selection, and the primary action.
- **Do** carry status as icon plus text (`✓ Imported`, `? Unknown`), so color is never the only carrier of meaning.
- **Do** use 6px radius and 36–38px minimum heights for anything interactive, and 4px radius for overlay pills on photographs.
- **Do** route new interface colors through the palette; the header, primary button, and ghost tints were once hardcoded greens and had to be tokenized before a second scheme could exist.
- **Do** check every surface pair in both modes; dark mode inverts polarity, so a border that reads as "lighter" in light mode reads as "darker" in dark.

### Don't:
- **Don't** add shadows, blurs, glow, or backdrop filters to any component, including menus, overlays, and hover states.
- **Don't** adopt a neon-cyan developer-dashboard palette or a typical SaaS presentation (hero bands, gradient panels, oversized pastel cards). Both are confirmed anti-references.
- **Don't** introduce a second accent color or use the accent for decoration; its scarcity is what makes selection readable.
- **Don't** use the destructive fill (#f5e3dc) as a text color or as a general warning tint; it is a two-action fill that always carries normal body ink.
- **Don't** block a new scheme on a shared neutral ramp. Graphite and Ink differ from Pine in their neutrals as well as their signal, and a scheme that only swaps the accent reads as a filter, not a scheme.
- **Don't** communicate any status with color alone, and don't let an uncertain state read as a confirmed one.
- **Don't** make a destructive control its own confirmation, and don't let the scope stated in a confirmation disagree with what the action will actually do.
