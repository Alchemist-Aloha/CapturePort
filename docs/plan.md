# CapturePort Implementation Plan

## 1. Development Strategy

Build CapturePort as a sequence of vertical slices.

Do not begin by implementing every backend subsystem independently and only later connect them to GPUI. Instead, each milestone should produce a runnable application demonstrating another part of the final ingest workflow.

The development progression should roughly be:

```text
Launch
  ↓
Browse source
  ↓
Select media
  ↓
Import files
  ↓
Remember imports
  ↓
Extract metadata
  ↓
Generate thumbnails
  ↓
Rename / organize
  ↓
Verify import
  ↓
Camera support
  ↓
Recovery / history
  ↓
Advanced ingest rules
```

The architecture should nevertheless maintain strict separation between:

```text
GPUI presentation
        ↓
application/controller layer
        ↓
domain/core
        ↓
source/catalog/planner/importer adapters
```

The core importer must never depend on GPUI.

---

# 2. Initial Technical Decisions

Lock down these decisions before significant implementation.

## Language and UI

```text
Rust
GPUI
Linux-first
```

GPUI owns:

- windows;
- views;
- commands;
- interaction;
- visual state;
- notifications.

It must not directly perform:

- SQLite operations;
- hashing;
- metadata extraction;
- file copying;
- libgphoto2 calls;
- thumbnail decoding.

---

## Database

Use:

```text
SQLite
rusqlite
```

Prefer bundled SQLite for predictable packaging.

Use one dedicated catalog worker that owns the SQLite connection.

Do not allow arbitrary application components to directly create database connections.

Architecture:

```text
UI / controllers
        ↓
CatalogHandle
        ↓ channel
CatalogWorker
        ↓
rusqlite::Connection
```

This removes a large class of SQLite locking/threading problems.

Suggested SQLite configuration:

```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
```

Use explicit transactions for import commits and migrations.

---

## Hashing

Use BLAKE3 for:

- quick fingerprints;
- optional full-file hashes;
- copy verification.

Define hashing behind:

```rust
trait Fingerprinter
```

rather than spreading BLAKE3 calls throughout the codebase.

---

## Camera interface

Use libgphoto2 through the `gphoto2` crate initially.

However, isolate it completely:

```text
source-gphoto/
```

The rest of CapturePort must know nothing about gphoto2 types.

This is important because camera bindings have a much smaller maintenance ecosystem than SQLite or GPUI.

---

## Metadata

Use adapters.

Likely initial components:

```text
kamadak-exif
    JPEG/TIFF/HEIF/etc. EXIF

rawler
    RAW metadata / embedded preview support

ffmpeg-next
    video metadata / video thumbnail support
```

Do not expose any of their types outside `metadata` or `thumbnail`.

---

# 3. Repository Structure

Start as a Cargo workspace, but avoid excessive micro-crates.

Recommended initial structure:

```text
captureport/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE
│
├── crates/
│   ├── captureport/
│   │   └── GPUI executable
│   │
│   ├── captureport-core/
│   │   ├── media.rs
│   │   ├── source.rs
│   │   ├── import.rs
│   │   ├── metadata.rs
│   │   ├── events.rs
│   │   └── errors.rs
│   │
│   ├── captureport-catalog/
│   │   ├── database.rs
│   │   ├── migrations.rs
│   │   ├── repository.rs
│   │   └── worker.rs
│   │
│   ├── captureport-ingest/
│   │   ├── planner.rs
│   │   ├── importer.rs
│   │   ├── verify.rs
│   │   ├── fingerprint.rs
│   │   └── recovery.rs
│   │
│   └── captureport-media/
│       ├── filesystem.rs
│       ├── gphoto.rs
│       ├── metadata.rs
│       └── thumbnail.rs
│
├── tests/
│   └── fixtures/
│
└── docs/
    ├── SPEC.md
    ├── ARCHITECTURE.md
    └── DATABASE.md
```

Four/five crates are enough initially.

Do not create one crate for every subsystem until compilation boundaries actually become useful.

---

# 4. Core Domain Types

Implement domain types before real device access.

Essential types:

```rust
SourceId
MediaId
ImportSessionId
PresetId

SourceIdentity
MediaItem
MediaMetadata
MediaBundle

ImportStatus
ImportPlan
PlannedImport
ImportSession

Destination
VerificationMode
```

Example conceptual `MediaItem`:

```rust
pub struct MediaItem {
    pub id: MediaId,

    pub source_id: SourceId,
    pub source_path: String,
    pub source_name: String,

    pub size: u64,
    pub media_type: MediaType,

    pub metadata: MetadataState,

    pub import_status: ImportStatus,

    pub bundle_id: Option<BundleId>,
}
```

Use explicit state enums instead of boolean fields.

Bad:

```rust
is_imported: bool
is_duplicate: bool
metadata_loaded: bool
```

Better:

```rust
enum ImportStatus {
    Checking,
    New,
    Imported(PreviousImport),
    PossibleDuplicate,
    Unknown,
}
```

Likewise:

```rust
enum MetadataState {
    Pending,
    Ready(MediaMetadata),
    Failed(MetadataError),
}
```

This significantly simplifies GPUI rendering.

---

# 5. MediaSource Abstraction

Define this before implementing cameras.

Conceptually:

```rust
pub trait MediaSource {
    fn identity(&self) -> SourceIdentity;

    fn enumerate(
        &self,
        sink: MediaSink,
    ) -> Result<()>;

    fn open(
        &self,
        item: &MediaLocator,
    ) -> Result<Box<dyn Read + Send>>;

    fn read_range(
        &self,
        item: &MediaLocator,
        offset: u64,
        length: usize,
    ) -> Result<Vec<u8>>;
}
```

Enumeration should be streaming/event-driven rather than:

```rust
fn enumerate() -> Vec<MediaItem>
```

because a source could contain tens of thousands of files.

The UI should see:

```text
MediaDiscovered
MediaDiscovered
MediaDiscovered
...
```

while scanning continues.

---

# 6. Milestone 0 — Project Skeleton

## Goal

Produce a clean GPUI application that compiles, launches, and has the final architectural boundaries.

Implement:

- Cargo workspace;
- logging;
- error infrastructure;
- config paths;
- GPUI main window;
- application commands;
- basic sidebar/content/footer layout;
- placeholder source list;
- placeholder media grid;
- status bar.

Initial UI:

```text
┌─────────────────────────────────────────────────────┐
│ CapturePort                                         │
├──────────────┬──────────────────────────────────────┤
│ Sources      │                                      │
│              │      No source selected              │
│              │                                      │
├──────────────┴──────────────────────────────────────┤
│ Ready                                               │
└─────────────────────────────────────────────────────┘
```

Create application directories according to XDG conventions.

For example:

```text
$XDG_CONFIG_HOME/captureport/
$XDG_DATA_HOME/captureport/
$XDG_CACHE_HOME/captureport/
```

Do not put cache thumbnails in the data directory.

### Tests

Verify:

- application launches;
- config/data/cache paths resolve;
- GPUI commands execute;
- logging initializes exactly once.

### Definition of Done

`cargo run` opens CapturePort with no warnings, no blocking work, and clean shutdown.

---

# 7. Milestone 1 — Fake Source + Application State

Before touching real files, implement a fake source.

Generate objects such as:

```text
DSC00001.ARW
DSC00001.JPG
DSC00002.ARW
C00001.MP4
...
```

Create 10,000+ synthetic entries to test state handling.

Implement application events:

```rust
enum AppEvent {
    SourceDetected(...),
    SourceRemoved(...),

    MediaDiscovered(...),
    MetadataReady(...),
    ThumbnailReady(...),
    ImportStatusChanged(...),

    ImportProgress(...),
    ImportCompleted(...),
    ImportFailed(...),
}
```

GPUI should render incremental events.

### Important Architectural Rule

GPUI components should render state.

They should not contain domain logic such as:

```text
"if filename ends in ARW then..."
```

That belongs in core/media.

### Tests

Simulate:

- 1 item;
- 100 items;
- 10,000 items;
- source removal midway;
- metadata failures;
- duplicate events.

### Definition of Done

CapturePort can smoothly display a large fake media source without real filesystem access.

---

# 8. Milestone 2 — Filesystem Source

Implement the first real `MediaSource`.

Support:

```text
directory
SD card mount
USB mass-storage camera
arbitrary mounted volume
```

Initial source selection can simply use a folder picker.

Do not implement auto-device discovery yet.

### Enumeration

Default rules:

- recursively inspect likely media paths;
- recognize standard camera extensions;
- ignore hidden/system directories unless requested;
- preserve original relative paths.

Detect:

```text
RAW
JPEG
HEIF
PNG
TIFF
MP4
MOV
MTS/M2TS
sidecars
unknown
```

Extension detection is only initial classification; metadata should later refine it.

### Cancellation

Enumeration must accept cancellation.

When source changes:

```text
old scan → cancel
new scan → begin
```

Do not allow stale scan events to populate the new source.

Use a generation/session ID:

```rust
ScanGeneration(u64)
```

and discard events belonging to old generations.

### Definition of Done

The user can select an SD-card-like directory and see its media files incrementally.

---

# 9. Milestone 3 — Basic Selection UX

Implement the core media browser.

Features:

- grid view;
- compact list option later;
- select/deselect;
- Ctrl+A;
- select all new;
- filter photos/videos;
- sort capture time/name initially;
- status footer.

Footer:

```text
438 items · 414 selected · 36.2 GB
```

At this stage, use placeholder thumbnails.

Implement virtualization before real thumbnails if GPUI does not already give suitable behavior.

Do not instantiate expensive view state for thousands of offscreen items.

### Definition of Done

Selecting thousands of items remains responsive.

---

# 10. Milestone 4 — Catalog and Database

Now implement SQLite.

Migration 001 should create:

```sql
sources
media
imports
import_sessions
presets
schema_version
```

Keep the first schema conservative.

Do not add every conceivable future column.

---

## Catalog Worker

Create:

```rust
CatalogHandle
CatalogCommand
CatalogResponse
```

Example:

```rust
enum CatalogCommand {
    LookupMedia(Vec<MediaIdentity>),
    BeginSession(...),
    CommitImport(...),
    CompleteSession(...),
    LoadHistory(...),
}
```

One worker owns:

```rust
rusqlite::Connection
```

The rest of the application communicates through messages.

### Import Matching v1

Initially match:

```text
source identity
relative path
filename
size
```

plus timestamp where available.

Do not introduce hashes yet.

### UI

Previously imported files become:

```text
✓ Imported
```

and are deselected by default.

Provide filters:

```text
[x] New
[ ] Imported
```

### Tests

Test:

- same source reconnect;
- renamed destination;
- database restart;
- source with repeated filename;
- two cameras using DSC00001.JPG;
- multiple imports of same card.

### Definition of Done

Disconnecting and reconnecting a previously imported source correctly identifies earlier items without scanning destination directories.

---

# 11. Milestone 5 — Metadata Layer

Implement normalized metadata extraction.

Architecture:

```text
MediaItem
   ↓
MetadataService
   ├─ EXIF adapter
   ├─ RAW adapter
   └─ Video adapter
   ↓
MediaMetadata
```

Do not expose EXIF/rawler/FFmpeg structs elsewhere.

### Metadata priority

For capture timestamp:

```text
embedded capture timestamp
↓
container timestamp
↓
filesystem timestamp
```

Record provenance:

```rust
enum TimestampSource {
    ExifOriginal,
    QuickTime,
    Camera,
    Filesystem,
}
```

This will matter later for clock correction.

### RAW Handling

Use `rawler` initially, but place all code behind:

```rust
trait RawMetadataProvider
```

because its API may evolve.

### Definition of Done

CapturePort consistently displays:

- capture date/time;
- dimensions;
- camera model;
- media type;
- duration for video where available.

Metadata failures do not prevent import.

---

# 12. Milestone 6 — Thumbnail Pipeline

Implement thumbnails separately from metadata.

Priority queue:

```text
1. currently visible items
2. near-visible items
3. selected items
4. remaining source
```

Cache thumbnails under:

```text
$XDG_CACHE_HOME/captureport/thumbnails/
```

Cache key should incorporate enough source identity to prevent collisions.

For example:

```text
source_id
relative path
size
mtime
```

hashed into a stable cache key.

### Image hierarchy

Prefer:

```text
embedded RAW preview
↓
embedded EXIF thumbnail
↓
reduced decode
↓
placeholder
```

Never decode a 60 MP RAW simply to make a grid thumbnail if an embedded JPEG is available.

### Video

Extract one representative frame asynchronously.

### Cache

The cache must be disposable.

Provide:

```text
Clear thumbnail cache
```

eventually.

### Definition of Done

A directory containing RAW + JPEG + video progressively receives thumbnails while scrolling remains smooth.

---

# 13. Milestone 7 — Template Engine

Implement your own small template language.

Do not use general-purpose scripting.

Initial tokens:

```text
{year}
{month}
{day}
{date}
{time}
{datetime}

{camera}
{camera_model}

{original_name}
{original_stem}
{extension}

{media_type}
{sequence}
```

Support formatting:

```text
{sequence:04}
```

Possibly later:

```text
{date:%Y-%m-%d}
```

### Parser

Templates must compile into an AST once:

```rust
enum TemplatePart {
    Literal(String),
    Token(Token),
}
```

Do not repeatedly regex-replace strings for every file.

### Validation

Detect:

- unknown token;
- malformed format;
- empty filename;
- path separator where prohibited;
- reserved/invalid filename;
- traversal such as `../`.

### Preview

UI:

```text
Template
{date}_{time}_{sequence:04}.{extension}

Example
20260927_142033_0001.ARW
```

### Tests

Template tests should be exhaustive.

This is deterministic logic and ideal for unit testing.

### Definition of Done

The same metadata + preset always produces exactly the same result.

---

# 14. Milestone 8 — Import Planner

Before copying files, introduce an explicit plan.

Input:

```text
selected media
+
metadata
+
preset
+
destination state
```

Output:

```rust
ImportPlan
```

Each item contains:

```rust
PlannedImport {
    source,
    final_destination,
    temporary_destination,
    expected_size,
    status,
}
```

The planner must detect:

- duplicate generated paths;
- existing destination;
- invalid path;
- unavailable destination;
- insufficient free space;
- unsupported source.

The importer must never invent destination names independently.

Only the planner decides final paths.

This creates an important invariant:

```text
Preview == actual import result
```

### Definition of Done

CapturePort can show the exact destination of every selected item without copying anything.

---

# 15. Milestone 9 — Transactional Import Engine

Now implement actual copying.

Per file:

```text
source
  ↓
temporary destination
  ↓
copy
  ↓
flush
  ↓
verify
  ↓
atomic rename
  ↓
database commit
```

Temporary filename example:

```text
.captureport-b6d3f43a.partial
```

Do not use:

```text
finalname.ARW
```

until verification succeeds.

---

## Copy State Machine

```rust
enum ImportItemState {
    Planned,
    Queued,
    Copying,
    Verifying,
    Committing,
    Completed,
    Failed,
    Cancelled,
}
```

Make transitions explicit.

Invalid transitions should be impossible or treated as programmer errors.

---

## Progress

Report:

```text
current file bytes
overall bytes
files completed
transfer rate
```

Do not update GPUI for every tiny buffer write.

Throttle UI progress events.

Something around human-visible refresh frequency is sufficient.

---

## Cancellation

Cancellation should mean:

```text
finish/abort current operation safely
do not start further files
leave completed imports committed
remove or mark partial temporary files
```

It does not mean rolling back successfully imported media.

### Definition of Done

CapturePort can safely import a large filesystem source and survive cancellation without presenting incomplete files as complete media.

---

# 16. Milestone 10 — Verification and Fingerprinting

Implement three verification levels.

## Fast

```text
source size == destination size
```

## Standard

```text
size
+
quick source fingerprint
+
quick destination fingerprint
```

## Strict

```text
full BLAKE3(source)
==
full BLAKE3(destination)
```

Default:

```text
Standard
```

---

## Quick Fingerprint

Initial algorithm:

```text
BLAKE3(
    version marker
    file size
    first 256 KiB
    last 256 KiB
)
```

Store algorithm version.

For example:

```text
quick-blake3-v1
```

Never store an unversioned fingerprint because the algorithm may change.

For files smaller than the combined sample range, hash the entire file.

---

## Important PTP Optimization

Do not assume arbitrary range reads from every camera are efficient.

For PTP sources:

```text
metadata/history match
```

should remain the first duplicate detection mechanism.

If efficient partial reads are unavailable, defer strong hashing until the file is downloaded.

### Definition of Done

Verification can distinguish corruption from successful copies and import history stores how verification was performed.

---

# 17. Milestone 11 — Better Duplicate Detection

Add the progressive model.

### Tier 1

```text
stable source identity
+
source path
+
size
+
capture timestamp
```

### Tier 2

Quick fingerprint.

### Tier 3

Full BLAKE3 only when necessary.

Classification:

```rust
New
Imported
PossibleDuplicate
Unknown
```

Never turn weak similarity into `Imported`.

### Important Test

Simulate:

```text
camera formatted
counter restarted
DSC00001.ARW appears again
```

CapturePort must not mistakenly exclude a completely different photo.

### Definition of Done

Camera/card reuse does not make import history unsafe.

---

# 18. Milestone 12 — RAW+JPEG and Sidecar Bundling

Implement logical media grouping.

Initial heuristic:

```text
same parent path
same filename stem
compatible extension combination
```

Examples:

```text
DSC1234.ARW
DSC1234.JPG
```

and:

```text
C0001.MP4
C0001.XML
```

Model:

```rust
MediaBundle {
    id,
    primary,
    members,
    bundle_type,
}
```

UI should primarily show bundles as parent captures and let users expand them
into a tree of individually selectable source files. Group-level select-all and
deselect-all controls should coexist with member selection; preview and history
must reflect the final selected file set.

Preset policy:

```text
Keep all
RAW only
JPEG only
```

Never delete ignored bundle members from the camera.

### Definition of Done

RAW+JPEG shooters see one logical capture rather than a confusing duplicate grid.

---

# 19. Milestone 13 — Import Presets

Introduce user presets only after the pipeline is stable.

Preset:

```rust
ImportPreset {
    photo_destination,
    video_destination,

    photo_path_template,
    video_path_template,

    filename_template,

    verification,

    bundle_policy,

    grouping,

    collision_policy,
}
```

Ship defaults.

### Everyday

```text
Photos:
~/Pictures/{year}/{date}

Videos:
~/Videos/{year}/{date}

Filename:
{original_name}

Verification:
Standard
```

### Organized

```text
{date}_{time}_{sequence:04}.{extension}
```

Users should never need to learn the template language to start using CapturePort.

### Definition of Done

Switching presets updates the complete import preview immediately.

---

# 20. Milestone 14 — Date and Session Grouping

Implement:

```text
None
Day
Month
Year
Time-gap session
```

For gap grouping:

```rust
next.capture_time - previous.capture_time > threshold
```

starts a new group.

Time-gap grouping assigns session numbers without adding implicit destination
folders. Users can opt into session folders with the folder template
`{date:%Y-%m-%d}_{session}`, for example:

```text
2026-09-27_01
2026-09-27_02
```

Group creation must occur before sequence assignment so filenames are deterministic.

### Ordering

Define a canonical import sort:

```text
corrected capture time
↓
original filename
↓
relative source path
```

Never rely on filesystem enumeration order.

### Definition of Done

Repeated planning of the same source creates identical groups and filenames.

---

# 21. Milestone 15 — Camera Support

Only now integrate libgphoto2.

Implement:

```rust
GPhotoSource
```

behind `MediaSource`.

Functions:

```text
autodetect
enumerate folders
enumerate media
obtain camera identity
download file
read preview where supported
observe disconnect
```

Do not expose camera configuration/control features.

CapturePort is an importer, not a tethering application.

---

## Camera Worker

Treat gphoto2 as blocking/external I/O.

Give each active camera a dedicated worker/serialized request queue if necessary.

Do not issue uncontrolled concurrent gphoto2 calls against one camera.

Architecture:

```text
Import engine
      ↓
GPhotoSourceHandle
      ↓
GPhoto worker
      ↓
libgphoto2
```

### Failure tests

Physically and synthetically test:

```text
camera disconnected during enumeration
camera disconnected during download
camera sleeps
camera returns access error
camera mounted by another process
camera reconnects
```

### Definition of Done

A PTP camera behaves approximately like a filesystem source from the rest of CapturePort's perspective.

---

# 22. Milestone 16 — Device Discovery

Add automatic source discovery.

Distinguish:

```text
PTP camera
mounted removable filesystem
ordinary directory
```

Avoid showing the same camera twice when both mechanisms expose it.

Prefer the mounted filesystem/Card source for mass-storage devices; suppress
libgphoto2's generic Mass Storage Camera and disk adapters. Keep real PTP camera
entries and their USB topology-based mount deduplication.

New devices should appear automatically.

Do not automatically begin copying.

UI:

```text
Sources

● Sony A7C II
  SD Card 128 GB

○ /run/media/user/camera-card
```

Remember aliases for stable devices.

### Definition of Done

Plugging in a camera/card causes it to appear without restarting CapturePort.

---

# 23. Milestone 17 — Import History

Add the History view.

Show sessions rather than building a media catalog.

Explicit browser **Mark as imported** declarations are stored separately from
successful import history. Restore their distinct manual label on source scans;
they must not fabricate destination copies or verification evidence.

Example:

```text
Sep 27, 2026
Sony A7C II

428 imported
36.4 GB

~/Pictures/2026/09/27
~/Videos/2026/09/27
```

Clicking a session can show:

```text
filename
destination
verification status
errors
```

Do not turn this into an album browser.

### Definition of Done

The user can answer:

> What did I import from this camera yesterday?

without CapturePort becoming a DAM.

---

# 24. Milestone 18 — Crash Recovery

On startup, inspect:

```text
incomplete sessions
partial files
unfinished database states
```

Partial file naming must make CapturePort-owned temporary files unambiguous.

Never automatically delete arbitrary `.partial` files belonging to another application.

Recovery options:

```text
Resume where safe
Clean incomplete files
Inspect session
```

For filesystem sources, resume can eventually support offset copying.

For PTP, restarting the current file may be safer initially.

### Definition of Done

Kill CapturePort during a large import, restart it, and obtain an accurate description of what completed and what did not.

---

# 25. Milestone 19 — Clock Correction

Implement an import-time timestamp transform.

Model:

```rust
struct TimeCorrection {
    offset: Duration,
    assumed_timezone: Option<Timezone>,
}
```

Original metadata remains unchanged.

Pipeline:

```text
raw capture timestamp
        ↓
timestamp normalization
        ↓
clock correction
        ↓
effective capture time
        ↓
sorting/grouping/templates
```

UI can support:

```text
Camera time:  14:03:12
Actual time:  14:08:37

Correction: +00:05:25
```

### Definition of Done

Correcting camera time updates preview paths immediately without changing source files.

---

# 26. Milestone 20 — Multiple Destinations / Backup

Do this after primary import reliability is established.

Plan one source into multiple destinations:

```text
Camera
 ├→ ~/Pictures
 └→ /mnt/NAS/Photos
```

Each copy gets independent status.

Model:

```rust
DestinationResult {
    destination_id,
    copy_state,
    verification_state,
}
```

Preset decides whether completion requires:

```text
primary only
```

or:

```text
all required destinations
```

Only after required destinations verify should source deletion ever become eligible.

### Definition of Done

A failed NAS does not invalidate a successful local copy, but the session clearly reports incomplete backup state.

---

# 27. Milestone 21 — Library Reconciliation

Implement explicit:

```text
Scan Existing Library
```

Never run a complete library scan whenever a camera connects.

The scan should:

```text
walk configured roots
↓
read cheap identity information
↓
calculate fingerprints when useful
↓
populate/reconcile catalog
```

Support cancellation and continuation.

This allows rebuilding useful database state after:

- database loss;
- reinstall;
- manual copies;
- migrating from another importer.

### Definition of Done

A fresh CapturePort database can learn about an existing organized media collection without changing it.

---

# 28. GPUI State Architecture

Keep one central application model or a small number of strongly scoped entities.

Conceptual structure:

```text
CapturePortApp

├── SourceState
├── MediaBrowserState
├── SelectionState
├── ImportConfiguration
├── ImportPlanState
├── ImportProgressState
└── HistoryState
```

Avoid having every thumbnail component own independent copies of `MediaItem`.

Store media centrally.

Views reference stable IDs.

This becomes important with thousands of media objects.

---

# 29. Concurrency Model

Do not introduce Tokio merely because the application performs background work.

Start with:

```text
GPUI foreground executor
GPUI background tasks
dedicated blocking workers
channels
```

Use dedicated workers for:

```text
SQLite
libgphoto2
potentially FFmpeg
```

Use bounded concurrency for:

```text
metadata
thumbnails
hashing
```

The current GPUI API explicitly provides foreground and background executors, so use those facilities rather than creating a competing UI runtime unless later profiling shows a concrete need.

---

# 30. Job Scheduler

Eventually create a small job scheduler rather than independently spawning arbitrary background tasks.

Priorities:

```text
P0
Active import

P1
Visible thumbnails
Visible metadata

P2
Import-status resolution

P3
Near-visible thumbnails

P4
Background source metadata

P5
Library reconciliation
```

Cap:

```text
thumbnail decodes
metadata extraction
hashing jobs
```

independently.

Disk workloads and CPU workloads have different useful concurrency levels.

---

# 31. Error Model

Use typed errors internally.

Example:

```rust
enum ImportError {
    SourceDisconnected,
    SourceReadFailed,
    DestinationUnavailable,
    DestinationFull,
    PermissionDenied,
    Collision,
    VerificationFailed,
    DatabaseFailed,
    Cancelled,
}
```

Each error should also expose user-facing context.

Never directly display:

```text
std::io::Error: Os { code: 5 ... }
```

as the primary message.

Instead:

```text
Could not copy DSC02341.ARW

The destination drive stopped responding.

The original file is unchanged.
```

Technical detail can be available under:

```text
Details
```

---

# 32. Logging and Diagnostics

Use structured logging.

Good candidates:

```text
tracing
tracing-subscriber
```

Include:

```text
session ID
source ID
media ID
operation
duration
error
```

Avoid writing EXIF/GPS or arbitrary user metadata into logs unnecessarily.

Provide later:

```text
Export diagnostic log
```

---

# 33. Test Infrastructure

The fake-source infrastructure is important enough to be considered a product subsystem.

Implement scenarios such as:

```text
NormalCamera
LargeCard
SlowCard
DuplicateCard
ReformattedCamera
DisconnectingCamera
CorruptSource
TimestampBrokenCamera
```

Example:

```rust
FakeSourceBuilder::new()
    .files(10_000)
    .read_latency(...)
    .disconnect_after_bytes(...)
    .build();
```

This lets coding agents test failure behavior without physical hardware.

---

# 34. Filesystem Integration Tests

Use temporary directories.

Every importer test should validate actual filesystem state.

Test scenarios:

```text
successful import
cancel halfway
destination full simulation
existing identical file
existing different file
source disappears
destination disappears
verification mismatch
duplicate generated filenames
invalid template
```

After every failure assert:

```text
original intact
no incorrect DB success record
no incomplete final destination
partial file identifiable
```

---

# 35. Database Tests

Run migration tests from every historical database version.

Test:

```text
fresh DB
upgrade DB
interrupted migration
invalid DB
missing DB
read-only DB
```

Backup before migrations once real user data exists.

---

# 36. Performance Benchmarks

Create reproducible benchmarks.

Synthetic source:

```text
10,000 photos
500 videos
1 TB represented metadata
```

Measure separately:

```text
enumeration throughput
catalog lookup
template generation
planning
thumbnail queue behavior
quick fingerprint throughput
full hash throughput
```

Never optimize based only on intuition.

---

# 37. UI Regression Testing

Build debug hooks allowing CapturePort to launch directly into fake states.

Examples:

```text
captureport --demo empty
captureport --demo camera
captureport --demo importing
captureport --demo errors
captureport --demo 10000
```

This will be extremely valuable for coding agents testing GPUI surfaces.

A deterministic demo mode is preferable to requiring a real camera for every visual test.

---

# 38. Packaging

Target Linux first.

Provide at least:

```text
Arch package
.deb
AppImage or equivalent portable distribution
```

Evaluate Flatpak carefully because direct USB camera access, mounted volumes, and removable storage permissions complicate sandboxing.

Do not let Flatpak requirements distort the core architecture.

Camera packages may depend on system libgphoto2 even if SQLite is bundled.

---

# 39. First Public MVP Boundary

The first release should stop here:

```text
filesystem sources
PTP cameras

photo + video discovery

metadata
thumbnails

new/imported detection

RAW+JPEG grouping

custom destination templates
custom rename templates

separate photo/video destination

import preview

transaction-safe import

standard/strict verification

presets

import history

basic recovery
```

Do not delay MVP for:

```text
NAS backup
source deletion
library reconciliation
advanced clock correction
plugins
cloud services
AI
photo editing
DAM functionality
```

---

# 40. Recommended Implementation Order

The practical coding order should be:

```text
00 project skeleton
01 core domain types
02 fake MediaSource
03 GPUI source/media browser
04 filesystem MediaSource
05 selection/filter/sorting
06 SQLite catalog worker
07 import-history matching
08 metadata normalization
09 thumbnail system
10 template engine
11 import planner
12 transactional copy engine
13 verification/fingerprints
14 collision handling
15 RAW+JPEG bundles
16 presets
17 session/date grouping
18 libgphoto2 adapter
19 automatic device detection
20 history view
21 crash recovery
22 clock correction
23 multiple destinations
24 library reconciliation
25 source deletion
```

Do not reorder `import planner → importer`.

The planner should exist before sophisticated copying because it establishes the invariant that what CapturePort previews is exactly what CapturePort executes.

Likewise, do not implement source deletion until verified multi-destination behavior is mature.

---

# 41. First Five Agent-Sized Development Tasks

For actual coding-agent work, start even smaller.

### Task CP-001 — Workspace and GPUI shell

Deliver:

```text
workspace
GPUI window
basic layout
logging
XDG paths
CI cargo check/test
```

No media functionality.

---

### Task CP-002 — Domain model and fake source

Deliver:

```text
MediaSource trait
domain types
fake source
event stream
10k synthetic item test
```

GPUI displays fake files.

---

### Task CP-003 — Filesystem source

Deliver:

```text
folder picker
recursive enumeration
media classification
cancellation
incremental UI updates
```

No metadata yet.

---

### Task CP-004 — SQLite import history

Deliver:

```text
catalog worker
migration system
source/media/import schema
lookup API
new/imported UI state
```

No hashing yet.

---

### Task CP-005 — Minimal real import

Deliver:

```text
destination chooser
simple import plan
preserve original filenames
temporary copy
size verification
atomic commit
DB update
progress UI
```

After CP-005, CapturePort is already a primitive but genuinely usable camera-folder importer.

That is the first major development checkpoint.

---

# 42. Second Development Checkpoint

Then implement:

```text
CP-006 Metadata

CP-007 Thumbnails

CP-008 Template engine

CP-009 Full planner + collisions

CP-010 BLAKE3 verification

CP-011 Advanced duplicate matching

CP-012 RAW+JPEG bundles
```

At this checkpoint CapturePort should already outperform a generic file-copy workflow substantially.

---

# 43. Third Development Checkpoint

Then implement hardware integration:

```text
CP-013 GPhoto adapter

CP-014 PTP enumeration

CP-015 PTP downloading

CP-016 Device discovery

CP-017 Disconnect/reconnect handling
```

Only after filesystem import is stable should the camera protocol become part of the critical path.

That keeps camera-specific debugging from being confused with importer bugs.

---

# 44. Final Reliability Pass

Before declaring 1.0, run destructive-condition testing.

Repeatedly test:

```text
kill -9 during import

unplug source

unplug destination

fill destination disk

make destination read-only

change destination during planning

reformat/repopulate test camera

reset camera filename counter

inject bit corruption

delete database

corrupt database

10k–100k media source

very large video files

identical timestamps

Unicode filenames

files without extensions

broken EXIF

wrong camera clock
```

For every case, verify the fundamental guarantee:

> CapturePort must prefer refusing or stopping an operation over silently losing, overwriting, skipping, or misclassifying media.

---

# 45. 1.0 Architecture Invariants

Treat these as tests rather than merely design documentation.

1. Source media is never modified during ordinary import.

2. Existing destination files are never silently overwritten.

3. Final destination names never contain partially copied files.

4. Database success is recorded only after destination commit.

5. Deleting the CapturePort database does not damage the media library.

6. A full destination-library scan is not required for normal camera reconnection.

7. Weak duplicate evidence never becomes an automatic `Imported` classification.

8. Import preview and actual destination paths use the same `ImportPlan`.

9. Expensive work never executes on GPUI's UI thread.

10. GPUI, SQLite, libgphoto2, RAW parsing, and FFmpeg remain behind replaceable boundaries.

11. Import rules produce deterministic results.

12. Source deletion is never coupled to the basic Import operation.

---

# 46. Desired End-State

The final execution path should remain conceptually simple:

```text
Device connected
       ↓
Source discovered
       ↓
Media enumerated
       ↓
History lookup
       ↓
Metadata + thumbnails
       ↓
New files selected
       ↓
Preset applied
       ↓
ImportPlan generated
       ↓
User reviews preview
       ↓
Transactional copies
       ↓
Verification
       ↓
Atomic commit
       ↓
Catalog update
       ↓
Import session completed
```

That pipeline should remain recognizable in the codebase even after CapturePort grows.

If implementing a new feature makes that flow substantially harder to understand, the feature should probably be moved behind an adapter or reconsidered entirely.
