# CapturePort improvement ledger

## Scope and acceptance

Baseline: `d341ac8` (v0.6.0), initially clean working tree. Repeat
**discover → baseline → prioritize → implement → test → compare → commit/reject**.
Only demonstrated improvements are accepted; no speculative speed claims.
Real photos, removable-media contents, user settings and production history are
out of bounds for destructive tests. GUI tests use isolated XDG directories.
The large Ralph iteration allowance is a harness guard, not a stopping criterion.

## Architecture / intended semantics

- Eight workspace crates: `core` (source contract, media/events, generation-aware
  app state and fake sources); `media` (read-only filesystem adapter); `gphoto`
  (serialized camera worker and discovery); `metadata` (EXIF/ffprobe extraction,
  bounded thumbnail workers/cache); `ingest` (bundles, templates, one deterministic
  planner, fingerprints, transactional import, recovery/reconciliation/deletion);
  `catalog` (SQLite worker); GPUI app; standalone synthetic benchmark.
- The GPUI `Browser` owns navigation, selection, bundles, settings and async work
  receivers. Five pages: Browser, Preview, History, Recovery, Settings. Scan
  metadata has two workers / 64 queued jobs; source previews use a 32-job queue;
  filesystem thumbnails use two workers / 128 jobs. UI work drains every 16 ms;
  device discovery polls every 3 s. These are implementation facts, not proven
  optimal settings.
- SQLite schema v3: sources, media, import sessions/results, presets and manual
  import declarations. One catalog worker serializes commands; clients wait for
  replies. Old-schema migration uses an online backup. The filesystem remains
  authoritative; observation/quick matches are not proof of a completed import.
- Import: plan final paths → create owned partial → stream/hash → verify →
  atomically publish without replacement → record history. Cancellation keeps
  durable completed results, removes only owned partials, and must not report
  incomplete copies as successful. Source deletion is separately confirmed and
  revalidates source and destination content/identity.
- Rust/Cargo 1.99.1; GPUI 0.2.2; stable toolchain selected by repository. CI checks
  formatting/check/tests. Debian/Arch/Fedora/portable packaging uses locked Cargo.
  Existing macro/xattr compatibility patches are documented, not new findings.

## Baseline measurements

Host: Linux 7.3.0-4-cachyos x86_64, i5-13600K (20 logical CPUs).
Checkout is btrfs; temporary benchmark/native fixtures under `/tmp` are **tmpfs**.
These probes must not be interpreted as physical-storage throughput/durability.
Command: `rtk cargo build -p captureport-bench --release --locked --offline`,
then run `target/release/captureport-bench` sequentially six times; discard the
first warm-up and report median and min–max of the other five. Per-process CPU
and peak RSS are collected using Linux `wait4` rusage. Conditions are warm-cache,
normal desktop activity; these are not hardware camera/storage benchmarks.
Raw local evidence: `/tmp/captureport-baseline-z3e1z8bn/{run-0..5.tsv,raw.json}`.

| Probe | Median ms | Min–max ms |
|---|---:|---:|
| Fake enumeration, 10,000 photos + 500 videos | 2.032 | 1.953–3.358 |
| Catalog upsert + lookup, 21,000 synchronous requests | 394.337 | 388.661–430.951 |
| Browser name sort, 100 × 10,500 items | 91.664 | 91.055–111.334 |
| Template generation, 10,500 items | 5.566 | 5.438–5.606 |
| Planner, 10,500 items | 65.307 | 64.032–66.051 |
| Thumbnail queue, **placeholder jobs only** | 20.888 | 19.690–23.083 |
| Quick hash of an 8 MiB file | 0.196 | 0.191–0.207 |
| Full hash of an 8 MiB file | 2.689 | 2.554–3.281 |

Whole probe median: wall **0.598 s** (0.594–0.657), CPU **0.636 s**
(0.636–0.690), peak RSS **40,340 KiB** (39,168–40,900).
The 1 TB card is represented metadata, not copied payload. The quick-hash probe
currently labels logical file bytes as bytes hashed: actual samples are 512 KiB,
not 8 MiB. Its printed byte throughput is **not** an accepted measurement.
Real RAW/video decode, filesystem traversal, copy throughput, startup/idle usage,
frame latency and large duplicate-heavy collections still need separate probes.

Coverage baseline: **179 existing tests**. The earlier combined baseline tool
output was truncated; it is not independent pass evidence here. Fresh post-change
verification below reruns the complete ladder and includes those existing tests.

## UI/workflow inventory and coverage

This is an inventory of implemented surfaces, **not** a claim of exhaustive
runtime coverage. `S` = source/spec inventory; `T` = related logic tests;
`G` = actual GUI exercised in this loop. Partial `G` coverage below is limited
to the stated states. Prior-session screenshots are historical evidence only.

| Surface / workflow | Principal implementation | Coverage / next check |
|---|---|---|
| Fresh launch, source-empty/loading/error states | main, browser/render + views | S/T/G: fresh empty light/dark; demo loading/ready; errors pending |
| Sources, refresh, mount, disconnect/reconnect | devices, browser/sources, gphoto/discovery | S/T/G: GTK opens/cancels; private broker selects fixtures; actual GTK choice/camera separate |
| Grid/filter/sort/selection and gallery grouping/rename | browser/views + gallery + session | S/T/G: populated demo grid at 1180×900 / 720×640; interactions/latency pending |
| RAW+JPEG bundles and member selection | ingest/bundle, browser/bundles + views | S/T/G: merged tile/JPEG preview/both files copied; member interaction pending |
| Details popup, original URL copy, dismissal/focus | browser/detail | S/T; G pending |
| Image/video previews, unavailable fallback, progress strip | metadata/thumbnail, browser/thumbnails + render | S/T/G: generated JPEG + ffmpeg video decode; paired RAW placeholder; genuine RAW/unavailable-tile G pending |
| Plan preview, collisions, backup paths, confirmation | ingest/planner, browser/import + panels | S/T/G: exact paths, first/re-import confirmation, late collision; backup UI pending |
| Import phases, cancel, outcomes | ingest/importer, tasks, browser/render | S/T/G: actual filesystem import/re-import/cancel/late-error; camera phases separate |
| History and session-detail inspection | catalog, browser/import + panels | S/T/G: list + row Enter expands details (focus ring visible) |
| Recovery, resume, partial cleanup confirmation | ingest/recovery, browser/import + panels | S/T/G: injected interrupted session + partial, arm→Enter confirm; resume path and camera recovery pending |
| Verified-original deletion confirmation/cancel/refusal | ingest/source_delete, tasks, browser/import | S/T; disposable fixture only; G pending |
| Presets: create/load/rename/overwrite/delete | saved_presets, browser/settings + panels | S/T/G/db: create + rename by keyboard, restart persistence; overwrite/delete unproven |
| Destinations, filename/folder templates and segment help | template_help, browser/panels, render | S/T/G: photo help expands/collapses by keyboard; token insertion/full form pending |
| Media rules/folder exclusions, clock/timezone/session gap | browser/settings + panels, ingest/preset | S/T; DST/edge/G pending |
| Backup, source alias, apply settings | browser/settings + panels | S/T; failure/persistence/G pending |
| Theme/language/thumbnail size and maintenance actions | i18n, browser/settings + panels | S/T/G: light/dark + restart persistence + compact; other actions pending |
| All 18 text fields, edit menu, keyboard/focus/scroll | text_edit, text_input, widgets | S/T/G: preset-name typing + Ctrl+D guard; other fields/menu/IME/clipboard pending |
| All shared controls/tooltips/disabled/hover/focus states | widgets, render, browser modules | S/T/G: theme/refresh/help/history-row/partial-confirm; PageUp/PageDown unit-tested only (native key injection blocked); settings cyclers not natively reached; remaining click-only rows pending |

## Ranked evolving backlog

1. **Critical / confirmed:** edge-only collision verification can record distinct
   media as imported. Iteration 1 below fixes this before performance work.
2. **High / partly fixed:** unsafe leaves/dangling suffixes (iteration 2) and
   destination replacement or observable rewrite during verification (iteration
   3) are fixed. Remaining: source/destination overlap, recovery/history
   durability, and filesystem snapshot limitations. Native/coarse-timestamp
   filesystem behavior is not claimed covered by btrfs fixtures.
3. **High / partly fixed:** serial-capable PTP cameras now retain their key across
   USB address changes (iteration 4). Missing/non-unique serials still require
   the conservative port fallback; stable association is blocked without reliable
   identity evidence. Existing port-based history is retained, not auto-merged.
4. **High / investigate:** concurrent camera reopen/source switching and
   generation-scoped completion/cancellation; blocked drivers and worker shutdown.
5. **Fixed in iteration 10:** Standard verification now reads only its edge
   samples (512 MiB: ~220 µs vs ~152 ms). Strict and pre-existing-file full
   verification are unchanged. Residual: Standard cannot detect middle-byte
   corruption by design; use Strict where that matters.
6. **Medium / measured opportunity:** catalog request sequence dominates the
   synthetic probe (~394 ms). Profile actual scan/import callers before batching
   or changing transaction semantics; memory DB is not durable import throughput.
7. **Medium / investigate:** progressive gallery rebuilding/name lookup and frame
   allocations, thumbnail cache/queue invalidation, obsolete work and decoder
   subprocess lifecycle. Measure real mixed-media cases.
8. **Measured, not the poll timer:** idle CPU is **3.4% of one core** (median of 15×10 s
   windows) and did not change when the drain poll was slowed from 16 ms to 1 s,
   so the drain is not the cost. An adaptive-poll patch was **reverted** as
   unsupported. Open question: GPUI/compositor frame scheduling for a visible idle
   window (an inconclusive hide-to-special-workspace probe raised CPU instead).
   Also unchanged: UI blocking I/O,
   keyboard/grid navigation, resize, settings feedback and unsafe re-entrancy.
   `docs/todo.md` contains an older critique: verify, do not assume still current.
9. **High / fixed in iteration 6:** interactive demo now opens the isolated CLI
   entry point in a separate window. Native normal-catalog counts stay zero
   while the demo is populated. Pre-existing synthetic rows are not purged.
10. **High / fixed in iteration 7:** theme/dynamic-label control IDs no longer
    discard focus. Native theme/refresh/template-help and input shortcut guard
    pass. Remaining audit: focus handoff when a control is actually removed,
    and keyboard reachability of inline template token controls.
11. **Fixed in iteration 9:** deletion entry/arming now share the exact verified
    copy policy, and the post-import header reports verified/failed (or cancelled)
    instead of planned copies "to copy". Per-row text still describes the plan;
    a completed import leaves rows labelled by their original plan status.
12. **Measurement gap:** correct quick-hash byte accounting, add realistic
   filesystem/decode/import/duplicate-heavy/fault baselines without claiming
   synthetic placeholder or 1 TB metadata tests cover those workloads.

## Iteration 1 — prevent false successful existing-file imports

**Issue →** A 1 MiB source/destination pair with equal sizes and identical first
and last 256 KiB, but one changed middle byte, was planned as an existing-file
candidate and recorded successfully in Fast and Standard modes.

**Root cause →** execution upgraded Fast only to Standard; Standard compared quick
fingerprints even though both full digests had already been read. Edge samples
cannot establish content identity for a file CapturePort did not just copy.

**Change →** pre-existing candidates always require Strict/full BLAKE3 before
recording success, independent of the mode for new copies. History receives the
actual Strict mode. Preview says “Existing file · verify first” instead of
asserting identity before checking; all ten catalogs and spec are synchronized.
The planner still uses cheap size/edge screening, not full enumeration hashing.

**Before/after →** deterministic failing regression before fix:
`[(Fast, Completed, 1 record), (Standard, Completed, 1), (Strict, Failed, 0)]`.
After fix all three produce `Failed, 0 records`; both source and destination
bytes remain unchanged. Positive re-imports in all modes still complete and
record Strict verification. No performance gain is claimed for this safety fix.

**Validation →** two importer regressions cover different-middle rejection and
positive existing-file recording across all three modes. Focused existing-file
suite: 4 passed; application/localization suite: 63 passed. Fresh full ladder:
formatting, locked/offline workspace check, **181 tests / 15 suites**, all-target
Clippy `-D warnings`, and diff whitespace checks pass. Existing-file cancellation
and no-overwrite regressions remain green. This reproduces false success, not an
actual source deletion: the separate deletion path already revalidates content.
The preview wording has source/translation tests, not native visual verification.

**Decision → ACCEPT**; committed as **`d671db6`**
(`fix: require full identity before accepting existing imports`).

## Iteration 2 — reject unsafe destination leaves without blocking

**Issue →** a destination file replaced by an external symlink after preview was
still recorded `Completed` when its target matched. Path fingerprinting also
followed leaf links. Parent/root validation did not validate the leaf. Separate
inspection found FIFO opens could block before cancellation/type checks, and
`exists()` treated dangling symlinks as free suffix candidates.

**Root cause →** following path metadata, ordinary blocking `File::open`, and
separate size/open resolution. This is a different failure from sampled identity.

**Change →** planning uses non-following metadata, blocks non-regular leaves,
and counts every directory entry as occupied during unique-suffix resolution.
Shared descriptor opening uses `O_NOFOLLOW | O_NONBLOCK`, rejects non-regular
handles, and is used by quick fingerprints, every verification mode and full
library reconciliation. Size and digest now use the same opened handle. No new
dependencies. Unique-suffix still works for links/directories/FIFOs; Stop, Skip
and Verify do not treat those unsafe leaves as verified media.

**Before/after →** two regressions failed before implementation: a link was
fingerprinted successfully, and late-link execution returned `Completed` rather
than `Failed`. Both now reject it; source, target and link remain unchanged.
Additional tests cover directories and FIFOs with a bounded two-second test
completion deadline, all three verification modes, four collision policies and
an occupied dangling-link suffix. The old FIFO implementation was not deliberately
left blocking: the bounded regression was added with the nonblocking fix.

**Validation →** ingest suite 56 passed; full workspace **185 tests / 15 suites**,
formatting, locked/offline check, strict all-target Clippy and diff checks pass.
No GUI/hardware behavior claimed. Remaining race limits are explicit in the spec:
parent checks are not directory-FD capabilities, and final-path replacement during
verification still needs a separate deterministic investigation.

**Performance comparison →** same release benchmark, one warm-up + five runs:
planner median **63.513 ms** (62.697–64.886), versus baseline **65.307 ms**
(64.032–66.051). Whole-probe wall median **0.615 s** (0.603–0.653), CPU
**0.646 s** (0.645–0.689), peak RSS **39,396 KiB** (39,196–39,448).
Unmodified sort/catalog probes also moved, so **no speedup is claimed**. This
acceptance rests on safety regressions, not the small timing difference.
Raw evidence: `/tmp/captureport-leaf-after-xu3_nvra/{run-0..5.tsv,raw.json}`.

**Decision → ACCEPT**; committed as **`462829e`**
(`fix: reject unsafe file types during destination verification`).

## Iteration 3 — bind verification success to the file still at the path

**Issue →** holding an open descriptor was insufficient: replacement, removal,
symlinking and rewriting already-read bytes during hashing all returned true,
including Strict mode. An unchanged descriptor can name an unlinked old version
while the planned destination names different media.

**Root cause →** verification compared the bytes read but did not revalidate the
path or file version after reading. This also allowed an in-place change behind
the reader to escape its digest.

**Change →** keep the opened descriptor alive, snapshot its device/inode, size,
mtime and ctime (with subsecond components), and require the final non-following
path metadata to name that unchanged regular file. Fast still checks only size,
but also revalidates path identity/version; Standard/Strict retain their content
policies. No additional data pass, dependencies or public APIs. Reuse the
existing counting recorder fixture for an end-to-end no-history regression.

**Before/after →** deterministic progress callbacks on disposable 3 MiB files:
all eight `(Standard/Strict × replace/rewrite/remove/symlink)` cases returned true
before the fix; all now return false/error. Rewrite changes an already-read byte;
the fixture starts with epoch mtime to avoid timestamp-resolution flakiness.
Engine-level late replacement now produces Failed with **zero success records**;
source, retained old destination and replacement contents all remain intact.

**Validation →** ingest 58 passed; full workspace **187 tests / 15 suites**,
format/check/strict all-target Clippy/diff checks pass (locked/offline). Existing
sample-boundary, positive re-import, cancellation and partial ownership tests
stay green. This is a safety change; no speedup is claimed or inferred from tests.

**Limits →** metadata-based change detection is not a stable filesystem snapshot.
Coarse timestamps can conceal same-inode, same-size rewrites; hostile directory
swaps or edits after the last check are not locked out. Stronger guarantees need
filesystem snapshot/writer coordination, not a promise from these path checks.
No FAT/exFAT runtime, actual GUI or camera behavior was validated this iteration.

**Decision → ACCEPT**; committed as **`123f9ea`**
(`fix: reject destination changes observed during verification`).

## Iteration 4 — stable identity for serial-capable PTP cameras

**Issue →** the camera key hashed model + transient `usb:BUS,DEVICE`, so ordinary
USB re-enumeration created a new source. Alias/gallery/history association could
break every reconnect. Discovery never populated the available USB serial.

**Baseline →** a synthetic sysfs descriptor with unchanged model/vendor/product/
serial and device number changing 2 → 48 failed the reconnect assertion before
implementation: `gphoto:ba646b945cce97a1` versus `gphoto:ba71e9945cd9f817`.
The fixture is removed before assertions; no real catalog or camera is used.

**Change →** read USB vendor/product/serial through sysfs for the matching bus/
device, prefer that serial-backed key, expose the serial in source metadata,
and use the chosen descriptor key consistently for enumerated media IDs.
Connection rejects a stale serial-backed selection before opening hardware.
Duplicate keys in one discovery snapshot fall back to individual port keys, so
repeated device-provided serials cannot hide/merge connected cameras. Missing,
blank or malformed attributes retain the old port-key behavior. No dependencies,
catalog migrations, or changes to existing source/history/settings rows.

**Validation →** three new tests: reconnect/serial/vendor distinction plus
missing/empty/malformed fallbacks using fake sysfs; repeated-serial separation;
and stale selection rejection without hardware open. Gphoto tests 25 passed;
full workspace **190 tests / 15 suites**, formatting, locked/offline check,
strict all-target Clippy and diff checks pass. No speedup claim; added sysfs
polling cost and physical-camera compatibility are not measured by these tests.

**Migration/limits →** an old port-based source can get one new serial-backed
record. Its old aliases/gallery names/history remain intact but are not guessed
onto the new source; reassignment may be needed. Devices lacking a usable unique
serial cannot promise reconnect identity. Repeated-serial setups may change keys
as peers appear/disappear; preserving separation takes precedence over merging.
Physical-camera discovery/transfer and UI continuity remain runtime checks.

**Decision → ACCEPT**; committed as **`3abcf75`**
(`fix: preserve serial-backed camera identity across reconnects`).

**Broad-review checkpoint →** four logical iterations covered different identity
and filesystem safety failures, but no fresh native `G` coverage exists. Next
iteration must move to an isolated native workflow/UI sweep and collect actual
startup/idle/workflow evidence rather than continuing only backend fixes.

## Iteration 5 — first native sweep; restore guarded app shortcuts

**Native setup →** rebuilt release binary; normal (not CLI demo) app with isolated
HOME/config/data/cache and generated JPEG/RAW-placeholder/video under
`/tmp/captureport-native-b8153740229241a69d738f54ba190f13`. Helper:
`/tmp/captureport-native-check.py`. Hyprland-targeted keys; no clipboard use.
All test processes/dialogs were stopped and original window focus restored.
Portal recents belong to the desktop service, not the isolated app; those local
screenshots are not committed or treated as isolated application data.

**Issue / before →** Ctrl+D and Ctrl+O do nothing in a fresh window, while
Tab×7/Enter reaches Settings. GPUI 0.2.2 evaluates guarded keybindings only when a
key context exists; the root had none, so `!CapturePortTextInput` matched nothing.

**Change / after →** give the Browser root a stable key context; extract the
existing shortcut table so a unit test checks the actual five guarded bindings
against empty, browser and nested text-input contexts. Native confirmation:
Ctrl+D now loads the large demo; Ctrl+O opens the folder picker, and Escape
cancels it. Text-field exclusion is unit-verified, **not** native field coverage.
No layout, palette, copy or shortcut semantics were redesigned.

**Actual visual/workflow coverage →** fresh empty light/dark, Settings navigation,
theme restart persistence, synthetic loading/progress/ready grid and compact
720×640 reflow. Photographs remain contained and selection/status labels remain
legible. Native folder selection could not be completed with bounded targeted
keyboard attempts; it was canceled, not counted as a successful filesystem
import. History/recovery/detail/preset/real-import/cancel/IME workflows remain.
No whole-app health score or screen-reader conformance is inferred from this
partial Linux check; mobile guideline scores do not apply to this desktop app.

**Measurements →** first compositor-window registration **0.529 s**, warm restart
before change **0.220 s**, after **0.283 / 0.288 s**. These include control IPC and
are **not first-frame timings or a supported performance comparison**. One 3 s
fresh empty idle sample: **3.67% of one CPU**, RSS 258,036→264,284 KiB, peak
269,240 KiB, 35 threads; exploratory n=1, no variance or optimization claim.
Raw measurements: `native-metrics.json` in the fixture directory.

**New ranked findings →** normal Ctrl+D left 7,500 synthetic media rows in the
isolated persistent catalog, unlike CLI `--demo`; this must not affect production.
Separately, theme activation loses keyboard focus (reproduced before and after
the context fix). The attempted native text-field guard check never reached an
input because of that focus loss; it is not a pass. Both are next high-priority
fixes. No source fixture was imported; destination stayed empty, with zero import
sessions. Source fixture hashes were recorded for identification, not claimed as
a before/after integrity comparison.

**Validation / decision → ACCEPT** guarded-shortcut fix: new predicate regression,
full workspace **191 tests / 15 suites**, format/check/strict Clippy/diff checks,
and fresh release build pass; native shortcut positive path verified.
Committed as **`8cef818`** (`fix: enable guarded app shortcuts in the browser context`). The native coverage checklist remains
open; finding blockers is not the same as completing its workflows.

## Reflection checkpoint / iteration 6 — isolate interactive demo history

**Reflection →** five verified fixes, 179→191 tests, and fresh native coverage
exposed failures beyond unit coverage. Disposable regressions/scoped commits
work well; keyboard-only picker troubleshooting was too long. Keep attempts
bounded, reject invalid screenshots, prioritize the demonstrated catalog/focus
blockers, then return to workflow and measured performance work.

**Issue/root cause →** normal Ctrl+D scanned Fake media through the live catalog;
CLI `--demo` already used memory. Baseline native fixture retained one Fake source
and 7,500 synthetic media rows in its ordinary persistent database.

**Change →** reuse `current_exe --demo 10000` in a separate window instead of
swapping the normal source or introducing multiple catalog namespaces in one
Browser. Reap its process off the UI thread; localized launch failure preserves
the normal window and offers restart/retry. Spec and both READMEs document the
boundary; all ten translation catalogs include the error.

**Before/after/validation →** a fresh isolated normal run opened a populated,
visible demo (5,000 capture previews / 7,500 accepted files) in another PID.
Persistent counts remained **0 sources / 0 media / 0 sessions**, normal window
remained source-empty, destination remained empty. Child disappearance while
parent stayed alive confirms reaping. Native artifacts and assertions:
`/tmp/captureport-native-c0f0be1e76dc44ca85d5971e6d4730df/`, including
`demo-isolation.json`. One off-workspace capture was rejected/deleted; accepted
capture required the target visible and active. Harness now checks that before
capturing. All test processes stopped; original focus restored; clipboard unused.
The new command-contract unit test, **192 tests / 15 suites**, full locked/offline
check ladder and fresh release build pass.

**Limits →** catalog isolation does not sandbox appearance/settings or confirmed
destination writes; tests must still isolate XDG and use disposable destinations.
No existing records are removed. Separate GUI process overhead is intentional,
not a performance improvement. Preserving a populated real source/selection was
reviewed in the no-mutation handler, not runtime-tested against user media.

**Decision → ACCEPT**; committed as **`fc97839`**
(`fix: launch interactive demos with isolated catalog history`).
Next: the independently reproduced theme/focus blocker, then native workflows.

## Iteration 7 — retain focus when control labels change

**Issue/root cause →** theme activation replaced element ID `Dark mode` with
`Light mode`, dropping the focused target. Native Tab/shortcuts then stopped.
The shared button helper derived identity from display copy; discovery/busy,
compact, view-options and help controls repeated the same pattern.

**Change →** add explicit-ID support to the existing button helper, preserving
all existing styling and the static-label API. Give dynamic-label controls stable
semantic IDs; mount IDs use device paths, template help IDs use their field key.
Reconcile/cancel share a stable action slot. No focus-to-root workaround, visual
restyle, new dependency or change to destructive confirmation focus. Spec/design
record the distinction between retained and actually removed controls.

**Before/after →** unit regression failed with `Some(Name("Light mode"))` versus
`Some(Name("Dark mode"))`; now IDs match and remain nonempty/distinct from other
controls, while static-button compatibility is pinned. Native Enter→Space→Enter
on theme toggled true→false→true without losing focus; Tab then reached Settings.
Typed `n` in preset name and pressed Ctrl+D: Settings/input remained, no demo
child appeared. Refresh→Tab navigation and template-help Enter/Space
expand/collapse also retain visible focus. Other migrated controls have shared
helper/source coverage, not claimed native hardware/workflow coverage.

**Validation →** **193 tests / 15 suites**, full locked/offline check ladder and
release build pass. Two bounded native confirmation batches; artifacts
`17-stable-theme-settings.png` through `20-help-collapsed.png` and
`theme-focus.json` under `/tmp/captureport-native-c0f0be1e76dc44ca85d5971e6d4730df/`.
All processes stopped, focus restored, clipboard untouched. No performance claim.

**Decision → ACCEPT**; committed as **`41f0d25`**
(`fix: preserve keyboard focus across dynamic button labels`).
Next: resume downstream native filesystem workflow coverage. If real GTK picker
keyboard automation remains blocked, use a private mocked picker broker seam,
explicitly separating that mock from real native scan/preview/import behavior.

## Iteration 8 — native filesystem workflows; hand off disappearing-button focus

**Test seam →** private D-Bus FileChooser broker returns only directories inside
an owned fixture root. No real desktop portal state is involved. Generated JPEG,
MP4 and a RAW placeholder exercise the real native app, FilesystemSource, codecs,
planner, copier and disk SQLite. Cancellation uses a 1 GiB sparse-padded MP4
(logical test payload, not representative camera footage or disk throughput).

**Issue/before →** cancellation succeeded, but its focused Cancel button vanished
and subsequent Ctrl+O never reached the broker. Pre-fix fixture
`/tmp/captureport-files-a9a0025a8f9c4855bd795f8f72b4b1aa/` records Cancelled,
no final/partial, only three broker requests, then a source-change timeout.

**Change →** accepted preview/import and cancellation actions hand focus to the
existing browser root before their triggers disappear. Two focus calls, using
the existing gallery/detail handoff pattern; no engine changes, new dependencies
or shipping test/source CLI. General asynchronous removals still need review.

**After / empirical regression →** one complete native run asserts:
- 3 Standard-verified copies, matching the displayed paths and source bytes;
- re-import: 3 Strict-verified existing copies, unchanged destination contents;
- 1 GiB import canceled: no final large file, no partial left;
- next Ctrl+O works (fourth broker request), then a late collision records one
  failed copy / partial session, preserving the unrelated file;
- SHA-256 of every original remains identical; prior successful copies intact.

**Evidence →** `/tmp/captureport-files-729da4c950414b9d91b8a18837c6ede0/`:
`results.json`, original hashes, SQLite, screenshots 01–06, and frozen
`workflow.py` / `broker.py`. Working helpers are `/tmp/captureport-files-workflow.py`
and `/tmp/captureport-filechooser-broker.py`. They are compositor-specific local
checks, not shipped scripts or Cargo tests. All app/broker/bus processes stopped;
focus restored; clipboard untouched. Initial harness assumptions about paired
JPEG case and Cancel's Tab position were corrected, not called product bugs.
A missed cancellation was counted as Completed, not passed; its known 1 GiB
fixture destination was later removed to reclaim RAM, with SQL/logs retained.

**Validation / limits →** full ladder **193 tests / 15 suites** and release build
pass; no new Rust test count is claimed for these native assertions. Actual GTK
folder selection, genuine RAW decoding, physical cameras, power-loss durability
and remaining UI surfaces are not covered. No performance gain claimed.

**Decision → ACCEPT**; committed as **`415fa47`**
(`fix: hand off focus before import controls disappear`).
Next: zero-success deletion affordance/outcome clarity, then history/recovery/
details/preset coverage and the still-uninvestigated measured performance paths.

## Reflection checkpoint (iteration 13)

**Progress →** 17 commits, 179→199 tests. Data-safety: sampled-identity false
success, unsafe leaves, mid-verification replacement, port-based camera identity,
and a vacuous deletion policy are fixed with red-before-fix regressions. Keyboard:
missing key context, label-based focus loss, disappearing-control focus, partial
arm→confirm focus. Native-only findings: demo catalog contamination, misleading
deletion state, unreadable history times/counts, duplicate recovery partials,
hidden recovery buttons, dead resume control.

**Working →** disposable fixtures plus a private FileChooser broker unlock real
imports without touching user data; OCR assertions catch layout/text defects that
unit tests cannot; every change ships with the full ladder and, where relevant, a
native rerun. Rejected work (adaptive poll) is recorded instead of kept.

**Weak spots →** tab-count navigation is brittle on long pages; the Settings
cyclers were fixed but not proven natively within the bounded 26 stops, so that
claim stays "source only". Genuine GTK selection, physical camera, real RAW decode,
IME round-trips and power-loss recovery remain untested. Idle CPU (~3.4% of one
core) is measured but unexplained.

**Adjustment →** stop blind Tab loops; before the next UI batch, add a
page-scroll/focus check that confirms the focused control is visible, and prefer
short, deterministic pages (or a debug-only focus report) over counting stops.
Keep spending most remaining effort where native evidence produced defects.

**Next →** (a) focused-control visibility/scroll-into-view on long Settings pages;
(b) preset rename/overwrite/delete and session-detail interactions natively;
(c) remaining click-only controls (tiles, gallery headings, discovered sources);
(d) measured catalog request cost; (e) final full ladder and release build with an
updated limitations section.

## Iteration 13 — populated recovery: duplicate partials, hidden controls, dead resume

**Fixture →** after a real import, the app is stopped and the disposable catalog
receives an `in_progress` session; one owned
`.captureport-a.JPG.<uuid>.partial` is planted in the destination. Nothing outside
that fixture directory is touched. Photo and video roots are the same folder on
purpose — a common single-library setup.

**Defects found natively →**
1. The same partial was listed **twice** with `2 incomplete file(s)` for one file:
   `inspect_partials` walked the photo root and the identical video root.
2. The interrupted-session row still printed `2026-11-01T10:00:00+00:00`.
3. With a long path, the row's path text took the whole width and pushed
   **Keep file / Confirm delete** out of the window — the destructive confirmation
   was invisible, while the status bar asked the user to confirm it.
4. `Review remaining files` was offered with no source connected; pressing it
   could only produce “Reconnect the source…”.

**Change →** `inspect_partials` canonicalises and remembers visited directories so
overlapping or repeated roots cannot duplicate entries (unit test with repeated
and nested roots); the recovery row reuses `format_session_time`; the path cell
is a truncating `flex_1 min_w_0` column so the buttons keep their space; the resume
button appears only when the interrupted session belongs to the connected,
scanned source, otherwise the row explains why. Requesting recovery without that
precondition now uses a translated message. Ten catalogs updated.

**Keyboard →** arming a partial now moves focus to **Confirm delete**
(`danger_button_with_focus`), so a second Enter confirms instead of losing the
position when the arming button is replaced; confirming returns focus to the
browser root. No engine or catalog-schema change.

**Native result →** `/tmp/captureport-recovery-4a17d3f3ea34a2690b2188686276043/`:
OCR asserts `1 unfinished session(s) · 1 incomplete file(s)`, the session started
`2026-11-01 05:00`, the partial listed once, the explanation instead of a dead
button, and visible Keep/Confirm. A single Enter removed the partial; afterwards
`0 incomplete file(s)`, the removal message is shown, the imported `a.JPG` is
byte-identical, no partial remains and both sessions are still recorded.

**Validation →** workspace **199 tests / 15 suites**, formatting, locked/offline
check, strict all-target Clippy and diff checks pass; release build passes.
Committed as `692ac20` (dedupe, timestamp, reachable row) and `6607a00`
(resume gating, focus handoff, catalogs). Camera interruption, resuming an
actually reconnected source, and power-loss behaviour remain unverified.

## Iteration 16 — paging reverted after the shortcut failed in the real window

**Correction of iteration 15 →** the earlier conclusion ("compositor key
injection did not reach the app") was wrong. With a pid-based window lookup and
`RUST_LOG` instrumentation, pressing Page Down through
`hl.dsp.send_shortcut({key="Next"})` fires the action: the handler logs
`up=false` with a real viewport (**367 px**), a real maximum offset
(**3045 px**) and offset **0**. Synthetic keys and the keybinding work.

**Real failure →** neither `ScrollHandle::set_offset` nor
`ScrollHandle::scroll_to_item` changed the page. Every later press still reported
offset `0px`, so the write never reached the painted scroller. Shipping a
shortcut that silently does nothing is worse than no shortcut.

**Decision → REJECT / REVERT** `53f230e` in `81ce1f1`; the spec paragraph is
removed in this entry. The clamp helper and its test went with the revert, so the
suite is back to **199 tests / 15 suites**. Remaining hypothesis: the tracked
handle is not the element that actually scrolls the Settings column (or this GPUI
version needs the scroll offset set during layout, not from an action).

**Better next step →** instead of paging, make focus visible: attach a focus
handle per Settings section and call `scroll_to_item(section_index)` from
`on_focus_in`, then prove it with Tab alone (which is known to work) and OCR —
no synthetic function keys required.

## Iteration 15 — keyboard paging for long pages

**Gap →** Settings is a single tall column; tabbing to a control below the fold
leaves the focus ring off-screen, and GPUI 0.2 only scrolls when the app calls
`ScrollHandle::scroll_to_item`, so nothing brought it back. Keyboard users had no
way to scroll at all: the compositor wheel and pointer are the only scrollers.

**Change →** PageUp/PageDown actions (guarded by `!CapturePortTextInput`) move the
current long page — Settings, History, Recovery — by 80% of the viewport, clamped
to the scrollable range by a small tested helper. Browser/Preview grids keep
their own scrolling. Spec updated. No layout or color change.

**Validation →** unit test covers down/up, clamping at both ends, a zero maximum
offset and a zero step: **200 tests / 15 suites**, formatting, locked/offline
check, strict all-target Clippy, release build.

**Not proven natively →** injecting Page Down through the compositor did not
reach the app. `hl.dsp.send_shortcut` accepted `Page_Down`/`PageDown`/`Next`
without error but the page did not move, and my probe captured the wrong window
(grim region from a stale client lookup), so no native claim is made. This
continues the earlier finding that guessed key names/stops are unreliable; a
honest native check needs the app to expose focus/scroll state or a corrected
key-injection path.

**Decision → ACCEPT** the unit-tested paging behaviour; **REJECT** any claim that
it resolves the off-screen-focus problem until it is verified in a real window.
Committed as `53f230e`.

## Iteration 14 — preset actions return keyboard focus to the name field

**Issue →** after **Create preset** succeeded, focus stayed on the button, so a
second keyboard action was unpredictable: Shift+Tab landed on the photo-root
field, not the preset-name field, and the rename attempt silently did nothing.
The same applied to Rename, Overwrite and Delete, whose inline confirmations
change the controls around the focused button.

**Change →** `manage_preset` takes the window and focuses the preset-name field
before dispatching any preset action. Five listeners updated. Focus now returns
to a known place after every preset action, so the next Tab is deterministic.
No behaviour, copy, persisted data or layout changed.

**Native loop →** `/tmp/captureport-presets-e22de434409b42daad10012726b9c934/`
(`results.json`, screenshots): 7 Tabs → Settings; 3 Tabs → name field; typed
`alpha`; one Tab + Enter created it (DB `presets=alpha`, message `Created alpha.`).
The field regained focus, so Ctrl+A + `beta` replaced the text; Tab,Tab + Enter
renamed it (DB `presets=beta`, message `Renamed preset to beta.`).

**Not proven →** Overwrite and Delete. After renaming, a bounded probe (Tab×4,
Enter, Enter) never produced the inline confirmation, so neither is claimed
keyboard-usable. Either the tab order past the field differs from the visual
order or arming needs a different stop; that needs a focused-control readout
rather than more guessed Tab counts.

**Validation →** workspace **199 tests / 15 suites**, formatting, locked/offline
check, strict all-target Clippy and diff checks pass; committed as `0c2c742`.
Pressed keys only affected the isolated fixture; app stopped, focus restored.

## Iteration 13b — keyboard-reachable history rows and settings cyclers

**Issue →** auditing every `on_click` without `tab_index` found controls whose only
interaction is a pointer: History session rows (no keyboard way to inspect a
session) and the Settings cyclers for Verification, Bundles, Grouping, Collision
and Backup-required (no keyboard way to change them). This contradicts the
spec's "Tab/Shift+Tab move focus on every page, and Enter/Space activate the
focused control".

**Change →** six click targets get `tab_index(0)` plus an accent focus border,
using the existing patterns; no behaviour, copy or layout change.

**Native evidence →** `/tmp/captureport-keys-7454a153e7e641909b90637c4bcbd8eb/`:
after Tab+Enter on a History row the row shows a focus border and expands to
`Session 1 · 1 copy/copies` with the destination, `Verified` and
`Verification: Standard`. The session list heading also now reads
`1 session(s)`.

**Not proven →** the Settings cyclers are fixed in source only. A bounded probe
(Tab+Enter up to 26 stops, OCR after each) never reached the Verification cycler;
repeated activation of **Create preset** kept producing its validation message,
so the settings page needs a focused-control visibility/scroll check before that
claim is upgraded. Tab-count navigation on long pages is too brittle to call a
pass.

**Validation →** workspace **199 tests / 15 suites**, formatting, locked/offline
check, strict all-target Clippy and diff checks pass; committed as `0d494bd`.
All native processes stopped, original focus restored, clipboard untouched.

## Iteration 12 — history/recovery/settings natively; readable session rows

**Scope →** extend the private-broker native harness to the pages that had no
runtime coverage: Import history, Import recovery, and Settings presets.
Navigation is keyboard-driven after a restart (no transient "Review import" row),
which makes the sidebar order stable, and pages are asserted with `tesseract`
OCR of the captured window, not by eyeballing.

**Covered natively →** a real two-file import; **Import history · 1 session(s)**
with the persisted row (`2026-11-01 12:25 · Source 1 · Completed · 2 imported ·
3.5 KB`); **Import recovery** empty state (`0 unfinished session(s) · 0 incomplete
file(s)`); Settings shows the new `loop preset` chip after creation; after a full
app restart the catalog still lists it (`presets_after_restart`) and History still
shows the completed session. Destination files remain exactly the two imported
items. The picker is still the only mocked seam.

**Defects found by looking at the result →**
1. History printed the stored RFC 3339 stamp with nanoseconds
   (`2026-11-01T17:24:06.125473218+00:00`), unlike every other time in the app.
2. Counts were ungrammatical for one item: `1 sessions`, `1 copies`.
3. Session status was rendered with Rust `{:?}` even though the catalogs already
   translate `Completed`/`Partial`/`Cancelled`/`Failed`.

**Change →** `format_session_time` parses the stored timestamp to local
`YYYY-MM-DD HH:MM` and falls back to the stored string rather than hiding it;
session status goes through the translation catalog; the three affected strings
use the app's existing `(s)`/`copy/copies` convention in all ten catalogs. Only
presentation changed — no stored value, query or schema touched.

**Validation →** new unit test pins the displayed shape (length, separators, no
`T`/offset/nanos) and the unparsable passthrough; full ladder **198 tests / 15
suites** plus formatting, locked/offline check, strict Clippy, diff checks and
release build. Native rerun artifacts
`/tmp/captureport-pages-7d1a07c12e0d4a79a1135a5af39851e1/` (`results.json`,
screenshots, OCR-asserted headings); helper `/tmp/captureport-pages-workflow.py`.
All app/broker/bus processes were stopped, original focus restored, clipboard
untouched.

**Not covered →** session-detail expansion, rename/overwrite/delete presets,
populated recovery (needs injected incomplete session/partial fixture), the media
details popup, and inline template-token keyboard access. Physical camera, real
RAW decode, genuine GTK selection and power-loss recovery remain limits.

**Decision → ACCEPT**; committed as **`e9db3e8`**
(`fix: show session history times, counts and status in human form`).

## Iteration 11 — idle-CPU hypothesis tested and rejected

**Question →** the window polls worker channels every 16 ms (62.5 Hz). The
earlier single sample said 3.67% of one core, which looked like polling overhead.

**Method →** release binary, isolated XDG, empty window focused on a 1180×760
floating window, three seconds of settle time, then 15 samples of 10 s each
(three separate launches). CPU from `/proc/<pid>/stat` ticks, RSS/threads and
voluntary+nonvoluntary context switches from `/proc/<pid>/status`.

**Results →**

| Variant | CPU, % of one core (median, range) | Context switches/s |
|---|---:|---:|
| 16 ms poll (baseline, n=5) | 3.4 (3.3–3.7) | 233 |
| adaptive 16/100 ms (n=5) | 3.5 (1.9–4.1) | 208 |
| 16/1000 ms (n=15) | 3.4 (2.8–4.6) | 152 |

RSS stayed at **258–270 MiB** with **35 threads** in every variant. Slowing the
poll 60× removed the expected wakeups (233→152/s) but **not** the CPU, so the
drain loop is not the dominant idle cost. A hide-to-special-workspace probe was
inconclusive (3.87% visible vs 6.37% hidden) and was not used as evidence.

**Decision → REJECT** the adaptive-poll change: it added a method and two
constants for no measured CPU benefit. The patch was reverted; the working tree
contains only this evidence. The honest remaining hypothesis is GPUI/Wayland
frame scheduling for a visible idle window, which needs a paint/frame-callback
instrument before any code change.

**Baseline now recorded →** steady-state idle is ~3.4% of one core and ~260 MiB
RSS for an empty window on this host. This is descriptive, not a regression
claim: no earlier comparable repeated measurement exists. Raw probes:
`/tmp/idle-after.txt`, `/tmp/idle-1000.txt`, `/tmp/captureport-idle-9a28e45d552c4245be98243524527752/idle.json`,
helper `/tmp/captureport-idle-probe.py`. Each launch was stopped after verifying
its isolated `XDG_CONFIG_HOME`; original focus restored.

**Validation →** after revert: formatting, locked/offline check, **197 tests / 15
suites**, strict all-target Clippy, diff checks and a release rebuild all pass.
No behaviour changed, so the native workflow suite was not rerun for this entry.

## Iteration 10 — Standard verification stops hashing what it never compares

**Issue →** `verify_copy(Standard)` streamed and BLAKE3-hashed the entire
destination, then compared only the edge quick fingerprint. For a 20 GB video it
re-read 20 GB to reach a decision that reads 512 KiB.

**Measurement before →** release build, tmpfs/page-cache fixture, 512 MiB file,
five samples after the digest was prepared: **162–163 ms** per Standard verify.
This is the best case for the old path; real SSDs and the end of the page cache
would make it slower. The probe was ad hoc and has been removed; it was never
claimed as disk throughput.

**Change →** one private `quick_from_file` helper reads the same edge samples
`quick_path` reads (whole file only when ≤ 512 KiB) from the already-open,
non-symlink handle; Standard uses it and reports one completion. Strict keeps the
streaming full hash. Size, symlink/FIFO refusal and the post-read identity/version
timestamp revalidation are unchanged, so the documented Standard guarantee
(size + edge fingerprint) is now what the code actually computes. A repeatable
`verify_standard_8mib` / `verify_strict_8mib` pair was added to the synthetic
benchmark, and the docs now say the two mean the same thing while reading
different byte counts.

**Measurement after →** 512 MiB, same fixture: Standard **218–220 µs** warm
(strict unchanged at **152–153 ms**), i.e. roughly 700× less work; the new
repeatable 8 MiB probes give Standard **0.273 ms** (0.256–0.282) versus Strict
**2.353 ms** (2.178–2.463) over five runs. Two tests pin the behaviour: the open
handle fingerprint equals the path fingerprint at 0/1/512 KiB/1 MiB boundaries,
and Standard reports one completion while Strict still reads every chunk.

**Correctness boundary →** detection is exactly as strong as before: the old code
also ignored middle bytes when comparing. A rewrite of already-read bytes during
Standard verification is now caught by the same final metadata/mtime/ctime
revalidation over a shorter window. Standard is still the wrong mode for
middle-byte corruption; Strict remains the documented remedy, and pre-existing
destination candidates still force Strict.

**Validation →** workspace **197 tests / 15 suites**, formatting, locked/offline
check, strict all-target Clippy and diff checks pass; release build passes. The
full native filesystem workflow was rerun to completion
(`/tmp/captureport-files-a32c84aff15646699f21e2b5e62e0ad6`): 3 Standard copies,
3 Strict re-verifications, cancel, late collision and unchanged originals. Faster
verification made the cancellation keystrokes race the 1 GiB copy, so the harness
now sends them without inter-key delays — a fixture-tuning note, not a product
fix. Camera transfers, real RAW decode, genuine GTK selection and physical
disks remain unverified.

**Decision → ACCEPT**; committed as **`6935dfa`**
(`perf: verify Standard copies from their edge samples only`).

## Iteration 9 — deletion entry mirrors the true verified-copy policy

**Issue →** the native run offered **Delete verified originals…** after a
cancelled 1 GiB import and after a zero-success collision, while the footer said
0 verified. The entry was gated on `last_import_result.is_some()` while the scan
itself used a stricter predicate. Separately, a plan with no required copy (or
only optional copies) passed the "all required copies completed" test vacuously.

**Change →** one exported policy (`verified_copy_requirements`) now decides the
preview entry, confirmation scope, `delete_sources` arming and backend refusal;
it rejects a missing/other-media result and plans without a required destination,
and indexes results by media ID instead of rescanning per item. The preview header
switches to verified/failed counts after an import, and to a distinct cancelled
wording so a cancellation is not misreported as failure. Spec, ten catalogs and
the deletion regression matrix are updated. No engine, planner or CLI changes.

**Before/after →** new regressions fail before the change (empty/optional-only
plans were deletable) and pass now; the shared-policy test also rejects a result
belonging to another media ID, and 1000-file unfavourable ordering is exercised.
Native run `/tmp/captureport-files-e7a254c0c95e4e509d84c530af5e2812/`: after a
3-file import the header reads **Import finished · 3 verified · 0 failed** with the
deletion action present; after cancellation and after the late collision it reads
**0 verified** and the deletion action is absent; all originals unchanged and the
source/destination/verification/collision assertions still pass.

**Measured (narrow) →** release-mode test binary, 1000 planned items against a
reversed result, five samples after a discarded warm-up: indexed shared policy
**47–48 µs** versus the previous per-item scan **113–118 µs**. This is a microbench
of scope computation only (which now runs on the preview render path), not import
throughput, and it is not comparable with the earlier synthetic suite.

**Validation →** workspace **195 tests / 15 suites**, locked/offline check, strict
all-target Clippy, formatting, diff checks and release build pass. Native picker
remains the private broker mock; app/filesystem/importer/SQLite remain real.
Cancellation-as-completion, camera deletion, genuine GTK selection and power-loss
recovery are still unverified. No claim that every dynamic control's focus or
all destructive surfaces were exercised.

**Decision → ACCEPT**; committed as **`74b3213`**
(`fix: gate source deletion on the verified-copy policy`).

## Stopping/remaining limits

Not close to the stopping criteria: major native workflows, real-data-shape
performance probes and high-risk candidates above remain. No physical camera
transfers, compositor correctness or native IME/clipboard round-trips have been
validated by the automated baseline. Each future iteration must refresh this
ledger, record its own evidence/decision/commit, and continue with justified work.
