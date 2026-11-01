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

Host: Linux 7.3.0-4-cachyos x86_64, i5-13600K (20 logical CPUs), btrfs.
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
| Sources, refresh, mount, disconnect/reconnect | devices, browser/sources, gphoto/discovery | S/T/G: folder picker opens/cancels; selection automation blocked; camera separate |
| Grid/filter/sort/selection and gallery grouping/rename | browser/views + gallery + session | S/T/G: populated demo grid at 1180×900 / 720×640; interactions/latency pending |
| RAW+JPEG bundles and member selection | ingest/bundle, browser/bundles + views | S/T; G pending |
| Details popup, original URL copy, dismissal/focus | browser/detail | S/T; G pending |
| Image/video previews, unavailable fallback, progress strip | metadata/thumbnail, browser/thumbnails + render | S/T/G: synthetic preview decode + progress; real mixed decode/fallback G pending |
| Plan preview, collisions, backup paths, confirmation | ingest/planner, browser/import + panels | S/T; G pending |
| Import phases, cancel, outcomes | ingest/importer, tasks, browser/render | S/T; real isolated normal/re-import/cancel/error G pending |
| History and session-detail inspection | catalog, browser/import + panels | S/T; G pending |
| Recovery, resume, partial cleanup confirmation | ingest/recovery, browser/import + panels | S/T; crash/fault and G pending |
| Verified-original deletion confirmation/cancel/refusal | ingest/source_delete, tasks, browser/import | S/T; disposable fixture only; G pending |
| Presets: create/load/rename/overwrite/delete | saved_presets, browser/settings + panels | S/T; persistence/restart G pending |
| Destinations, filename/folder templates and segment help | template_help, browser/panels, render | S/T; G pending |
| Media rules/folder exclusions, clock/timezone/session gap | browser/settings + panels, ingest/preset | S/T; DST/edge/G pending |
| Backup, source alias, apply settings | browser/settings + panels | S/T; failure/persistence/G pending |
| Theme/language/thumbnail size and maintenance actions | i18n, browser/settings + panels | S/T/G: light/dark + restart persistence + compact; other actions pending |
| All 18 text fields, edit menu, keyboard/focus/scroll | text_edit, text_input, widgets | S/T; G pending; native IME/clipboard limits explicit |
| All shared controls/tooltips/disabled/hover/focus states | widgets, render, browser modules | S/G: Tab/Enter Settings navigation; theme focus-loss confirmed; full sweep pending |

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
5. **Medium / measured opportunity:** Standard verification reads/hashes the whole
   destination although its contract compares edge samples. Benchmark identical
   fixtures, preserve Strict and full existing-file verification, then decide.
6. **Medium / measured opportunity:** catalog request sequence dominates the
   synthetic probe (~394 ms). Profile actual scan/import callers before batching
   or changing transaction semantics; memory DB is not durable import throughput.
7. **Medium / investigate:** progressive gallery rebuilding/name lookup and frame
   allocations, thumbnail cache/queue invalidation, obsolete work and decoder
   subprocess lifecycle. Measure real mixed-media cases.
8. **Medium / investigate:** startup/idle CPU and the 16 ms drain, UI blocking I/O,
   keyboard/grid navigation, resize, settings feedback and unsafe re-entrancy.
   `docs/todo.md` contains an older critique: verify, do not assume still current.
9. **High / native confirmed:** Ctrl+D in a normal run writes demo media into
   the ordinary disk catalog (isolated test observed 7,500 media / one Fake source).
   CLI `--demo` isolation does not cover this action. Fix before more demo-driven
   workflows; no production catalog was used in the reproduction.
10. **High / native confirmed:** keyboard navigation stops after activating the
   theme button. Fresh Tab×7/Enter opens Settings; after Tab/Enter theme toggle,
   the same sequence and shortcuts stop responding, even with the root-context
   fix. Investigate the label-dependent control ID / removed focus target.
11. **Measurement gap:** correct quick-hash byte accounting, add realistic
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
and fresh release build pass; native shortcut positive path verified. Commit SHA
will be recorded immediately after commit. The native coverage checklist remains
open; finding blockers is not the same as completing its workflows.

## Stopping/remaining limits

Not close to the stopping criteria: major native workflows, real-data-shape
performance probes and high-risk candidates above remain. No physical camera
transfers, compositor correctness or native IME/clipboard round-trips have been
validated by the automated baseline. Each future iteration must refresh this
ledger, record its own evidence/decision/commit, and continue with justified work.
