# Interface catalogs

These UTF-8 JSON catalogs are embedded into the desktop binary; translation needs
no runtime service, internet connection, or additional packaging step.

`en.json` defines the complete English-keyed interface vocabulary. Every other
catalog must have the same keys. Use numbered placeholders (`{0}`, `{1}`, …) in
formatted messages; translators may reorder them but must preserve each one.
Literal import-template segments, formatting directives, sample paths, format
identifiers, and the media-rule input vocabulary must also remain unchanged.
Never translate filenames, device identities, user-created preset/gallery names,
or values stored as backend identifiers.

Translations were machine-assisted, with manual corrections to common controls
and Chinese workflow/safety wording. Native-speaker review is still recommended,
especially for detailed help. To add or revise a language, update the catalog,
`src/i18n.rs`, the language documentation, and run `cargo test -p captureport`.
Catalog tests check completeness, nonempty translations, and placeholder parity.
