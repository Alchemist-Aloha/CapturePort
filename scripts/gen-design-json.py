#!/usr/bin/env python3
"""Generate .impeccable/design.json (DESIGN.md sidecar).

Colour tokens and all four schemes are read out of the Rust palette table, and the
narrative is parsed out of DESIGN.md, so the sidecar cannot drift from either.
Run from anywhere:  python3 scripts/gen-design-json.py
"""
import json
import math
import os
import re
import subprocess
import sys

os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

SRC = "crates/captureport/src/render.rs"
SCHEMES = ("Pine", "Darkroom", "Graphite", "Ink")

# ---------------------------------------------------------------- palette table
source = open(SRC).read()
body = source[source.index("fn new(scheme: ColorScheme, dark: bool) -> Self {"):]
body = body[:body.index("\n    }\n}")]

arms = {}
for match in re.finditer(r"\(ColorScheme::(\w+), (true|false)\) => Self \{(.*?)\n            \},", body, re.S):
    scheme, dark, block = match.group(1), match.group(2) == "true", match.group(3)
    arms[(scheme, dark)] = {
        f.group(1): f.group(2).lower().zfill(8 if len(f.group(2)) == 8 else 6)
        for f in re.finditer(r"(\w+):\s*(?:gpui::)?rgba?\(0x([0-9a-fA-F]+)\)", block)
    }

missing = {(s, d) for s in SCHEMES for d in (False, True)} - set(arms)
if missing:
    sys.exit(f"failed to parse palettes from {SRC}: {sorted(missing)}")
roles = set.intersection(*[set(v) for v in arms.values()])
if len(roles) != 19:
    sys.exit(f"expected 19 palette roles, found {len(roles)}: {sorted(roles)}")

# ------------------------------------------------------------- sRGB -> OKLCH
def srgb_to_linear(c):
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4

M1 = ((0.4122214708, 0.5363325363, 0.0514459929),
      (0.2119034982, 0.6806995451, 0.1073969566),
      (0.0883024619, 0.2817188376, 0.6299787005))
M2 = ((0.2104542553, 0.7936177850, -0.0040720468),
      (1.9779984951, -2.4285922050, 0.4505937099),
      (0.0259040371, 0.7827717662, -0.8086757660))
STEPS = [15, 26, 38, 49, 61, 72, 84, 95]


def oklch(hex6):
    linear = [srgb_to_linear(int(hex6[i:i + 2], 16) / 255) for i in (0, 2, 4)]
    lms = [sum(M1[i][j] * linear[j] for j in range(3)) for i in range(3)]
    lms = [math.copysign(abs(v) ** (1 / 3), v) for v in lms]
    lightness, a, b = [sum(M2[i][j] * lms[j] for j in range(3)) for i in range(3)]
    return lightness, math.hypot(a, b), math.degrees(math.atan2(b, a)) % 360


def ramp(hex8):
    lightness, chroma, hue = oklch(hex8[:6])
    canonical = f"oklch({lightness * 100:.1f}% {chroma:.3f} {hue:.1f})"
    return canonical, [f"oklch({s}% {chroma:.3f} {hue:.1f})" for s in STEPS]


# frontmatter slug <-> Rust palette role, plus the role vocabulary the panel shows
SLUG = {
    "canvas": ("warm-paper", "neutral", "Warm Paper"),
    "panel": ("pale-sage", "neutral", "Pale Sage"),
    "card": ("bright-paper", "neutral", "Bright Paper"),
    "border": ("soft-fern", "neutral", "Soft Fern"),
    "muted": ("moss-grey", "neutral", "Moss Grey"),
    "placeholder": ("faint-sage", "neutral", "Faint Sage"),
    "selected": ("mint-wash", "neutral", "Mint Wash"),
    "accent": ("pine-signal", "primary", "Pine Signal"),
    "danger": ("pale-clay", "danger", "Pale Clay"),
    "text": ("deep-ink", "neutral", "Deep Ink"),
    "header": ("cedar", "chrome", "Cedar"),
    "header_text": ("paper-white", "chrome", "Paper White"),
    "header_muted": ("pale-mint", "chrome", "Pale Mint"),
    "primary_bg": ("pine-button", "action", "Pine Button"),
    "primary_hover": ("pine-button-deep", "action", "Pine Button Deep"),
    "ghost_bg": ("ghost-fern", "action", "Ghost Fern"),
    "ghost_hover": ("ghost-fern-hover", "action", "Ghost Fern Hover"),
    "ghost_border": ("ghost-fern-line", "action", "Ghost Fern Line"),
}

pine_light = arms[("Pine", False)]
color_meta = {}
for role, (slug, kind, display) in SLUG.items():
    canonical, tonal = ramp(pine_light[role])
    color_meta[slug] = {"role": kind, "displayName": display, "canonical": canonical, "tonalRamp": tonal}

schemes = {
    scheme.lower(): {
        "light": {r: f"#{v}" for r, v in sorted(arms[(scheme, False)].items())},
        "dark": {r: f"#{v}" for r, v in sorted(arms[(scheme, True)].items())},
    }
    for scheme in SCHEMES
}

# ------------------------------------------------------ narrative from DESIGN.md
doc = open("DESIGN.md").read()
markdown = doc.split("---\n", 2)[2]
section_of = {"Overview": "overview", "Colors": "colors", "Typography": "typography",
              "Layout": "layout", "Elevation & Depth": "elevation", "Shapes": "shapes",
              "Components": "components"}

current, sections = None, {}
for line in markdown.splitlines():
    if line.startswith("## "):
        current = line[3:].strip()
        sections[current] = []
    elif current is not None:
        sections[current].append(line)


def paragraphs(lines):
    """Unwrap hard-wrapped prose into one string per paragraph."""
    out, buffer = [], []
    for line in lines:
        if line.strip():
            buffer.append(line.strip())
        elif buffer:
            out.append(" ".join(buffer))
            buffer = []
    if buffer:
        out.append(" ".join(buffer))
    return out


overview_lines = sections["Overview"]
opening = next(i for i, l in enumerate(overview_lines) if l.startswith("**Creative North Star"))
characteristics_at = next(i for i, l in enumerate(overview_lines) if l.startswith("**Key Characteristics:**"))

rules = []
for heading, lines in sections.items():
    for line in lines:
        match = re.match(r"\*\*The (.+?) Rule\.\*\* (.+)", line.strip())
        if match:
            rules.append({
                "name": f"The {match.group(1)} Rule",
                "section": section_of.get(heading, heading.lower()),
                "body": match.group(2).strip(),
            })


def bullets(heading, marker):
    marker_text = f"- **{marker}**"
    chosen = [l.strip() for l in sections[heading] if l.strip().startswith(marker_text)]
    return [line.split(marker_text, 1)[1].strip() for line in chosen]


narrative = {
    "northStar": re.search(r'\*\*Creative North Star: "([^"]+)"\*\*', markdown).group(1),
    "overview": "\n\n".join(paragraphs(overview_lines[opening + 1:characteristics_at])),
    "keyCharacteristics": [l.strip()[2:] for l in overview_lines[characteristics_at + 1:]
                           if l.strip().startswith("- ")],
    "rules": rules,
    "dos": bullets("Do's and Don'ts", "Do"),
    "donts": bullets("Do's and Don'ts", "Don't"),
}

# -------------------------------------------------------------------- components
components = [
    {
        "name": "Primary Button", "kind": "button", "refersTo": "button-primary",
        "description": "The single affirmative action on a screen: confirm an import, apply settings, preview the plan.",
        "html": '<button class="ds-btn-primary">Confirm import</button>',
        "css": (".ds-btn-primary { background: #247c66; color: #ffffff; border: none; border-radius: 6px;"
                " padding: 8px 16px; min-height: 38px; font: 600 14px system-ui, sans-serif; cursor: pointer;"
                " transition: background 0.12s linear; }"
                " .ds-btn-primary:hover { background: #195d4c; }"
                " .ds-btn-primary:focus-visible { outline: 2px solid #276f51; outline-offset: 2px; }"),
    },
    {
        "name": "Secondary Button", "kind": "button", "refersTo": "button-ghost",
        "description": "Every action that is not the primary one: a translucent tint with a hairline of the same hue.",
        "html": '<button class="ds-btn-secondary">Refresh devices</button>',
        "css": (".ds-btn-secondary { background: rgba(80, 131, 107, 0.133); color: #1d2f24;"
                " border: 1px solid rgba(87, 133, 109, 0.4); border-radius: 6px; padding: 8px 12px;"
                " min-height: 36px; font: 400 14px system-ui, sans-serif; cursor: pointer;"
                " transition: background 0.12s linear; }"
                " .ds-btn-secondary:hover { background: rgba(80, 131, 107, 0.267); }"
                " .ds-btn-secondary:focus-visible { border-color: #276f51; outline: none; }"),
    },
    {
        "name": "Filter Chip", "kind": "chip", "refersTo": "chip-active",
        "description": "Filter, sort, preset, scheme and appearance selection; the active state is the scheme's primary fill.",
        "html": ('<div class="ds-chips"><button class="ds-chip ds-chip--active">All</button>'
                 '<button class="ds-chip">New</button></div>'),
        "css": (".ds-chips { display: flex; gap: 8px; }"
                " .ds-chip { background: rgba(80, 131, 107, 0.133); color: #1d2f24; border: none;"
                " border-radius: 6px; padding: 4px 12px; font: 400 14px system-ui, sans-serif; cursor: pointer;"
                " transition: background 0.12s linear; }"
                " .ds-chip:hover { background: rgba(80, 131, 107, 0.267); }"
                " .ds-chip--active { background: #247c66; color: #ffffff; }"
                " .ds-chip--active:hover { background: #195d4c; }"
                " .ds-chip:focus-visible { outline: 2px solid #276f51; outline-offset: 2px; }"),
    },
    {
        "name": "Text Input", "kind": "input", "refersTo": "input",
        "description": "Settings and gallery-name fields. Focus is a single 1px border colour swap, with no glow.",
        "html": ('<label class="ds-field"><span class="ds-field-label">Photo destination</span>'
                 '<input class="ds-input" value="~/Pictures" /></label>'),
        "css": (".ds-field { display: flex; flex-direction: column; gap: 4px; }"
                " .ds-field-label { color: #4d6355; font: 400 12px system-ui, sans-serif; }"
                " .ds-input { background: #fffef9; color: #1d2f24; border: 1px solid #d3dfd2; border-radius: 6px;"
                " padding: 8px 12px; font: 400 14px system-ui, sans-serif; }"
                " .ds-input:focus { border-color: #276f51; outline: none; }"),
    },
    {
        "name": "Sidebar Navigation Item", "kind": "nav", "refersTo": "nav-item-active",
        "description": "Text-only navigation; the active row is a tonal fill plus semibold, never an icon or indicator bar.",
        "html": ('<nav class="ds-nav"><button class="ds-nav-item ds-nav-item--active">Browse media</button>'
                 '<button class="ds-nav-item">History</button></nav>'),
        "css": (".ds-nav { display: flex; flex-direction: column; gap: 8px; padding: 16px; background: #eff2e9; width: 238px; }"
                " .ds-nav-item { background: transparent; color: #1d2f24; border: none; border-radius: 6px;"
                " padding: 8px 12px; min-height: 36px; text-align: left; font: 400 14px system-ui, sans-serif;"
                " cursor: pointer; transition: background 0.12s linear; }"
                " .ds-nav-item:hover { background: #dcefdc; }"
                " .ds-nav-item--active { background: #dcefdc; font-weight: 600; }"
                " .ds-nav-item:focus-visible { outline: 2px solid #276f51; outline-offset: 2px; }"),
    },
    {
        "name": "Media Tile", "kind": "card", "refersTo": "media-card",
        "description": "The grid unit: full-bleed cover image, overlay pills on the photograph, metadata below; selection moves background and border together.",
        "html": ('<figure class="ds-tile ds-tile--selected">'
                 '<div class="ds-tile-image"><span class="ds-pill ds-pill--type">\u25a3 RAW</span>'
                 '<span class="ds-pill ds-pill--size">24.8 MB</span></div>'
                 '<figcaption class="ds-tile-meta"><span class="ds-tile-name">DSC02341.ARW</span>'
                 '<span class="ds-tile-time">14:32</span>'
                 '<span class="ds-tile-status">\u2713 Imported</span></figcaption></figure>'),
        "css": (".ds-tile { width: 210px; margin: 0; border: 1px solid #d3dfd2; border-radius: 6px;"
                " background: #fffef9; overflow: hidden; }"
                " .ds-tile--selected { border-color: #276f51; background: #dcefdc; }"
                " .ds-tile-image { position: relative; aspect-ratio: 3 / 2; background: #e5ede3;"
                " display: flex; align-items: flex-end; justify-content: space-between; padding: 4px; }"
                " .ds-pill { background: rgba(0, 0, 0, 0.6); color: #ffffff; border-radius: 4px;"
                " padding: 0 4px; font: 600 12px system-ui, sans-serif; }"
                " .ds-pill--size { font-weight: 400; }"
                " .ds-tile-meta { display: flex; flex-direction: column; padding: 4px 8px 8px; }"
                " .ds-tile-name { font: 600 14px system-ui, sans-serif; color: #1d2f24;"
                " white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }"
                " .ds-tile-time, .ds-tile-status { font: 400 12px system-ui, sans-serif; color: #4d6355; }"
                " .ds-tile-status { font-weight: 600; }"),
    },
    {
        "name": "Status Readout", "kind": "custom", "refersTo": "chip-idle",
        "description": "Import status as icon plus text, naming the prior import for imported and possible-duplicate states; colour never carries the state on its own.",
        "html": ('<ul class="ds-status"><li class="ds-status-item">\u2713 Imported \u00b7 session 12 \u00b7 2026-09-27</li>'
                 '<li class="ds-status-item">! Possible duplicate \u00b7 session 9 \u00b7 2026-09-20</li>'
                 '<li class="ds-status-item">? Unknown</li><li class="ds-status-item">New</li>'
                 '<li class="ds-status-item">Checking\u2026</li></ul>'),
        "css": (".ds-status { display: flex; flex-direction: column; gap: 4px; margin: 0; padding: 0; list-style: none; }"
                " .ds-status-item { font: 600 12px system-ui, sans-serif; color: #4d6355; }"),
    },
    {
        "name": "Tonal Slider", "kind": "custom", "refersTo": "chip-active",
        "description": "Thumbnail size and session gap are stepped ramps, not drag handles; every lit step up to the current value.",
        "html": ('<div class="ds-ramp" role="slider" aria-valuenow="3" aria-valuemin="1" aria-valuemax="5"'
                 ' aria-label="Thumbnail size" tabindex="0">'
                 '<span class="ds-ramp-step ds-ramp-step--on"></span><span class="ds-ramp-step ds-ramp-step--on"></span>'
                 '<span class="ds-ramp-step ds-ramp-step--on"></span><span class="ds-ramp-step"></span>'
                 '<span class="ds-ramp-step"></span></div><span class="ds-ramp-value">250 px</span>'),
        "css": (".ds-ramp { display: flex; align-items: center; gap: 4px; }"
                " .ds-ramp-step { width: 30px; height: 28px; display: flex; align-items: center; cursor: pointer; }"
                " .ds-ramp-step::after { content: ''; display: block; width: 100%; height: 4px;"
                " border-radius: 9999px; background: #d3dfd2; }"
                " .ds-ramp-step--on::after { height: 6px; background: #276f51; }"
                " .ds-ramp:focus-visible { outline: 2px solid #276f51; outline-offset: 2px; }"
                " .ds-ramp-value { font: 400 12px system-ui, sans-serif; color: #4d6355; }"),
    },
]

# ----------------------------------------------------------------------- output
design = {
    "schemaVersion": 2,
    "generatedAt": subprocess.check_output(["date", "-u", "+%Y-%m-%dT%H:%M:%SZ"]).decode().strip(),
    "title": "Design System: CapturePort",
    "extensions": {
        "northStar": narrative["northStar"],
        "colorMeta": color_meta,
        "typographyMeta": {
            "display": {"displayName": "Display", "purpose": "The CapturePort wordmark in the header, and nothing else."},
            "headline": {"displayName": "Headline", "purpose": "Panel titles and empty-state headlines."},
            "title": {"displayName": "Title", "purpose": "Filenames, button and navigation labels, leading summary lines."},
            "body": {"displayName": "Body", "purpose": "Default interface size: footer text, settings values, sentence copy."},
            "label": {"displayName": "Label", "purpose": "Metadata, timestamps, counts, field captions, tile overlays."},
            "fontFamilyNote": ("No family is configured; GPUI resolves the platform UI font. "
                               "There is no secondary or monospaced face."),
        },
        "shadows": [],
        "motion": [],
        "breakpoints": [
            {"name": "sidebar-compact", "value": "760px",
             "purpose": ("Single breakpoint. Below it the sidebar narrows from 238px to 200px; "
                         "the media grid reflows continuously between the two widths.")},
        ],
        "schemeNote": ("Colour scheme and light/dark mode are independent axes. Each scheme defines all 19 palette "
                       "roles for both modes; DESIGN.md's frontmatter records Pine light, the default configuration."),
        "schemes": schemes,
        "paletteRoles": sorted(roles),
        "componentNote": ("These snippets are CSS translations of GPUI-rendered components, for the live panel's shadow "
                          "DOM; the application itself renders through GPUI, not HTML/CSS."),
    },
    "components": components,
    "narrative": narrative,
}

os.makedirs(".impeccable", exist_ok=True)
with open(".impeccable/design.json", "w") as handle:
    json.dump(design, handle, indent=2, ensure_ascii=False)
    handle.write("\n")

print(f"wrote .impeccable/design.json: {len(color_meta)} colour tokens, {len(components)} components, "
      f"{len(schemes)} schemes x 2 modes x {len(roles)} roles, {len(rules)} named rules")
