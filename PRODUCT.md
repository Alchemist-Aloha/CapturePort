# CapturePort Product

<!-- impeccable:product-schema 1 -->

## Platform

Native Linux desktop (GPUI).

## Users

Photographers importing photographs and videos from cameras, memory cards, mounted storage, or folders.

## Product Purpose

CapturePort helps photographers move new media into a filesystem library through a predictable, reviewable import. Success means the selected files reach their planned destinations, pass verification, and appear in import history while originals remain available.

## Positioning

CapturePort is a focused ingest utility. A single deterministic plan shows the final destinations before copying and drives the import itself. The application does not require a proprietary library or take on photo catalog, editing, or general file management work.

## Operating Context

A photographer connects a camera or card, or opens a source folder; reviews discovered media and prior import status; selects files and an import preset; previews destination paths; confirms the import; and checks completion or recovery if interrupted. The app supports PTP cameras, removable storage, and folders. Settings and history are stored under XDG directories; the media library remains on the filesystem.

## Capabilities and Constraints

- The import engine is independent of GPUI and accesses every source through `MediaSource`.
- Discovery is read only. Source files are never deleted automatically.
- Destination collisions are resolved during planning, and files are copied to owned partial paths, verified, and atomically published before a successful history record.
- Prior imports are checked progressively; uncertain media is not treated as safely imported.
- Interrupted work remains identifiable and recoverable. Cleanup and post import source deletion require separate explicit actions.
- Fast startup, low idle resource use, and strong keyboard navigation are product goals.

## Evidence on Hand

- [Product and behavior contract](docs/spec.md)
- [Implementation plan](docs/plan.md)
- [Current workflow and development commands](README.md)
- The application provides deterministic demo states for empty, camera, importing, error, and large source views. These are fixtures, not evidence of physical camera or compositor validation.

## Product Principles

1. Let photographers review exact outcomes before committing an import.
2. Keep originals safe and make incomplete work unmistakable.
3. Treat the filesystem as the lasting library and the database as supporting state.
4. Keep the routine import path fast, clear, and keyboard accessible.
