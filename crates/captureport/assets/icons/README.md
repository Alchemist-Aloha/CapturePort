# Interface icons

Material Symbols Outlined (Material 3) at 24px, vendored unmodified.

- Upstream: https://github.com/google/material-design-icons
- Path: `symbols/web/<name>/materialsymbolsoutlined/<name>_24px.svg`
- Revision: `bd8cb85bd4bad964fe6918f79665bb40c3a8efef`
- License: Apache License 2.0, see [LICENSE.txt](LICENSE.txt)

Each file keeps the upstream 960-unit grid (`viewBox="0 -960 960 960"`) and its
single filled `<path>`, so the geometry is verifiable against upstream.
CapturePort renders them as masks tinted by the control's text color, so one
file serves every scheme and both modes.

Icon names here are the upstream Material Symbols names; `src/icons.rs` maps
them to interface semantics.
