# CapturePort 1.0.0

CapturePort 1.0.0 is the first stable release. Since v0.5.0 the work has gone
into three things: imports you can trust with your only copy, settings that are
easier to get right, and packages that are easier to install.

## Highlights

- **Keep browsing while an import runs.** Imports run in the background. You can
  open other cards, folders or cameras, and review their media, while one copies.
  Only the camera being imported stays locked. The import panel shows which
  source it is copying from, and its result names that source if you have moved on.
- **Sessions that span midnight stay together.** When photos are grouped by time
  gap, a session's folder uses the date of its first shot, so a shoot from 23:00
  to 01:00 lands in one folder. File names still use each file's own date.
- **Settings reworked.**
  - Options are shown as labelled choices with a short description of what the
    selected verification, collision and bundle option does.
  - Text fields apply when you leave them. The Apply button is gone, and an
    invalid value keeps the previous setting and shows why.
  - Folder pickers for the destination and backup roots.
  - Time zones and clock corrections can be entered as `UTC+08:00` or
    `+H:MM:SS`, with a worked example.
  - Ignored media types are toggles instead of free text.
  - A wider session-gap slider, from 5 minutes to 30 days.
- **Destination folders are created for you.** If the destination root does not
  exist yet, the preview creates it. It refuses when the folder would land on the
  wrong disk, for example under `/media` or `/mnt` while the card or drive is not
  mounted.
- **AppImage and clearer downloads.** Releases now include an AppImage alongside
  the `.deb`, `.rpm`, Arch package and portable archive. Every download follows
  one naming pattern and says which distribution it is for. The release page
  opens with a table of what to download.

## Safer imports

- An existing file at the destination that looks the same as the source is now
  only a candidate. Before it counts as already imported, CapturePort compares
  full hashes, whatever verification mode you chose.
- Verification rejects destination files that change while being checked, and
  destinations that are not regular files.
- Deleting originals from the card is only offered for files whose copies met the
  verification policy. When none qualify, the delete controls are not shown.
- Free-space checks follow symlinked destination roots and add up all copies
  going to the same disk.
- Cameras that report a USB serial number keep the same identity when they are
  plugged into a different port, so their history and names carry over.
- Camera operations can be cancelled while libgphoto2 is busy, and camera folder
  errors are reported instead of skipped.
- A settings file that cannot be read is set aside as `*.json.invalid` instead of
  being overwritten.

## Usability

- The preview warns when Settings has changes that it is not using yet.
- Renaming a session waits until the scan finishes. The button says so instead of
  failing silently.
- Thumbnail loading shows progress. Media without a preview still appears, with a
  placeholder tile.
- Camera spooling and existing-file checks show their own progress phases.
- Session history shows readable times, counts and status.
- Recovery lists each partial file once, with readable details.
- History rows, settings choices and preset actions can be reached by keyboard,
  and focus no longer gets lost when buttons change or disappear.
- The sidebar preset label opens the Presets tab.
- Text fields support selection, word movement and undo/redo.

## Translations

- Corrected Japanese and Korean wording where "import" and "destination" were
  translated in the wrong sense.
- Fixed ungrammatical deletion and cancellation counts in most languages.
  Native-speaker review is still welcome.

## Packaging and build

- New `scripts/build-appimage.sh`. The AppImage bundles libgphoto2's camera and
  port drivers. It needs glibc 2.39 or newer (Ubuntu 24.04 and later).
- Releases are titled "CapturePort <version>". Tags with a suffix such as
  `v1.1.0-rc.1` are published as pre-releases.
- The separate Arch debug package is no longer built.
- The one-command installer still downloads `captureport-linux-x86_64`.
- Building needs Rust 1.99.1 or newer.

## Upgrading

No manual steps. Saved settings, presets, session names and import history carry
over. Saved session names from earlier versions are migrated automatically.
