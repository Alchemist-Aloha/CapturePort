# CapturePort agent instructions

## Source of truth

- Treat [`docs/spec.md`](docs/spec.md) as the normative product and behavior
  contract. Read the relevant sections before changing implementation behavior.
- Use [`docs/plan.md`](docs/plan.md) for milestone order and definitions of
  done. Keep implementation aligned with the plan unless the user directs
  otherwise.
- If a change introduces user-visible or application behavior not covered by
  the spec, update `docs/spec.md` in the same change. Keep README and packaging
  documentation aligned when commands or release behavior changes.

## Architecture

- This is a Rust workspace. Keep the import backend independent of GPUI; GPUI
  is a presentation layer over reusable engine code.
- Route every source through the `MediaSource` abstraction. The importer must
  not assume a source is seekable, writable, permanently mounted, or fast.
- Use one deterministic `ImportPlan` for both preview and execution. Do not
  duplicate destination or filename-generation logic between those paths.
- The filesystem is authoritative. SQLite stores settings, cache, and import
  history; deleting the database must not make the media library unusable.

## Safety invariants

- Source discovery is read-only. Reject traversal and unsafe symlink escapes,
  and never delete source files automatically.
- Do not silently overwrite destination files. Resolve collisions during
  planning and show the final destination paths before copying.
- Copy into CapturePort-owned partial files, verify the copy, atomically publish
  the final file, and only then record a successful database result.
- Preserve interruption and recovery behavior. Incomplete files must not look
  like completed imports.
- Use progressive duplicate detection (history lookup, quick BLAKE3
  fingerprint, then full hash when required). Never classify uncertain media as
  safely imported.

## Working conventions

- Inspect `git status` and the relevant diff before editing. Preserve unrelated
  user changes and avoid resetting or deleting files without explicit approval.
- Follow [`/home/likun/.codex/RTK.md`](/home/likun/.codex/RTK.md): prefix shell
  commands with `rtk` where a proxy command exists.
- Prefer small, focused changes and tests. Avoid adding dependencies or
  inventing behavior outside the spec and plan.
- Keep source, tests, documentation, and packaging changes synchronized when
  they describe the same behavior.

## Verification

Run the narrowest relevant tests while iterating, then use the full ladder
when practical:

```sh
rtk cargo fmt --all -- --check
rtk cargo check --workspace --locked --offline
rtk cargo test --workspace --offline
rtk cargo clippy --workspace --all-targets --offline -- -D warnings
rtk git diff --check
```

If the offline cache is unavailable, report that limitation and run the
corresponding locked command when dependencies are available. Treat GPUI
compositor behavior, physical camera access, and other hardware-dependent
checks as separate runtime validation; do not claim those are covered by
static checks or unit tests.
