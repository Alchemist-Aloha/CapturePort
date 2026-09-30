# Capture Port

## 1. Product Definition

A lightweight native Linux application for safely importing photographs and videos from cameras, memory cards, mounted storage devices, and arbitrary source folders.

The application is intentionally an **ingest utility**, not a photo catalog, DAM, RAW editor, or media organizer.

Its responsibility ends after media has been:

1. discovered,
2. classified,
3. checked against previous imports,
4. organized according to user-defined rules,
5. copied,
6. verified,
7. recorded in import history.

The filesystem remains the authoritative media library.

The application's database exists only to accelerate future imports, remember history and settings, and provide auditability. Deleting the database must never make imported media unusable or difficult to locate.

---

# 2. Primary Goals

The application should optimize for:

- extremely fast startup;
- immediate camera/card detection;
- very low idle resource usage;
- simple and predictable import workflows;
- safe handling of photographs and video;
- reliable detection of previously imported files;
- flexible date/time grouping;
- flexible automatic renaming;
- interruption-safe importing;
- clear preview of all destination paths before copying;
- strong keyboard navigation;
- native Linux behavior;
- minimal database overhead;
- clean Rust architecture independent of the UI framework.

A typical workflow should require approximately:

1. Connect camera or insert card.
2. Application discovers media automatically.
3. Previously imported media is deselected automatically.
4. User reviews new media.
5. User chooses or confirms an import preset.
6. User presses **Import**.
7. Application copies and verifies media.
8. Application reports completion.

---

# 3. Non-Goals

The application should deliberately avoid becoming another digiKam or Lightroom.

The initial product does not need:

- RAW image editing;
- photo development;
- image adjustment tools;
- photo collections or albums;
- face recognition;
- semantic search;
- cloud synchronization;
- extensive metadata editing;
- full DAM functionality;
- permanent proprietary library structures;
- AI features;
- general-purpose file management.

Basic preview, metadata inspection, rating display, and source browsing are acceptable where they directly support importing.

---

# 4. Technology

Primary implementation:

```text
Language         Rust
GUI              GPUI
Database         SQLite via rusqlite
Hashing          BLAKE3
Camera protocol  libgphoto2 / PTP
Filesystem       Native Rust filesystem APIs
Async work       Background worker/task system
```

The backend must not depend on GPUI.

GPUI is a presentation layer over an independent import engine.

The core importer should eventually be reusable by:

```text
GUI
CLI
automated tests
possible daemon/service
```

---

# 5. Source Model

The application exposes all import sources through a common abstraction.

Conceptually:

```rust
trait MediaSource {
    fn identity(&self) -> SourceIdentity;

    fn enumerate(&self) -> Result<Vec<MediaItem>>;

    fn open_stream(
        &self,
        item: &MediaItem,
    ) -> Result<Box<dyn Read>>;

    fn read_range(
        &self,
        item: &MediaItem,
        offset: u64,
        length: usize,
    ) -> Result<Vec<u8>>;
}
```

Initial source implementations:

```text
FilesystemSource
    Mounted SD cards
    USB mass-storage cameras
    External drives
    Arbitrary folders

GPhotoSource
    PTP cameras
    MTP-compatible camera devices
```

Future sources may be added without changing the importer.

The importer must never assume the source is seekable, writable, permanently mounted, or fast.

---

# 6. Source Identity

A connected device should receive the most stable identity possible. Opened
folders on the same volume remain distinct catalog sources: source identity
includes the folder's path within its mounted volume.

Identity sources may include:

```text
camera serial number
camera manufacturer
camera model
USB vendor/product identifiers
volume UUID
filesystem UUID
device identifier
user-assigned alias
```

No single identifier should be assumed universally available.

Example:

```rust
struct SourceIdentity {
    source_type: SourceType,
    stable_id: Option<String>,
    serial: Option<String>,
    manufacturer: Option<String>,
    model: Option<String>,
    volume_uuid: Option<String>,
}
```

Users may assign friendly names:

```text
Sony A7C II
Work Camera
Drone SD Card
Phone
```

These aliases may also be used in path templates.

---

# 7. Media Discovery

Media discovery should begin immediately after a source becomes available.

Enumeration and metadata extraction should be progressive.

The UI must not wait for the entire source to be parsed before displaying files.

Desired behavior:

```text
source detected
    ↓
enumerate files
    ↓
extract metadata incrementally
    ↓
generate thumbnails on a bounded background queue
    ↓
show each capture only when metadata and thumbnail are ready
    ↓
resolve import-history status incrementally
```

A card containing thousands of items should remain responsive while previews
load. Ready captures appear progressively without reordering already displayed
tiles; loading and unavailable-preview counts remain visible. Failed metadata
or thumbnail extraction must not produce blank media tiles. Reopening the source
retries unavailable previews; files remain untouched.

Automatic mounted-source discovery lists volumes under `/media/` and
`/run/media/`. Mounts under `/mnt/` are not added to the Sources sidebar
automatically; users can still choose them with **Open folder**. Removable
storage must be mounted before it can be scanned; a device visible in a file
manager's device list is not necessarily mounted. Sources also lists unmounted
removable filesystem cards with a distinct **Not mounted** state and an explicit
**Mount** button. Mounting is never automatic: the OS's UDisks service handles
authorization and the mount operation. CapturePort rechecks the device before
requesting the mount and refreshes Sources afterward. Mount errors remain visible
without clearing usable sources. A libgphoto2 detection failure must not hide
mounted filesystem sources.

A detected removable mount is labelled with its device model on the first line
and its actual mount directory on the second, both in the Sources list button
and as the browse page title; the Sources "Current:" line joins them with a
middle dot. The volume's own label (for example `Card · Disk`) is not shown,
because it does not distinguish one inserted card from another; when the block
device reports no model, the mount directory stands alone.

Libgphoto2's generic **Mass Storage Camera** and `disk:` camera adapters are
hidden from the Sources sidebar. Mounted storage remains available through the
filesystem adapter as a model-and-mount-dir source. Actual PTP cameras remain
listed; matching USB mounts are suppressed only for those PTP cameras.

Device discovery starts at launch and polls every three seconds. **Refresh devices**
(and **Scan for cameras** in the empty view) requests an immediate scan on the
same serialized worker. While a manual scan is pending, the action shows
“Scanning devices…” and repeated requests are ignored. Completion reports the
number of sources found; failure preserves the previous list and offers retry.

---

# 8. Supported Media

The importer should distinguish at minimum:

```text
RAW image
JPEG
HEIF/HEIC
PNG
TIFF
video
sidecar
unknown media
```

Supported video containers should initially include common camera formats such as:

```text
MP4
MOV
MTS/M2TS where practical
```

Files whose extensions are not recognized as image, RAW, or video are excluded
from the browser by default; sidecar and unknown media are the default ignored
types. Import settings expose comma-separated **additional photo extensions**,
**additional video extensions**, **excluded extensions**, and **ignored media
types** (`raw`, `photo`, `video`, `sidecar`, `unknown`). Odd extensions listed
under the additional photo or video fields are imported as stills or video,
extensions listed under excluded are dropped outright, and listed media types
are dropped. These rules persist with the preset across sessions, are applied
while scanning, and are re-applied when building an import plan, so the preview
and history contain only files the rules allow.

---

# 9. Logical Media Bundles

Files should not always be treated independently.

Related files should optionally be grouped into logical media bundles.

Examples:

```text
DSC01234.ARW
DSC01234.JPG
```

becomes:

```text
DSC01234
 ├ RAW
 └ JPEG
```

Likewise:

```text
C0001.MP4
C0001.XML
```

may form a video bundle.

Potential bundle types:

```text
RAW + JPEG
RAW + HEIF
video + sidecar
photo + auxiliary metadata
live-photo-style pairs
```

The source browser displays a bundle as one parent capture in the virtualized
grid. Clicking the parent's image or its View files line expands a tree panel in
the same browser, with each source file as an indented child. Clicking the group
name instead selects all members, or clears them when every member is already
selected. Children can be selected independently. The tree provides Select all
in group and Deselect all in group actions; the parent shows the selected-member
count, including partial selections. Expanding and collapsing never changes
selection. A bundle remains visible under a filter if any member matches it,
while its expanded tree shows all members.

Explicit member and group selections take precedence over RAW-only/JPEG-only
preset policy for those selected members. Untouched bundles continue to follow
the preset. The exact selected files drive the deterministic import preview and
execution; import history records only files actually attempted by that plan.

A RAW+JPEG pair is labeled `RAW+JPEG` in the browser and planned as one unit.
The RAW is the source file; the JPEG is a sidecar. Both share the unit's session
and sequence number, and the JPEG is written beside the RAW using the RAW's
destination stem with a `.jpg` extension (the primary copy and any backup copy).
The import review collapses the pair into one row listing both destinations,
and the history detail groups the pair's recorded copies into one block. The
browser renders the pair from the JPEG's thumbnail, so a RAW whose own preview is
missing or expensive to decode does not hide or delay the pair.

The browser display can also turn pairing off, listing the RAW and JPEG as
separate selectable files. Separate display changes only browsing and selection;
the selected files still drive the plan and the import bundle policy is
unchanged.

Import rules can specify:

```text
Keep RAW + JPEG
RAW only
JPEG only
All bundle members
```

---

# 10. Media Metadata Model

Different file formats expose metadata differently.

The rest of the application should not care whether capture time came from EXIF, QuickTime metadata, a filesystem timestamp, or camera PTP information.

Normalize metadata into an internal structure:

```rust
struct MediaMetadata {
    capture_time: Option<DateTime<FixedOffset>>,
    filesystem_time: Option<DateTime<FixedOffset>>,

    camera_make: Option<String>,
    camera_model: Option<String>,
    camera_serial: Option<String>,

    width: Option<u32>,
    height: Option<u32>,

    orientation: Option<Orientation>,

    duration: Option<Duration>,

    lens: Option<String>,

    gps: Option<Coordinates>,

    media_type: MediaType,
}
```

Metadata extraction failures must not block importing.

EXIF GPS normalization applies the latitude/longitude hemisphere reference
tags, so south and west coordinates are negative. Missing or invalid reference
tags or out-of-range coordinates leave GPS unavailable rather than assuming a
hemisphere; both valid axes are required.

---

# 11. Timestamp Handling

Timestamp handling must be explicit because camera clocks are frequently incorrect.

The user can choose a capture-time source:

```text
Embedded capture timestamp
Camera-reported timestamp
Filesystem modification timestamp
```

Default priority:

```text
embedded capture time
    ↓
camera timestamp
    ↓
filesystem modification time
```

The importer should support temporary clock correction during an import.

Example:

```text
Camera shows:
2026-09-26 14:03:12

Actual time:
2026-09-26 14:08:37

Correction:
+00:05:25
```

The correction should affect:

```text
grouping
destination paths
generated filenames
displayed corrected capture time
```

It should not silently modify source-file metadata.

Timezone interpretation should also be configurable where metadata lacks timezone information. Import settings present a timezone selection menu of common UTC offsets plus a
Capture metadata option and a custom seconds field; the chosen offset is stored
with the preset, persists across sessions, and is applied to grouping,
destination paths, generated filenames, and displayed corrected capture times.
Import history therefore reflects the timezone through the recorded
destinations and times.

---

# 12. Import History

The application must not rely on a simple:

```text
last_imported_file
```

marker.

That model fails after:

```text
camera counter reset
card reformat
multiple cards
multiple cameras
manual file deletion
manual imports
clock changes
file renaming
partial imports
```

Instead, every successfully imported source item is recorded individually.
The history detail shows each recorded copy with its destination file's
modification time formatted in the selected timezone, so changing the timezone
setting changes how history times are displayed.

---

# 13. Duplicate Detection Philosophy

The importer should answer:

> Have I successfully imported this source media before?

It should not attempt to solve general-purpose duplicate-photo detection during every import.

There are two separate concepts:

```text
Import-history matching
Library duplicate detection
```

Import-history matching is automatic and fast.

Library-wide duplicate scanning is optional and explicit.

---

# 14. Import Status

Every discovered media item receives one of these states:

```text
NEW

IMPORTED

POSSIBLE_DUPLICATE

UNKNOWN
```

Meaning:

### NEW

No credible previous import exists.

Selected by default.

### IMPORTED

The application has strong evidence that this exact source object was imported successfully.

Deselected by default.

### POSSIBLE_DUPLICATE

Metadata strongly resembles an existing item, but identity cannot safely be established.

Deselected or visually flagged according to user preference.

### UNKNOWN

Identity checking could not be completed.

The UI must never silently classify uncertain media as already imported.

---

# 15. Duplicate Identification Strategy

Use progressive matching.

## Level 1 — Import database lookup

Cheap fields:

```text
source identity
source path
source filename
file size
capture timestamp
```

A database identity match alone suggests a possible duplicate, not a verified
match: a source file may have changed in place. Import-time fingerprints must
remain intact when subsequent scans observe that file again.

---

## Level 2 — Quick fingerprint

If source identity is ambiguous, calculate a lightweight fingerprint.

Recommended input:

```text
file size
+
first data block
+
last data block
```

For example:

```text
first 256 KiB
last 256 KiB
```

hashed using BLAKE3.

The exact chunk size should be configurable internally and benchmarked.

This avoids rereading multi-gigabyte video files while identifying possible
duplicates. A quick-only match never proves that unchanged middle bytes were
imported; show **Possible duplicate** with the prior session for review.

---

## Level 3 — Full content hash

A full BLAKE3 hash is used when:

```text
strict verification is enabled
identity remains ambiguous
the user explicitly requests content-level deduplication
library reconciliation is occurring
```

Full hashing should not be mandatory during ordinary source enumeration.

---

# 16. Library Reconciliation

The normal import path must not scan the user's complete picture/video library.

Instead provide an explicit function:

```text
Scan existing library
```

This operation:

1. walks configured destination roots;
2. records existing files;
3. reads metadata where useful;
4. calculates fingerprints;
5. optionally calculates full hashes;
6. reconciles them with import history.

Use cases:

```text
first installation
database was deleted
user imported files manually
library moved from another computer
destination folders changed
```

The operation may be incremental.

Results should be cached.

---

# 17. Database Philosophy

The database is a cache plus history.

The filesystem remains authoritative.

Deleting:

```text
~/.local/share/<app>/database.sqlite
```

must never break or invalidate the user's media library.

A user should be able to reconstruct useful history by scanning existing destination directories.

---

# 18. Database Schema

A lightweight SQLite schema should contain approximately:

```sql
sources
-------
id
stable_id
source_type
manufacturer
model
serial
alias
first_seen_at
last_seen_at


media
-----
id
source_id
source_path
source_filename
source_size
capture_time
media_type

quick_fingerprint
content_hash

first_seen_at


import_sessions
---------------
id
source_id
preset_id
started_at
completed_at
status


imports
-------
id
session_id
media_id

destination_path
destination_size

copy_completed_at
verification_method
verified_at

status


presets
-------
id
name
configuration_json
created_at
updated_at
```

Indexes should exist for fields commonly used during source matching:

```text
source_id
source_path
source_filename
source_size
capture_time
quick_fingerprint
content_hash
```

Do not prematurely normalize every configuration setting into relational tables.

Preset rules can reasonably be stored as versioned structured data.

---

# 19. Import Sessions

Each import operation creates an immutable logical session.

Example:

```text
Import #48
September 26, 2026 18:42

Source:
Sony A7C II

Files:
438

Imported:
428

Previously imported:
10

Destination:
~/Pictures
~/Videos
```

Import sessions provide:

```text
history
auditability
failure recovery
troubleshooting
reimport assistance
```

They are not photo albums.

---

# 20. Destination Rules

Destination paths are generated through templates.

Examples:

```text
~/Pictures/{year}/{month}/{day}/

~/Pictures/{year}/{date}/

~/Videos/{year}/{date}/
```

Supported initial variables:

```text
{year}
{month}
{day}

{date}
{time}
{datetime}

{camera}
{camera_make}
{camera_model}
{camera_serial}

{media_type}

{original_name}
{original_stem}
{extension}

{sequence}
{session}
{session_name}
```

Formatting should be supported:

```text
{sequence:04}
```

producing:

```text
0001
0002
0003
```

`{session}` is the one-based deterministic shooting-session number assigned
after timestamp correction and grouping. It can be used in destination or
filename templates.

`{session_name}` is the gallery display name of the item's shooting session:
the user's rename when one is set, otherwise the gallery's default name (its
earliest media's date, with the `a`/`b`/`c` duplicate suffix). It renders
`unknown` when time-gap grouping is off, so the segment never empties a folder
level or turns a path invalid. Because it follows the visible gallery grouping,
an import of a subset can merge galleries that had different names; each item
keeps the name of the gallery it came from.

Folder and filename templates also expose the normalized metadata available to
the importer. All segments work in both kinds of template:

| Category | Segments | Output |
| --- | --- | --- |
| Time | `{hour}`, `{minute}`, `{second}` | Two-digit corrected capture-time parts |
| Source | `{source_name}`, `{file_size}` | Source display name; file size in bytes |
| Camera | `{lens}` | Recorded lens model or make |
| Dimensions | `{width}`, `{height}`, `{dimensions}` | Pixels; dimensions such as `6000x4000` |
| Orientation | `{orientation}` | `normal`, `rotate90`, `rotate180`, `rotate270`, `mirror_horizontal`, or `mirror_vertical` |
| Video | `{duration_millis}`, `{duration_seconds}` | Milliseconds; seconds with three decimal places |
| Location | `{gps_latitude}`, `{gps_longitude}` | Decimal degrees with seven decimal places |
| Recorded timestamps | `{capture_time}`, `{filesystem_time}` | Original recorded timestamps, without clock correction |
| Timestamp origin | `{timestamp_source}` | `exif_original`, `quicktime`, `camera`, or `filesystem` |

Camera segments prefer embedded metadata over source identity. `{camera}` falls
back to the source model or display name. Unavailable camera identity retains
its existing empty-string fallback. Missing optional metadata in the added
segments produces `unknown`; this keeps missing metadata from silently removing
a folder level.

`{date}`, `{time}`, and `{datetime}` accept strftime formatting, for example
`{date:%Y-%m-%d}` and `{datetime:%Y-%m-%d_%H-%M-%S}`. Common directives are `%Y`
(year), `%m` (month), `%d` (day), `%H` (hour), `%M` (minute), `%S` (second), and
`%%` (literal percent). These segments use the corrected effective capture time.
Both sequence and session support a zero-padding width of 1–12, such as
`{sequence:04}` or `{session:03}`. Unformatted session numbers keep their existing
two-digit minimum width.

Only literal `/` characters in folder templates create folder levels. Token
values are sanitized as a single segment: path separators, control characters,
and `:*?"<>|` become underscores. Unsafe trailing dots/spaces, reserved device
names such as `CON` and `NUL`, and reserved CapturePort partial prefixes are
neutralized in token values. Literal unsafe paths remain invalid, including
these reserved names, `.` or `..` folder components, repeated separators,
invalid filename characters, and trailing dots/spaces. Filename templates cannot
contain folder separators. A blank folder
template imports directly into the absolute destination root.

---

# 21. Media-Type Destinations

Photos and videos should be independently routable.

Example:

```text
Photos
→ ~/Pictures/{year}/{date}/

Videos
→ ~/Videos/{year}/{date}/
```

Other rules may eventually route:

```text
RAW
JPEG
video
sidecars
```

independently.

---

# 22. Date Grouping

The grouping system should be more capable than merely:

```text
YYYY/MM/DD
```

Built-in modes:

```text
None
Day
Week
Month
Year
```

Custom path templates remain available for arbitrary structures.

---

# 23. Session / Gap-Based Grouping

An optional grouping mode should detect shooting sessions from time gaps.

Example source times:

```text
09:01
09:04
09:13

13:54
13:58
14:03
```

With:

```text
New session after 60 minutes
```

the application assigns two shooting sessions. Time-gap grouping controls
gallery sections, session numbers, and per-session sequences; it does not append
an extra destination directory. Primary and backup folder templates determine
the session folder structure. To create folders such as:

```text
2026-09-26_01/
2026-09-26_02/
```

use `{date:%Y-%m-%d}_{session}` explicitly in the folder template.

Possible configuration:

```text
Session split threshold:
30 min
1 hour
2 hours
custom
```

This feature should be optional and deterministic.

---

# 24. Filename Templates

Filename generation should use the same template engine as destination directories.

Example:

```text
{date}_{time}_{sequence:04}.{extension}
```

produces:

```text
20260926_142301_0001.ARW
20260926_142303_0002.ARW
```

Another preset:

```text
{date}_{time}_{camera_model}_{sequence:04}.{extension}
```

The original filename should always be available as a token.

Extensions should preserve appropriate case according to user preference.

---

# 25. Sequence Determinism

Sequence numbering must be deterministic.

Sorting should be explicitly defined.

Default:

```text
corrected capture timestamp
then original filename
then source path
```

Running the same import plan again should generate the same filenames unless files or settings changed.

---

# 26. Destination Preview

Before import, the application should calculate the complete import plan.

Users must be able to inspect:

```text
SOURCE                         DESTINATION

DCIM/100MSDCF/DSC02341.ARW  →  Photos/2026/09/26/
                               20260926_142310_0001.ARW

DCIM/100MSDCF/DSC02342.ARW  →  Photos/2026/09/26/
                               20260926_142312_0002.ARW
```

Conflicts must be shown before importing.

Possible statuses:

```text
Ready

Already imported

Destination collision

Invalid path

Destination unavailable

Insufficient disk space
```

---

# 27. Collision Handling

Destination collisions must never silently overwrite media.

Available policies:

```text
Stop and ask
Skip
Generate unique suffix
Verify whether content is identical
```

Recommended default:

```text
If destination exists:
    compare size / fingerprint

if identical:
    mark existing

if different:
    raise collision
```

Automatic destructive replacement should not be part of the default importer workflow.

---

# 28. Import Transaction

A successful import must behave transactionally from the user's perspective.

Per-file process:

```text
1. Validate destination
2. Create temporary file
3. Copy source → temporary file
4. Flush and close temporary file
5. Verify copied file
6. Atomically rename temporary → destination
7. Record successful import in database
```

Conceptually:

```text
DSC01234.ARW

    ↓

Pictures/2026/09/26/.camera-import-a84e.tmp

    ↓ verification

Pictures/2026/09/26/20260926_142301_0001.ARW
```

The final destination filename must never represent an incomplete copy.

---

# 29. Verification

Verification modes:

```text
Fast
    file size only

Standard
    size + source/destination quick fingerprint

Strict
    full source/destination BLAKE3
```

Recommended default:

```text
Standard
```

Users importing professionally or before deleting source media may enable Strict mode.

---

# 30. Crash Recovery

The importer must tolerate:

```text
application crash
system shutdown
camera disconnect
card removal
destination disconnect
disk-full condition
```

On restart, incomplete import sessions should appear as:

```text
Interrupted import detected
```

Temporary files should be identified safely.

The application may offer:

```text
Resume
Clean up
Inspect
```

Never infer successful import merely because a destination file exists.

---

# 31. Source Deletion

Deleting media from a camera/card is a high-risk operation and should not be part of the main import button.

It should be a separate optional operation.

Deletion should only be offered for media whose required copies have been successfully verified.

Possible requirement:

```text
Primary destination verified
AND
Backup destination verified, if configured
```

before deletion is available.

Default:

```text
Do not delete source files.
```

---

# 32. Multiple Destinations / Backup

The importer should eventually support simultaneous destinations.

Example:

```text
Camera
   ├─→ ~/Pictures
   └─→ /mnt/NAS/Photos
```

Each destination has independent state:

```text
Primary
    copied ✓
    verified ✓

NAS
    copied ✓
    verified ✓
```

The source item is considered fully completed according to the preset's required-destination policy.

This makes the tool useful as a genuine ingest utility rather than merely a copy dialog.

---

# 33. Presets

Import settings should be first-class reusable presets.

Example:

```text
Everyday

Photo destination:
~/Pictures/{year}/{date}

Video destination:
~/Videos/{year}/{date}

Filename:
{date}_{time}_{sequence:04}

Verification:
Standard

Backup:
Disabled
```

Another:

```text
Travel

Destination:
~/Photos/{year}/{event}

Keep:
RAW + JPEG

Verification:
Strict

Backup:
NAS
```

A preset may contain:

```text
destination rules
renaming rules
grouping rules
bundle rules
verification mode
backup rules
clock settings
selection filters
```

---

# 34. User Interface

The primary UI should remain compact.

Suggested layout:

```text
┌─────────────────────────────────────────────────────────────┐
│ Sony A7C II                               438 items · 36 GB │
├─────────────────────────────────┬───────────────────────────┤
│                                 │ Settings                  │
│  [✓] thumbnail   DSC02341       │                           │
│  [✓] thumbnail   DSC02342       │ Preset: Everyday       ▾ │
│  [ ] thumbnail   DSC02343 ✓     │                           │
│  [✓] thumbnail   C0012.MP4      │ Photos                    │
│                                 │ ~/Pictures/{year}/{date}  │
│                                 │                           │
│                                 │ Videos                    │
│                                 │ ~/Videos/{year}/{date}    │
│                                 │                           │
│                                 │ Rename                    │
│                                 │ {date}_{time}_{seq:04}    │
│                                 │                           │
├─────────────────────────────────┴───────────────────────────┤
│ 414 new · 24 imported             34.7 GB       Import 414 │
└─────────────────────────────────────────────────────────────┘
```

The application should emphasize the media, not configuration chrome.

The browser keeps source choice, media filters, and selection actions visible.
Secondary gallery controls (sort order, thumbnail size, and session gap) live
under **View options** so they do not crowd out the media. Bundle cards show the
number of selected files and open a separate member-selection panel; expanding
a bundle never selects it.

The workspace navigation exposes **Review import** once a plan exists. That
view shows the planned destinations, a ready/blocked summary, and a visible
reason when confirmation is unavailable. Settings group destination, naming,
import-rule, capture-time, and backup controls so their effects are easier to
find. Empty browser, history, and recovery views explain the next useful action.

---

# 35. Default Selection

Initial filter:

```text
[x] New

[ ] Previously imported

[ ] Possible duplicates
```

Previously imported media should remain visible unless the user chooses to hide it.

This prevents confusion such as:

> Why are half of the files on my camera missing?

Imported media should visually indicate its state.

Example:

```text
✓ Imported Sep 20
```

The main browser selection toolbar includes **Mark as imported**. It applies
to all selected files, including selected files hidden by the current filter
and individually selected bundle members. Files already classified as imported
are left unchanged. The action is unavailable while scanning, planning,
importing, or saving a manual mark, and when no eligible files are selected.

Manual marking records an explicit user declaration in the catalog; it does
not copy files or create a successful import session, destination record, or
verification evidence. Marked files are deselected, appear under the Imported
filter with `✓ Imported · marked manually`, and are excluded from Select new.
The declaration survives rescans and restarts for the same source media identity
and quick fingerprint, when one is available. A changed fingerprint invalidates
the declaration. Actual completed-copy evidence takes precedence on rescan.
Filesystem scans restore a manual declaration only after obtaining a current
quick fingerprint; a failed read cannot reuse stale cached evidence. PTP sources
retain explicit identity-based declarations without downloading media to scan.
Catalog writes run off the UI thread and update the browser only after the
entire selected batch is persisted. Failed writes leave the selection and status
unchanged. Changing sources while the write finishes cannot mark files in the
new source. Any existing import preview is invalidated when marking starts.

---

# 36. Views

Initial application needs only two main views:

## Source View

Thumbnail/list browser of current source.

## Import History

Previous import sessions.

Avoid introducing permanent album/library navigation.

---

# 37. Thumbnail Loading

Thumbnail loading must never block media enumeration.

Priority:

```text
visible rows
near-visible rows
remaining selected items
everything else
```

Sources may provide embedded thumbnails.

Prefer embedded previews where possible rather than decoding full RAW files.

Video thumbnails should be generated asynchronously.

Thumbnail caches may be stored separately from the database and safely deleted.

---

# 38. Performance Requirements

Targets should include:

### Application launch

UI should appear effectively immediately on ordinary hardware.

### Source enumeration

First source entries should appear before full enumeration completes.

### Database matching

Thousands of items should be matched without noticeable UI stalls.

### Thumbnail generation

Never block scrolling or interaction.

### Hashing

Hashing must run outside the UI thread.

### Import

Copy throughput should approach source/destination storage limits rather than CPU limits.

---

# 39. Background Work

Long-running operations should use a bounded worker system.

Examples:

```text
source enumeration
metadata extraction
thumbnail generation
fingerprinting
full hashing
copying
verification
library scan
```

Prioritize user-visible work.

Avoid spawning unbounded tasks for every media item.

For example:

```text
enumeration
    high priority

visible thumbnails
    high priority

import copies
    high priority

offscreen thumbnails
    low priority

full library reconciliation
    lowest priority
```

---

# 40. Import Queue

The import engine should operate on an explicit queue.

Each item has a state machine approximately like:

```text
Discovered

Matched

Planned

Queued

Copying

Verifying

Committed
```

Failure states:

```text
CopyFailed

VerificationFailed

SourceUnavailable

DestinationUnavailable

Collision

Cancelled
```

The UI subscribes to state changes rather than performing import logic itself.

---

# 41. Backend Event Model

UI commands:

```text
DiscoverSources

OpenSource

EnumerateSource

BuildImportPlan

StartImport

PauseImport

ResumeImport

CancelImport

ScanLibrary
```

Backend events:

```text
SourceDetected

SourceRemoved

MediaDiscovered

MetadataResolved

ThumbnailReady

ImportStatusResolved

PlanUpdated

CopyStarted

CopyProgress

VerificationStarted

VerificationCompleted

ItemCompleted

ItemFailed

SessionCompleted
```

GPUI renders application state produced by these events.

---

# 42. Architecture

Suggested workspace:

```text
crates/

    app/
        GPUI application
        views
        interaction

    core/
        shared domain types
        media model
        configuration

    source/
        MediaSource abstraction

    source-filesystem/
        cards
        mounted cameras
        folders

    source-gphoto/
        libgphoto2 integration

    metadata/
        metadata normalization

    thumbnail/
        thumbnail extraction/cache

    catalog/
        SQLite
        migration
        queries

    fingerprint/
        quick fingerprints
        BLAKE3 hashing

    rules/
        templates
        grouping
        filenames

    planner/
        import-plan generation
        collisions
        disk-space calculation

    importer/
        queue
        copy
        verification
        recovery
```

This may initially live in fewer crates, but module boundaries should follow this architecture.

Do not split into many crates simply for organizational aesthetics.

---

# 43. Configuration Storage

Separate:

```text
application preferences
```

from:

```text
import history
```

Application preferences may use:

```text
TOML
JSON
or SQLite
```

depending on implementation simplicity.

Presets should be versioned so future schema changes can be migrated.

---

# 44. Database Migrations

Database schema migrations must be explicit.

On startup:

```text
open database
validate schema version
perform forward migration
continue
```

The application should preferably keep a small automatic backup before destructive migrations.

Because the database is reconstructable, migration failure must not threaten user media.

---

# 45. Filesystem Safety

Before importing, verify:

```text
destination exists or can be created
destination is writable
sufficient free space exists
generated paths are valid
filename collisions are known
```

If multiple destinations are required, evaluate disk capacity for each.

---

# 46. Symlinks and Path Safety

Library scanning should avoid accidental traversal of:

```text
recursive symlink loops
network mount loops
unexpected filesystem boundaries
```

Policies should be explicit.

Import destinations should be canonicalized when practical.

---

# 47. Removable Destination Handling

If the destination is removable or network-backed:

```text
destination disappears
```

must pause/fail safely without corrupting completed imports.

Completed files should remain committed.

Incomplete temporary files should remain identifiable.

---

# 48. Network Storage

NAS destinations should not require special architecture.

They should normally appear as filesystem destinations such as:

```text
/mnt/photos
/run/user/.../gvfs/...
```

The application should not initially implement SMB or NFS clients internally.

Filesystem mounting remains an OS responsibility. CapturePort may request a
user-selected removable source mount through UDisks; it does not mount
destination filesystems or unmount any source automatically.

---

# 49. Search and Filtering

Source filtering should remain simple and fast.

Useful filters:

```text
New
Imported
Possible duplicate

Photo
Video

RAW
JPEG

Date range

Camera

Filename search
```

No general-purpose DAM query language is necessary.

---

# 50. Sorting

Supported sorts:

```text
Capture time
Filename
File size
Media type
Import status
```

Capture time ascending should be the default.

---

# 51. Keyboard Workflow

A professional ingest tool should support efficient keyboard operation.

Examples:

```text
Space
Select/deselect

Ctrl+A
Select visible/new items

Enter
Preview

I
Start import

F
Toggle filter panel

1–5
Optional rating assignment if implemented
```

Exact bindings should follow Linux conventions and be customizable later.

---

# 52. Accessibility

The interface should not communicate important state using color alone.

Import status requires:

```text
icon
text
and optionally color
```

Example:

```text
✓ Imported
! Possible duplicate
● New
```

---

# 53. Initial Setup

On first launch:

```text
Choose photo destination

Choose video destination

Choose filename style

Choose verification mode
```

Provide useful defaults.

For example:

```text
Photos:
~/Pictures/{year}/{date}

Videos:
~/Videos/{year}/{date}

Filename:
Keep original

Verification:
Standard
```

The user should be able to start importing without learning the template language.

---

# 54. Advanced Template Editor

Advanced users may directly edit templates.

The editor should include:

```text
token picker
live examples
validation
collision warnings
```

Example:

```text
Template

{date}_{time}_{camera}_{sequence:04}.{extension}


Preview

20260926_142304_A7C2_0001.ARW
```

Invalid templates must be detected before import.

Import settings describe folder and filename syntax next to their fields. Each
photo-folder, video-folder, and filename template has its own uniquely identified
segments/help control; clicking one reveals the corresponding insertable tokens.
Each field offers an expandable, grouped segment reference with descriptions;
clicking a segment inserts it at the text cursor and returns focus to the field.
Examples update while editing and are explicitly labeled as sample metadata.
Invalid syntax and invalid sample paths appear inline and prevent saving,
with an error naming the affected field. Photo and video folder examples use
corresponding sample media types. Examples are illustrative; the
deterministic import preview remains authoritative for actual paths, grouping,
and collision resolution.

---

# 55. Rule Evaluation

Import rules must be deterministic.

Given:

```text
same input metadata
same preset
same selected items
```

the import plan should always produce the same destination paths.

Random UUID filenames should not be used for final media unless explicitly requested.

---

# 56. Error Reporting

Errors should explain:

```text
what failed
which media item failed
whether source media remains safe
whether retry is possible
```

Bad:

```text
Error 37
```

Good:

```text
Could not copy DSC02341.ARW.

The destination drive was disconnected.

The original file is unchanged.

Reconnect the destination and retry.
```

---

# 57. Logging

Maintain structured application logs suitable for troubleshooting.

Logs may include:

```text
source detection
import session
copy operation
verification result
camera errors
database errors
filesystem errors
```

Never log full image contents.

Full paths may contain user-sensitive information, so diagnostic export should be explicit.

---

# 58. Testing Strategy

The importer engine should be extensively testable without physical cameras.

Create fake `MediaSource` implementations capable of simulating:

```text
thousands of files
slow reads
disconnects
corrupt data
camera clock errors
duplicate filenames
reformatted cards
renamed source files
read failures
```

Core tests should cover:

```text
template generation
duplicate matching
sequence determinism
collision handling
transaction safety
interruption recovery
database migration
paired-file grouping
time correction
```

Physical camera integration tests can be separate.

---

# 59. MVP

The first useful release should include only:

1. Mounted filesystem sources.
2. PTP camera access through libgphoto2.
3. Photo and video enumeration.
4. Thumbnail grid/list.
5. Metadata extraction.
6. SQLite import history.
7. Previously imported detection.
8. Default selection of new media.
9. Separate photo/video destinations.
10. Date-based folder templates.
11. Filename templates.
12. Import-plan preview.
13. Transactional copy.
14. Standard verification.
15. Import-session history.
16. RAW+JPEG grouping.
17. Collision detection.
18. Crash-safe temporary files.

This is already a complete product.

---

# 60. Post-MVP Features

Next priorities:

### Phase 2

```text
Import presets
Clock correction
Session/gap grouping
Strict full-file verification
Advanced filters
Library reconciliation
```

### Phase 3

```text
Multiple destinations
NAS backup
Resume interrupted imports
Delete-after-verified-import
Advanced sidecar pairing
```

### Optional Later Features

```text
CLI
watch/auto-import mode
plugin API
external post-import commands
XMP integration
rating import
event naming
```

---

# 61. Features Explicitly Deferred

Do not allow these to expand the MVP:

```text
AI tagging
photo editing
cloud backup
face recognition
album management
geographic browsing
full EXIF editor
media transcoding
duplicate-photo similarity detection
```

They are different products.

---

# 62. Core Product Invariants

These rules should be treated as architectural invariants.

### Invariant 1

The original source media is never modified during a normal import.

### Invariant 2

The database never becomes the only way to locate or understand imported media.

### Invariant 3

A destination filename never represents an incomplete copy.

### Invariant 4

A source item is never automatically marked successfully imported before the
destination commit succeeds. Explicit manual declarations are separately
labeled and never count as successful copy or verification evidence.

### Invariant 5

Potential duplicates are never silently discarded.

### Invariant 6

Existing destination media is never overwritten silently.

### Invariant 7

The UI thread never performs expensive media I/O.

### Invariant 8

The user can preview final destination paths before import.

### Invariant 9

Duplicate detection during normal camera connection does not require rescanning the entire library.

### Invariant 10

Import rules are deterministic.

---

# 63. Product Character

The intended product is not:

> a lightweight Lightroom.

It is:

> a modern, extremely fast camera-ingest tool that reliably moves media from removable devices into a clean, predictable filesystem.

The application should feel closer to a specialized system utility than a creative application.

Its strongest differentiators should be:

- native performance;
- simple operation;
- transparent filesystem organization;
- safe imports;
- excellent duplicate handling;
- deterministic renaming;
- flexible date/time grouping;
- camera and memory-card support;
- minimal persistent state;
- no catalog lock-in.

The ideal interaction is:

```text
plug camera in
↓
new media appears
↓
previous imports are already identified
↓
destination preview looks correct
↓
Import
↓
done
```

Everything else should support that workflow rather than compete with it.

---

# 64. Browser Behavior

The application opens to an empty source browser. Users can choose a local directory with **Open folder**. A **10k-item demo** source is available for exercising selection and scrolling without a camera. Deterministic `--demo` modes cover empty, camera, importing, errors, and 10,000-item states for UI review; a fixture run uses an in-memory catalog and serves a decodable preview per item, so fixtures never touch real import history and always populate the grid.

Filesystem scanning is recursive and read-only. Hidden entries and common system folders are skipped by default, symbolic links are not followed, and files rejected by the active media rules (see section 8) are omitted. The selected source's relative paths are preserved. Changing sources cancels the previous scan and discards its late results.

Camera/PTP/MTP enumeration accepts recognized image, RAW, and video extensions regardless of folder, including media under `Pictures/`, `Movies/`, or `DCIM/`. It skips non-media files (including sidecars) before showing them in the browser. Folder traversal remains recursive so camera media outside `DCIM/` is not lost. Explicitly opened ordinary filesystem folders retain the broader unknown-file behavior above.

The browser shows progressively populated media tiles in a virtualized grid. RAW+JPEG and video+sidecar pairs appear as one expandable capture with member selection in its tree panel. A tile overlays its media-type badge at the thumbnail's top-right and its file size at the bottom-left, with the source file's modification time in a compact format below the filename; camera items without a local file omit the time. The filename and metadata caption carries its own surface — the `card` tone when unselected and the `selected` tone when selected — so the text never sits directly on the page canvas. The status line below that is icon plus text (`✓ Imported`, `! Possible duplicate`, `? Unknown`, `New`, `Checking…`); for a file classified as imported or a possible duplicate it also names the prior import that matched, as a session number and date, so an uncertain classification is traceable rather than asserted. Selection is shown by the tile background, the tile border, and a `Selected` / `Part selected` label drawn on the image over the same black scrim as the media-type badge. The label never uses a themed surface: a card-coloured chip measures 1.19:1 against a bright frame and disappears over a blown-out photograph, while white ink on the 60% black scrim holds 5.7:1 against the brightest possible frame. New items are selected by default. The user can toggle a single-file tile, select all visible items, select all visible new items, or clear the visible selection. Bulk selection preserves explicit bundle-member choices, including members hidden by the current filter; unavailable previews remain counted in selection and listed in the import preview. Filters include All, Photos, Videos, New, Imported, and Possible duplicates; sorts are Capture time and Name. The footer reports discovered file count, selected file count, and selected bytes.

The browser has a time-gap slider with 5, 15, 30, 60, 120, 240, 480, and 1440 minute stops. Choosing a stop enables time-gap grouping for the import preset and persists it. Visible captures are sectioned into galleries using the same corrected timestamp and strictly-greater-than threshold rule as the import planner. Gallery sections are based on the complete scanned capture sequence, so filtering does not create artificial boundaries. Each gallery has an editable display name stored separately in `gallery_names.json` under the XDG configuration directory. Until renamed, a gallery's default name is the creation date of its earliest media (`YYYY-MM-DD`), not a session number; when several galleries share a date, they are suffixed `a`, `b`, `c`, … in chronological order. Stored names that match the old auto-generated `Session N` pattern are ignored so they fall back to the date. Clicking a gallery's display name selects every visible media item in that gallery, or clears them when all are already selected (bundle members are included). Rename focuses the gallery name field immediately; Save keeps the editor open
and reports an error if persistence fails. Display names enter destination templates only through the explicit
`{session_name}` segment; otherwise they are display-only, and actual output
folders continue to come from the import preset's photo/video destination rules
and their date/time/session variables. Import previews remain authoritative for the exact paths and session numbers of the selected import subset.

The thumbnail grid adapts its column count to the available window width and
keeps image previews and labels within their tiles. Card image height also
shrinks with shorter windows, leaving room for the card's filename and status.
Every card keeps a visible media-type badge after its thumbnail loads; video
and still formats are clearly distinguished. File size appears alongside the
timestamp beneath the image. Image previews preserve the complete frame inside
a neutral well. Selected and partly selected captures have explicit text labels
in addition to the selection border. Import status remains visible on bundled
captures, with a separate disclosure for their member files.
A five-step slider in the browser changes the preferred thumbnail size; the
actual displayed size adjusts to fit the window. Long filenames are truncated
in the grid while sidebar action labels remain fully visible. A header
toggle switches between light and dark appearance across all views, and the
Settings screen repeats that choice in its Appearance section. Four color
schemes are available: Pine, Darkroom, Graphite, and Ink. Each defines a
complete light and a complete dark palette, so the scheme and the light/dark
mode are independent choices. The active scheme drives every visual surface,
including the neutral header, primary buttons, and secondary button surfaces. Scheme, mode,
and thumbnail size are saved separately from import presets in `ui.json` under
the XDG configuration directory. Pine is the default for a fresh configuration,
and a `ui.json` written before color schemes existed keeps its saved mode and
thumbnail size while using Pine.
The desktop interface uses bundled Spectral for its display voice and bundled
Adwaita Sans for the interface, for consistent typography across Linux
installations. Spectral, an OFL-licensed serif, sets the wordmark and every
page, panel, section, and empty-state heading; Adwaita Sans sets controls,
navigation, filenames, metadata, and labels. A compact 48-pixel neutral header
carries the Spectral wordmark lockup; outlined secondary buttons, quiet filter
tabs, and a 52-pixel selection footer keep the media grid central. The sidebar
is 232 pixels wide, reducing to 184 pixels below a 760-pixel window width.
Interactive controls use a 4-pixel radius; settings forms keep a readable
maximum width. Existing shortcuts, source actions, planning, copying,
verification, and recovery behavior are unchanged by the visual treatment.

Spacing uses shared logical-pixel roles: 4 for tight detail groups, 8 for
adjacent controls, 16 for content gutters and media gaps, and 24 between
sections. Buttons, source rows, filter tabs, and text fields have a 36-pixel
minimum height. Buttons and fields use 16-pixel horizontal padding; compact
filter tabs use 8. Header, sidebar, content, and footer gutters align at 16
pixels. A control label truncates rather than overlapping its own outline, so a
narrow rail never clips a button. Unavailable actions do not leave empty gaps.
Media metadata and gallery headings reserve consistent space in the virtualized
row geometry, and the column calculation accounts for both media gaps and the
scrollbar gutter.

Controls carry Material Symbols Outlined (Material 3) action icons, vendored
with their license under `crates/captureport/assets/icons/`. An icon is tinted
from its control's text color, so the four schemes and both modes need no
per-scheme icon assets. Icons mark actions and destinations: buttons, rail
navigation, and destructive controls. Selector chips — filter, sort, preset, and
appearance — remain text-only. The documented import-status text markers (`✓ Imported`, `! Possible
duplicate`, `● New`) remain text rather than becoming glyphs. Action icons render
at 16 pixels beside 14-pixel control text, and at 12 pixels inside a 12-pixel
media-type badge, where the badge pairs its icon with a `VIDEO`, `RAW+JPEG`, or
`SIDECAR` label.

Every vertically scrollable surface shows a palette-matched scrollbar at its
right edge when content exceeds the viewport: the Sources sidebar, media grid,
expanded media group, import preview, history, recovery, settings, and timezone
menu. The thumb reflects the visible fraction and current position. Users can
drag it or click the track to navigate; wheel scrolling remains available.
Scrollbar space is reserved so controls and media are not covered by the thumb.

### Text contrast floors

Every text and surface pairing the interface renders is asserted at **4.5:1**,
the WCAG AA floor for normal-size text, across all four schemes in both modes.
The interface has no large-text pairing, so no paragraph carries the 3:1
allowance. Two pairings are pinned by dedicated tests because they are the ones
a palette retune is most likely to break silently:

- The **import-preview status chip** ("Import blocked") is filled with the
  `border` token and uses ink, not muted text. Muted text on that fill measures
  4.27:1 in Ink/light, below the floor; ink measures 7.5:1 at worst. A test reads
  the chip's own ink choice rather than a lookalike pair.
- **Image overlays** (the media-type badge and the selection label) carry their
  own black scrim instead of a themed surface, so they are legible over any
  frame. White ink on the 60% black scrim holds 5.7:1 against a pure white
  photograph, the worst case a frame can present; the card-surface treatment it
  replaces measured 1.19:1 against a bright frame in light mode.

**Known gap — control boundaries.** The outline of a button, chip or text field
measures 1.44–1.56:1 against its own surface, below the 3:1 that WCAG 1.4.11 asks
of a component whose outline is the only marker of its extent. This is recorded
rather than fixed because raising the boundary role to 3:1 collides with the
accent focus ring: the two would sit 1.25:1 apart in Darkroom/light, so the focus
indicator would stop reading as a change of state. The correct fix raises both
together — a stronger resting outline *and* a focus indicator that still clears
3:1 against it — which is a change to the palette's signal colours, not a
contrast-token adjustment. Region separators (panel, tile and footer hairlines)
are decorative and are deliberately exempt; they are asserted only at 1.2:1.

Editable text fields in import settings and gallery names use the active light
or dark palette for their surface, text, border, selection, and focus state.
Clicking positions the cursor, dragging or Shift-click selects text, and
Ctrl+V/C/X pastes, copies, or cuts. Single-line fields replace pasted line
breaks with spaces and scroll horizontally to keep the cursor visible.

Shortcuts are Ctrl+O to open a folder, Ctrl+D for the demo, Ctrl+A to select visible items, Ctrl+Shift+A to select visible new items, Ctrl+I to open the import preview (and confirm only while reviewing it), and Escape to clear the visible selection on the browser. The import action first builds a preview in the background; a separate confirmation starts copying from that exact plan.
The browser footer shows the selected count and size with a Preview import action when items are selected. The preview names blocked destinations and only offers Confirm import when every planned item can execute. After an import finishes, the same selection cannot be confirmed again until the selection or settings change and a fresh plan is built.

Import settings are editable in the app and saved as `preset.json` under the XDG configuration directory. The screen groups preset/destinations and filename first, followed by import safety, media discovery, capture time, backup copies, source alias, and appearance. It exposes separate photo/video roots and folder templates, a filename template, verification, grouping, collision and bundle policies, clock correction, and optional backup roots. Appearance and source alias save immediately; import settings require the persistent **Apply settings** button at the bottom of the settings view. A blank pair of backup roots disables backup. Skipped collision items do not block other ready copies; the preview counts them separately. Camera/card aliases are stored in the catalog and shown when the device reconnects.

Recovery, history, reconciliation, thumbnail-cache clearing, and post-import source deletion are separate actions. Recovery cleanup only offers CapturePort-owned partial files, and deleting one requires confirmation. Source deletion requires a second explicit confirmation after a verified filesystem import, states how many originals and how many bytes will be removed and from which source, offers an explicit cancel, and compares the source's full content hash with every planned destination before removing it.

---

# 65. Linux Build Distribution

Every GitHub branch push and tag push produces downloadable x86_64 Linux build
artifacts: a release executable, a portable archive, a Debian package, an Arch
pacman package, and an RPM package. Distribution packages are built on their
respective distro environments and declare runtime system dependencies;
portable builds also require compatible host graphics and libgphoto2 libraries.
Workflow artifacts include SHA-256 checksums and are retained for 30 days.
Numeric release tags supply package versions; other builds carry the crate
version, workflow run number, and commit identifier. The workflow supports
manual runs and uploads artifacts without creating a GitHub Release.
