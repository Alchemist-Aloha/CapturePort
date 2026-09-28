---
name: CapturePort
description: "A native ingest instrument for photographers: a verified threshold between camera and library."
colors:
  canvas: "#fafbf9"
  panel: "#f0f2ef"
  card: "#ffffff"
  border: "#d2d8d2"
  muted: "#58625b"
  placeholder: "#e8ece7"
  selected: "#e1eae1"
  accent: "#276f51"
  danger: "#f5e3dc"
  text: "#242b26"
  header: "#f0f2ef"
  primary: "#247c66"
  primary-hover: "#195d4c"
  primary-text: "#ffffff"
typography:
  display:
    fontFamily: "Adwaita Sans, sans-serif"
    fontSize: "16px"
    fontWeight: 600
  headline:
    fontFamily: "Adwaita Sans, sans-serif"
    fontSize: "18px"
    fontWeight: 600
  title:
    fontFamily: "Adwaita Sans, sans-serif"
    fontSize: "14px"
    fontWeight: 600
  body:
    fontFamily: "Adwaita Sans, sans-serif"
    fontSize: "14px"
    fontWeight: 400
  label:
    fontFamily: "Adwaita Sans, sans-serif"
    fontSize: "12px"
    fontWeight: 400
rounded:
  sm: "4px"
  full: "9999px"
spacing:
  tight: "4px"
  control-gap: "8px"
  content: "16px"
  section: "24px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.primary-text}"
    rounded: "{rounded.sm}"
    padding: "4px 16px"
    height: "36px"
  button-secondary:
    backgroundColor: "{colors.card}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    padding: "4px 16px"
    height: "36px"
  filter-idle:
    backgroundColor: "transparent"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    padding: "4px 8px"
  filter-active:
    backgroundColor: "{colors.selected}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    padding: "4px 8px"
  input:
    backgroundColor: "{colors.card}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    padding: "4px 16px"
    height: "36px"
  media-card:
    backgroundColor: "{colors.canvas}"
    rounded: "{rounded.sm}"
  media-card-selected:
    backgroundColor: "{colors.selected}"
    rounded: "{rounded.sm}"
---

# Design System: CapturePort

## Overview

**Creative North Star: “The Verified Threshold”**

CapturePort is a quiet picture-desk instrument for reviewing media before import. The visual overhaul gives the photographs and their import decisions priority: neutral chrome, a restrained scheme accent, compact controls, and explicit state labels. The four named schemes remain recognizable, while light and dark modes change the full surface system rather than only the accent.

The bundled Adwaita Sans face gives the desktop UI a consistent Linux-native voice. Flat surfaces, 1px borders, and a contained image well keep the contact sheet calm during long review sessions. Core import behavior and safety language remain unchanged.

**Key Characteristics:**

- 48px neutral header, 232px source rail (184px below 760px), and 52px footer.
- 4px control radius and 36px minimum button, navigation, and filter height.
- Adwaita Sans from the bundled OFL-licensed font asset.
- Quiet filter selection, explicit “Selected” / “Part selected” labels, and status text with icon.
- Full palette parity across Pine, Darkroom, Graphite, and Ink in light and dark modes.

## Colors

The active Pine light tokens above are the frontmatter source. The complete source palette matrix is recorded in `.impeccable/design.json`; each scheme owns all 19 roles in both modes.

### Primary

- **Pine accent** (#276f51): focus borders, selected media borders, and affirmative emphasis.
- **Pine primary** (#247c66): the filled action surface; hover deepens to #195d4c.

### Neutral

- **Canvas** (#fafbf9): application background.
- **Panel** (#f0f2ef): source rail and neutral chrome.
- **Card** (#ffffff): media and input surfaces.
- **Border** (#d2d8d2): 1px structural separation.
- **Muted** (#58625b): metadata, counts, and helper text.
- **Placeholder** (#e8ece7): unloaded image wells.
- **Selected** (#e1eae1): quiet selection and hover tone.
- **Text** (#242b26): primary ink.
- **Danger** (#f5e3dc): destructive-action surface only.

### Named Rules

**The One Signal Rule.** Each scheme has one accent family. Use it for focus and selection; use the scheme’s primary token for the affirmative button.

**The Scheme Parity Rule.** Every scheme defines all 19 palette roles in both modes. Header, borders, surfaces, button treatments, and muted text must travel together.

## Typography

**Display / Body / Label Font:** Adwaita Sans, with a generic sans-serif fallback. The regular face is bundled at `crates/captureport/assets/fonts/AdwaitaSans-Regular.ttf` with its OFL text alongside it.

**Character:** Compact, legible, and native to the Linux desktop. Weight carries hierarchy while the 12px/14px rhythm keeps dense media review readable.

### Hierarchy

- **Display** (600, 16px): CapturePort wordmark.
- **Headline** (600, 18px): panel and empty-state titles.
- **Title** (600, 14px): filenames, navigation labels, and leading row text.
- **Body** (400, 14px): controls, summaries, and explanatory copy.
- **Label** (400, 12px): time, size, status, counts, and image overlays.

## Layout

The desktop shell is vertical: a 48px header, a flexible body, and a 52px footer. The body places a fixed source rail beside a fluid content area. The rail is 232px wide at normal windows and 184px below the single 760px window-width breakpoint.

The browser uses a reflowing contact sheet with 16px grid gaps and 16px content gutters. Thumbnail targets remain the existing stepped sizes; each image well uses `ObjectFit::Contain` so photographs are never cropped. The layout reserves 208px for browser control chrome and 104px for media metadata; the resulting media row chrome is 122px (104px details + 16px grid gap + 2px borders). A grouped gallery heading contributes 44px (36px control height + 8px control gap).

The footer keeps selection totals and “Preview import” together. It remains present while status messages change, so the review action has a stable location.

Every page uses 16px header/footer gutters. Settings fields use 16px group separation, with an 8px section margin plus the parent 16px rhythm for a 24px section step; headings end 4px before their content. Hidden sidebar actions are removed from layout so unavailable actions leave no phantom gap. Scrollbars reserve a 16px gutter with a 4px inset.

## Elevation & Depth

CapturePort is flat by default. Depth comes from tonal surface changes and 1px borders; there are no shadows, blur layers, or decorative motion. Image badges use a solid black scrim for legibility over photographs.

**The Border-Before-Background Rule.** Use a border or a quiet tonal shift to separate states before adding visual weight.

## Shapes

The system uses a 4px radius for buttons, filters, navigation rows, fields, cards, and media tiles. Image overlays use the same compact radius. Borders are 1px and palette-derived. Buttons, navigation rows, and filters use a 36px minimum height with 16px horizontal and 4px vertical padding. Inputs use a 34px inner height plus 2px of borders for a 36px outer control.

## Components

### Buttons

- **Primary:** filled scheme primary, contrasting text, 36px minimum height, 4px radius, and 16px horizontal/4px vertical padding.
- **Secondary:** outlined card/panel surface, 1px scheme border, 36px minimum height, 4px radius, and 16px horizontal/4px vertical padding.
- **Hover / Focus:** hover changes the relevant scheme surface; focus uses the accent border. No shadow or glow.

### Filters and Navigation

- Source navigation uses 36px rows with 16px horizontal/4px vertical padding. Compact filter chips use 36px rows with 8px horizontal/4px vertical padding and 4px radius.
- Active filters are quiet: selected tone and text weight communicate state without a saturated filled chip.
- Navigation remains text-first and the active row uses the selected surface.

### Media Tile

- The tile is a flat, bordered 4px card with a contained image well and a placeholder while thumbnails load.
- Type badge, file size, and explicit selection label sit over or beside the image without cropping it.
- Metadata is separate: filename, time plus size, import status, then “View files” / “Hide files” for bundles.
- The filename wrapper is explicitly full width before ellipsis truncation, so long source names remain stable in the grid.
- Import status always uses icon plus text, including prior-session detail when available.

### Footer and Import Disclosure

The 52px footer presents item totals, selected totals, byte totals, status text, and the existing Preview import action. Bundle disclosure remains a separate row interaction from import status, so “Selected”, “Part selected”, status, and “View files” communicate distinct facts.

## Do's and Don'ts

### Do:

- **Do** preserve all four schemes and both modes as complete source palettes.
- **Do** keep the 48px header, 232/184px rail, 52px footer, and 4px/36px control language coherent.
- **Do** use Adwaita Sans for every surface and keep the OFL asset with its license.
- **Do** keep thumbnails contained and expose file size, import status, and bundle disclosure as separate metadata.
- **Do** label selection explicitly and keep color from carrying status by itself.

### Don't:

- **Don't** reintroduce a broad colored masthead, saturated filter pills, repeated filled secondary buttons, shadows, or decorative gradients.
- **Don't** crop photographs to fit a tile.
- **Don't** merge bundle disclosure into the import-status label.
- **Don't** add a new scheme or mode with inherited or partial palette roles.
- **Don't** change the importer’s deterministic plan, verification, collision, recovery, or source-safety behavior as part of visual work.
