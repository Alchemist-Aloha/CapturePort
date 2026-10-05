# CapturePort

CapturePort copies photos and videos from a camera, memory card, or folder into a library you choose — and shows you exactly where every file will land before it copies anything.

It is a native Linux application inspired by the classic Windows Import Pictures and Videos wizard. There is no catalog to import into and no library to lock you in: the filesystem is the library, and CapturePort's only job is to add files to it safely. It does not edit, rate, or organize media after import.

## What "safe" means here

- **You review first.** CapturePort plans the import and lists the final
  destination path of every file. Nothing is copied until you confirm.
- **Originals stay in place.** Importing never moves or deletes the source
  file; deleting from the source is a separate, deliberately confirmed action.
- **A half-finished import never looks finished.** Files are copied under
  temporary names, verified, then published atomically — and only then recorded
  in the import history.
- **Nothing is silently overwritten.** Name collisions are resolved while
  planning and shown in the preview.
- **Nothing is silently skipped.** Media that may already be in your library is flagged as a possible duplicate with the prior session it matched, so you decide, not the app.

## Install

CapturePort is distributed as Linux packages and a portable archive. Every
branch and tag push builds x86_64 artifacts for Debian/Ubuntu (`.deb`), Arch
(pacman), and Fedora (`.rpm`) in the **Linux packages** GitHub Actions workflow.
Open the workflow run and download them from the **Artifacts** section.

Runtime requirements:

- a Vulkan-capable graphics stack (GPUI's supported Linux drivers);
- `libgphoto2` for PTP cameras;
- `lsblk`, `udisksctl`, and the system UDisks service to mount unmounted cards.

## Import your photos

1. **Choose a source.** Connect a camera or insert a card and it appears under
   **Sources**; or click **Open folder…** for any directory on disk. An
   unmounted card is listed with a **Mount** button.
2. **Review what was found.** CapturePort scans the source and shows a
   thumbnail, capture time, file size, and import status for each item. Items
   already in your history are marked and left unselected, so new media is what
   remains selected. Use the **All / Photos / Videos / New / Imported /
   Possible** filters, and **View options** to sort by capture time or name and
   change the thumbnail size.
3. **Select what to copy.** Toggle individual items, use **Select all** or
   **Select new**, and click a gallery heading to select everything in one
   shooting session.
4. **Preview the plan.** **Preview import** opens the Review screen with every
   destination path and a count of files to copy, skipped, and blocked.
5. **Confirm.** Existing files, collisions, and unreadable media are resolved
   before the copy starts; each file is verified, then published.
6. **Check the result.** **History** lists the session and every destination it
   wrote.

## Sources

- **Folders** — any directory, scanned recursively and read-only.
- **Removable cards** — mounted volumes under `/media` and `/run/media` are
  discovered automatically. An unmounted card is listed with an explicit
  **Mount** button; mounting is never automatic, and your system's UDisks
  service handles authorization.
- **Cameras** — PTP cameras are detected through libgphoto2 and browsed
  directly, without mounting.

**Refresh devices** (or **Scan for cameras** in the empty view) runs an
immediate scan. A detected card is labelled by its device model and mount
directory.

## Organize where files go

Open **Settings** to control how imports are organized:

- **Presets** — enter a name and **Create preset** to save all Settings values,
  including browsing, appearance, and the source-alias field. Click a saved name
  to restore it; use **Rename**, **Overwrite**, or **Delete preset** to manage
  it. Overwrite and delete require confirmation. Loading an alias does not rename
  a connected device until you click **Save source alias**.
- **Destinations** — separate root folders for photos and videos.
- **Folder and filename templates** — build paths from your media, for example
  `{year}/{date:%Y-%m-%d}/{camera_model}` for folders and
  `{datetime:%Y-%m-%d_%H-%M-%S}_{sequence:04}.{extension}` for filenames.
  Template fields have built-in syntax help with clickable segments and live
  examples; segments cover date and time, camera and lens, dimensions,
  orientation, duration, GPS, recorded timestamps, and file size.
- **Grouping** — split an import into shooting sessions by a time gap, and use
  `{session}` (number) or `{session_name}` (the gallery name) in a folder
  template if you want a folder per session.
- **Clock correction** — shift capture times, or state the camera's assumed UTC
  offset, so files land under the date they were actually taken.
- **Verification, collisions, and bundles** — how strictly copies are checked,
  what happens on a name collision, and how RAW+JPEG pairs are handled.
- **Backup destinations** — optionally write a second copy of everything.

See [Destination rules](docs/spec.md#20-destination-rules) for the full segment
list and how missing metadata is filled in. The import preview is always the
authority on the paths you will actually get.

## History and recovery

**History** shows each import session and the individual destination copies it
made. If an import is interrupted, **Recovery** lists the unfinished sessions
and the incomplete files CapturePort owns, and can build a new preview of the
remaining files from the reconnected source. Cleaning up incomplete files and
deleting originals from a source are separate actions, each requiring its own
confirmation.

## Mark as imported

If you already imported some files elsewhere, select them and use **Mark as
imported**. This records the decision — it does not copy files or add verified
history — and the `marked manually` status survives rescans and restarts.

## Keyboard shortcuts

| Key | Action |
| --- | --- |
| `Ctrl+O` | Open a folder |
| `Ctrl+A` | Select all visible items |
| `Ctrl+Shift+A` | Select all visible new items |
| `Escape` | Clear the selection on the browser |
| `Ctrl+I` | Open the import preview (pressing it again on the Review screen starts the copy) |

## Where your data lives

CapturePort follows the XDG base directory layout:

- **Config** — import settings, appearance, and gallery names.
- **Data** — the catalog and import history.
- **Cache** — disposable thumbnails, safe to delete at any time.

The catalog only accelerates imports and remembers history; deleting it never
makes imported media unusable or hard to find.

## Appearance

CapturePort ships four color schemes — **Pine**, **Darkroom**, **Graphite**, and
**Ink** — each in light and dark mode. Toggle the mode from the header and pick
the scheme in Settings.

## Try it without a device

Deterministic demo sources are available for trying the interface or reporting
a bug: `--demo empty`, `--demo camera`, `--demo importing`, `--demo errors`, and
`--demo 10000`. A demo run uses an in-memory catalog, so it cannot write
synthetic rows into your real history.

```sh
captureport --demo camera
```

## For developers

```sh
rtk cargo test --workspace --offline
rtk cargo run -p captureport --offline
```

Builds require Rust 1.88 or newer, Cargo, a C compiler, GPUI's Linux graphics
development libraries, and the system `libgphoto2` and `libgphoto2_port`
development files. Packaging and benchmark commands are documented in
[docs/build-and-benchmarks.md](docs/build-and-benchmarks.md).

- [docs/spec.md](docs/spec.md) — the product and behavior contract.
- [docs/plan.md](docs/plan.md) — the staged build plan.
- [DESIGN.md](DESIGN.md) — the visual system.

The application is MIT licensed. The bundled Spectral and Outfit fonts and
the Material Symbols Outlined icon set ship with their own licenses in the
application packages.
