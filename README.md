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
appear in the Sources list. Select media, choose **Preview / import**, review
every destination, then confirm the import. **Import settings** controls photo
and video destinations, path and filename templates, grouping, clock correction,
verification, bundle handling, and optional backup destinations. Settings are
stored in the XDG config directory. The XDG data directory holds the catalog;
the XDG cache directory holds disposable thumbnails.

The **History** view shows sessions and their individual destination copies.
After an interrupted import, **Recovery** lists incomplete sessions and
CapturePort-owned partial files. It can build a new preview of remaining files
from the reconnected source. Cleanup and post-import source deletion each
require a separate explicit action.

Deterministic UI fixtures are available with `--demo empty`, `--demo camera`,
`--demo importing`, `--demo errors`, and `--demo 10000`. Packaging and
benchmark commands are described in [docs/build-and-benchmarks.md](docs/build-and-benchmarks.md).

The implementation contract is [docs/spec.md](docs/spec.md); the staged build
plan is [docs/plan.md](docs/plan.md).
