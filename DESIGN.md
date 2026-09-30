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
    fontFamily: "Spectral, serif"
    fontSize: "22px"
    fontWeight: 700
  headline:
    fontFamily: "Spectral, serif"
    fontSize: "18px"
    fontWeight: 600
  section:
    fontFamily: "Spectral, serif"
    fontSize: "16px"
    fontWeight: 600
  title:
    fontFamily: "Outfit, sans-serif"
    fontSize: "14px"
    fontWeight: 600
  body:
    fontFamily: "Outfit, sans-serif"
    fontSize: "14px"
    fontWeight: 400
  label:
    fontFamily: "Outfit, sans-serif"
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

The bundled Spectral serif carries the wordmark and every page, panel, section, and empty-state heading; Outfit handles the dense interface — controls, navigation, filenames, metadata, and labels. Flat surfaces, 1px borders, and a contained image well keep the contact sheet calm during long review sessions. Core import behavior and safety language remain unchanged.

**Key Characteristics:**

- 48px neutral header carrying the Spectral wordmark, 232px source rail (184px below 760px), and 52px footer.
- 4px control radius and 36px minimum button, navigation, and filter height.
- Spectral for the display voice and Outfit for the interface, both bundled with their OFL licenses.
- Quiet filter selection, explicit “Selected” / “Part selected” labels, and status text with icon.
- Material Symbols Outlined action icons, tinted from their control's text color.
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

**Display / Headline / Section Font:** Spectral, with a generic serif fallback. The SemiBold and Bold faces are bundled at `crates/captureport/assets/fonts/Spectral-SemiBold.ttf` and `crates/captureport/assets/fonts/Spectral-Bold.ttf`, with `Spectral-OFL.txt` alongside.

**Interface Font:** Outfit, with a generic sans-serif fallback. The Regular and SemiBold faces are bundled at `crates/captureport/assets/fonts/Outfit-Regular.ttf` and `crates/captureport/assets/fonts/Outfit-SemiBold.ttf`, with `Outfit-OFL.txt` alongside them.

**Character:** An editorial serif voice over a compact, native interface. Spectral marks where the reader is — the wordmark, the page, the section — while Outfit keeps dense media review legible at 12px and 14px. The split is by job, never by surface: Spectral never sets interface chrome, data, or type below 16px, and Outfit never sets a page heading.

### Hierarchy

- **Display** (Bold 700, 22px, Spectral): the CapturePort wordmark.
- **Headline** (SemiBold 600, 18px, Spectral): page, panel, and empty-state titles.
- **Section** (SemiBold 600, 16px, Spectral): settings sections and gallery session headings.
- **Title** (SemiBold 600, 14px, Outfit): filenames, navigation labels, and leading row text.
- **Body** (400, 14px, Outfit): controls, summaries, and explanatory copy.
- **Label** (400, 12px, Outfit): time, size, status, counts, and image overlays.

## Iconography

**Set:** Material Symbols Outlined (Material 3), vendored unmodified at
`crates/captureport/assets/icons/` with its Apache-2.0 license and upstream
revision recorded in that directory's `README.md`. GPUI paints each SVG as a
mask tinted by the control's text color, so one file serves all four schemes in
both modes and no icon carries a hard-coded color.

**Grid and weight:** every glyph keeps Material's 960-unit grid and its single
filled path, so the family shares one optical weight and corner language by
construction rather than by hand-matching.

**Size:** 16px beside 14px control text; 12px inside a 12px tile badge. The step
holds Material's own 24px-glyph-to-16px-text ratio, so an icon never outweighs
the label it sits with.

### Named Rules

**The Action Rule.** Icons mark actions and destinations: buttons, rail
navigation, and destructive controls. Selector chips — filters, sort, preset,
and appearance — stay text-only, because their state is communicated by the
selected tone and weight that this system already defines.

**The Status Vocabulary Rule.** Import status keeps its documented text markers
(`✓ Imported`, `! Possible duplicate`, `● New`). They are a written status
notation, not an icon system, and are never replaced by glyphs.

**The Paired-Badge Rule.** A media-type badge pairs one icon with one label
(`VIDEO`, `RAW+JPEG`, `SIDECAR`). A mark never stands alone where a reader would
have to guess its meaning.

## Logo

The app mark lives at `crates/captureport/assets/captureport.svg`. It is nine
identical seven-segment strokes on one grid: four green strokes form the `C`
(top-left and bottom-left arms on the left rail) and five amber strokes form the
`P` (top-right, upper-right, and middle-right bowl on the centre rail). Every
rail is two segments meeting at the middle, so the centre stem breaks exactly
like the left rail. The stem sits in the C's opening, so the strokes read as
`CP` while still tracing the frame and cross of `田`. It installs to the
hicolor icon theme as `captureport.svg` and the desktop entry names it with
`Icon=captureport`. `#9ccbad` (Pine accent) and `#e8b06a` (Darkroom accent) on
`#171a19` tie the mark back to the bundled schemes.

## Layout

The desktop shell is vertical: a 48px header, a flexible body, and a 52px footer. The header carries the wordmark lockup on the left — the Spectral logotype, a 1px vertical rule, and the descriptor — with the theme toggle at the right. The body places a fixed source rail beside a fluid content area. The rail is 232px wide at normal windows and 184px below the single 760px window-width breakpoint.

The browser uses a reflowing contact sheet with 16px grid gaps and 16px content gutters. Thumbnail targets remain the existing stepped sizes; each image well uses `ObjectFit::Contain` so photographs are never cropped. The layout reserves 208px for browser control chrome and 104px for media metadata; the resulting media row chrome is 122px (104px details + 16px grid gap + 2px borders). A grouped gallery heading contributes 44px (36px control height + 8px control gap).

The footer keeps selection totals and “Preview import” together. It remains present while status messages change, so the review action has a stable location.

Every page uses 16px header/footer gutters. Settings fields use 16px group separation, with an 8px section margin plus the parent 16px rhythm for a 24px section step; headings end 4px before their content. Hidden sidebar actions are removed from layout so unavailable actions leave no phantom gap. Scrollbars reserve a 16px gutter with a 4px inset.

## Elevation & Depth

CapturePort is flat by default. Depth comes from tonal surface changes and 1px borders; there are no shadows, blur layers, or decorative motion. Every overlay drawn on a photograph — the media-type badge and the selection label — uses a solid black scrim for legibility over photographs, because a theme surface cannot promise contrast against an image of unknown luminance.

**The Border-Before-Background Rule.** Use a border or a quiet tonal shift to separate states before adding visual weight.

## Shapes

The system uses a 4px radius for buttons, filters, navigation rows, fields, cards, and media tiles. Image overlays use the same compact radius. Borders are 1px and palette-derived. Buttons, navigation rows, and filters use a 36px minimum height with 16px horizontal and 4px vertical padding. Inputs use a 34px inner height plus 2px of borders for a 36px outer control.

## Components

### Header

- A 48px neutral bar. On the left, the wordmark lockup: `CapturePort` in Spectral Bold 22px, then a 1px `border` rule 20px tall, then the descriptor `Photo and video ingest` in Outfit 14px `header_muted`. The theme toggle sits at the right.
- The serif logotype is the one display element always on screen; it replaced the former 16px sans wordmark and is where the bar gets its weight. The bar keeps its 48px height, `header` surface, and 1px bottom border.

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
- Type badge, file size, and explicit selection label sit over or beside the image without cropping it. The type badge and the selection label both carry the black scrim; the selection label is never a themed surface.
- Metadata is separate: filename, time plus size, import status, then “View files” / “Hide files” for bundles. The metadata caption carries its own surface — `card` when unselected, `selected` when selected — so the title and metadata never float directly on the page canvas.
- The filename wrapper is explicitly full width before ellipsis truncation, so long source names remain stable in the grid.
- Import status always uses icon plus text, including prior-session detail when available.

### Footer and Import Disclosure

The 52px footer presents item totals, selected totals, byte totals, status text, and the existing Preview import action. Bundle disclosure remains a separate row interaction from import status, so “Selected”, “Part selected”, status, and “View files” communicate distinct facts.

## Do's and Don'ts

### Do:

- **Do** preserve all four schemes and both modes as complete source palettes.
- **Do** keep the 48px header, 232/184px rail, 52px footer, and 4px/36px control language coherent.
- **Do** use Spectral for the wordmark and page, panel, section, and empty-state headings, and Outfit for everything else; keep both OFL assets with their licenses.
- **Do** keep thumbnails contained and expose file size, import status, and bundle disclosure as separate metadata.
- **Do** keep every control icon on the Material Symbols grid at 16px, or 12px inside a 12px badge label.
- **Do** keep action icons tinted from the same token as their label text.
- **Do** label selection explicitly and keep color from carrying status by itself.

### Don't:

- **Don't** reintroduce a broad colored masthead, saturated filter pills, repeated filled secondary buttons, shadows, or decorative gradients.
- **Don't** crop photographs to fit a tile.
- **Don't** merge bundle disclosure into the import-status label.
- **Don't** add a new scheme or mode with inherited or partial palette roles.
- **Don't** replace the documented import-status text markers with icons.
- **Don't** put icons on filter, sort, preset, or appearance chips.
- **Don't** author a one-off icon outside the vendored Material Symbols set.
- **Don't** change the importer’s deterministic plan, verification, collision, recovery, or source-safety behavior as part of visual work.
