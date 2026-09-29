# CapturePort

CapturePort is a Linux photo and video ingest application. It browses folders,
removable cards, and PTP cameras, then builds a reviewable plan before it copies
selected media. Copies use temporary files, verification, atomic publication,
and a SQLite import history. Originals remain in place during import.

## Develop

```sh
rtk cargo test --workspace --offline
rtk cargo run -p captureport --offline
```

Use **Open folder** for a local source. Detected cameras and removable cards
appear in the Sources list. Unmounted removable cards show a **Mount** button;
this requires `lsblk`, `udisksctl`, and the system UDisks service. Select media,
choose **Preview / import**, review
every destination, then confirm the import. **Import settings** controls photo
and video destinations, path and filename templates, grouping, clock correction,
verification, bundle handling, and optional backup destinations. Settings are
stored in the XDG config directory. The XDG data directory holds the catalog;
the XDG cache directory holds disposable thumbnails.

Folder and filename template fields include syntax help, clickable metadata
segments, and live sample examples. For example, use
`{year}/{date:%Y-%m-%d}/{camera_model}` for folders and
`{datetime:%Y-%m-%d_%H-%M-%S}_{sequence:04}.{extension}` for filenames.
Segments also cover lens, dimensions, orientation, duration, GPS, recorded
timestamps, and file size. See [Destination rules](docs/spec.md#20-destination-rules)
for formats and missing-metadata behavior. Review the import preview for the
actual final paths.

Time-gap grouping creates galleries and session numbers without adding a folder
layer. Add `{date}_{session}` to a folder template if you want session folders.

In the media browser, **Mark as imported** records selected files that you have
already imported elsewhere. Their `marked manually` status survives rescans and
restarts. This action does not copy files or add verified import history.

The **History** view shows sessions and their individual destination copies.
After an interrupted import, **Recovery** lists incomplete sessions and
CapturePort-owned partial files. It can build a new preview of remaining files
from the reconnected source. Cleanup and post-import source deletion each
require a separate explicit action.

The desktop interface embeds Adwaita Sans; its SIL Open Font License ships
with the application packages. Appearance offers Pine, Darkroom, Graphite, and
Ink schemes in both light and dark mode.

Deterministic UI fixtures are available with `--demo empty`, `--demo camera`,
`--demo importing`, `--demo errors`, and `--demo 10000`. Packaging and
benchmark commands are described in [docs/build-and-benchmarks.md](docs/build-and-benchmarks.md).

Every branch and tag push builds x86_64 Linux binaries, portable archives, and
Debian, pacman, and RPM packages in the **Linux packages** GitHub Actions workflow.
Download them from the workflow run's **Artifacts** section.

The implementation contract is [docs/spec.md](docs/spec.md); the staged build
plan is [docs/plan.md](docs/plan.md).
