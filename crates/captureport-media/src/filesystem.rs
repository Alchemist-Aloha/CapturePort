use captureport_core::{
    MediaId, MediaItem, MediaLocator, MediaSource, ScanContext, SourceError, SourceId,
    SourceIdentity, SourceType,
};
use std::{
    collections::hash_map::DefaultHasher,
    fs::{self, File},
    hash::{Hash, Hasher},
    io::{Read, Seek, SeekFrom},
    path::{Component, Path, PathBuf},
};

/// A read-only, recursive source rooted at a user-selected directory.
pub struct FilesystemSource {
    root: PathBuf,
    identity: SourceIdentity,
    include_hidden: bool,
}

impl FilesystemSource {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, SourceError> {
        let root = fs::canonicalize(path.as_ref()).map_err(io_error)?;
        if !root.is_dir() {
            return Err(SourceError::Io(format!(
                "Not a directory: {}",
                root.display()
            )));
        }
        let (stable_id, volume_uuid) = volume_identity(&root);
        let mut hasher = DefaultHasher::new();
        stable_id.hash(&mut hasher);
        let display_name = root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        Ok(Self {
            root,
            identity: SourceIdentity {
                id: SourceId(hasher.finish()),
                source_type: SourceType::Filesystem,
                stable_id: Some(stable_id),
                serial: None,
                manufacturer: None,
                model: None,
                volume_uuid,
                display_name,
            },
            include_hidden: false,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn with_hidden(mut self, include_hidden: bool) -> Self {
        self.include_hidden = include_hidden;
        self
    }

    fn item_path(&self, locator: &MediaLocator) -> Result<PathBuf, SourceError> {
        let relative = Path::new(&locator.0);
        if relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(SourceError::MissingItem(locator.0.clone()));
        }
        let path = self.root.join(relative);
        // A file could become a symlink after enumeration. Reject it and avoid
        // following a link outside the selected source.
        let canonical = fs::canonicalize(&path).map_err(io_error)?;
        if !canonical.starts_with(&self.root) {
            return Err(SourceError::MissingItem(locator.0.clone()));
        }
        let metadata = fs::symlink_metadata(&path).map_err(io_error)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(SourceError::MissingItem(locator.0.clone()));
        }
        Ok(path)
    }
}

/// Resolve a mounted directory to the most stable identity Linux exposes.
/// The path fallback keeps arbitrary folders useful while removable media
/// uses its filesystem UUID whenever `/dev/disk/by-uuid` is available.
fn volume_identity(root: &Path) -> (String, Option<String>) {
    let mut best: Option<(PathBuf, PathBuf)> = None;
    let Ok(mounts) = fs::read_to_string("/proc/self/mounts") else {
        return (format!("filesystem:{}", root.display()), None);
    };
    for line in mounts.lines() {
        let mut fields = line.split_whitespace();
        let Some(device) = fields.next().map(unescape_mount) else {
            continue;
        };
        let Some(mountpoint) = fields.next().map(unescape_mount) else {
            continue;
        };
        if !root.starts_with(&mountpoint)
            || best.as_ref().is_some_and(|(current, _)| {
                mountpoint.components().count() <= current.components().count()
            })
        {
            continue;
        }
        best = Some((mountpoint, device));
    }
    let Some((mountpoint, device)) = best else {
        return (format!("filesystem:{}", root.display()), None);
    };
    let location = root.strip_prefix(&mountpoint).unwrap_or(root);
    if let Some(uuid) = volume_uuid_for_device(&device) {
        return (
            format!("filesystem:uuid:{uuid}:{}", location.display()),
            Some(uuid),
        );
    }
    if device.starts_with("/dev/") {
        return (
            format!(
                "filesystem:device:{}:{}",
                device.display(),
                location.display()
            ),
            None,
        );
    }
    (format!("filesystem:{}", root.display()), None)
}

fn volume_uuid_for_device(device: &Path) -> Option<String> {
    let canonical_device = fs::canonicalize(device).ok()?;
    let entries = fs::read_dir("/dev/disk/by-uuid").ok()?;
    entries.filter_map(Result::ok).find_map(|entry| {
        let target = fs::canonicalize(entry.path()).ok()?;
        (target == canonical_device).then(|| entry.file_name().to_string_lossy().into_owned())
    })
}

fn unescape_mount(value: &str) -> PathBuf {
    PathBuf::from(
        value
            .replace("\\040", " ")
            .replace("\\011", "\t")
            .replace("\\134", "\\"),
    )
}

impl MediaSource for FilesystemSource {
    fn identity(&self) -> SourceIdentity {
        self.identity.clone()
    }

    fn enumerate(
        &self,
        scan: &ScanContext,
        emit: &mut dyn FnMut(MediaItem) -> Result<(), SourceError>,
    ) -> Result<(), SourceError> {
        let mut pending = vec![self.root.clone()];
        let mut next_id = 0u64;
        while let Some(directory) = pending.pop() {
            if scan.is_cancelled() {
                return Err(SourceError::Cancelled);
            }
            for entry in fs::read_dir(directory).map_err(io_error)? {
                if scan.is_cancelled() {
                    return Err(SourceError::Cancelled);
                }
                let entry = entry.map_err(io_error)?;
                let name = entry.file_name();
                if !self.include_hidden
                    && (name.to_string_lossy().starts_with('.') || is_system_directory(&name))
                {
                    continue;
                }
                let file_type = entry.file_type().map_err(io_error)?;
                if file_type.is_symlink() {
                    continue;
                }
                let path = entry.path();
                if file_type.is_dir() {
                    pending.push(path);
                } else if file_type.is_file() {
                    let relative = path
                        .strip_prefix(&self.root)
                        .map_err(|error| SourceError::Io(error.to_string()))?;
                    let relative = relative.to_str().ok_or_else(|| {
                        SourceError::Io(format!("Non-UTF-8 path: {}", path.display()))
                    })?;
                    let size = entry.metadata().map_err(io_error)?.len();
                    next_id = next_id
                        .checked_add(1)
                        .ok_or_else(|| SourceError::Io("Too many media items".into()))?;
                    emit(MediaItem::new(
                        MediaId(next_id),
                        self.identity.id,
                        relative,
                        size,
                    ))?;
                }
            }
        }
        Ok(())
    }

    fn open_stream(&self, item: &MediaLocator) -> Result<Box<dyn Read + Send>, SourceError> {
        Ok(Box::new(
            File::open(self.item_path(item)?).map_err(io_error)?,
        ))
    }

    fn read_range(
        &self,
        item: &MediaLocator,
        offset: u64,
        length: usize,
    ) -> Result<Vec<u8>, SourceError> {
        let mut file = File::open(self.item_path(item)?).map_err(io_error)?;
        file.seek(SeekFrom::Start(offset)).map_err(io_error)?;
        let mut result = Vec::with_capacity(length.min(1024 * 1024));
        file.take(length as u64)
            .read_to_end(&mut result)
            .map_err(io_error)?;
        Ok(result)
    }
}

fn is_system_directory(name: &std::ffi::OsStr) -> bool {
    matches!(
        name.to_str(),
        Some("System Volume Information" | "$RECYCLE.BIN")
    )
}

fn io_error(error: std::io::Error) -> SourceError {
    SourceError::Io(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use captureport_core::{CancellationToken, MediaType, ScanGeneration};
    use std::io::Write;

    fn scan() -> ScanContext {
        ScanContext::new(ScanGeneration(1), CancellationToken::new())
    }

    #[test]
    fn folders_on_one_volume_have_distinct_source_identities() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("a")).unwrap();
        fs::create_dir_all(dir.path().join("b")).unwrap();
        let a = FilesystemSource::new(dir.path().join("a")).unwrap();
        let b = FilesystemSource::new(dir.path().join("b")).unwrap();
        assert_ne!(a.identity().stable_id, b.identity().stable_id);
    }

    #[test]
    fn recursively_emits_relative_paths_and_unknown_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("DCIM/100MEDIA")).unwrap();
        fs::create_dir_all(dir.path().join(".Trash")).unwrap();
        File::create(dir.path().join("DCIM/100MEDIA/DSC0001.ARW"))
            .unwrap()
            .write_all(b"raw")
            .unwrap();
        File::create(dir.path().join("DCIM/100MEDIA/C0001.MP4")).unwrap();
        File::create(dir.path().join("DCIM/100MEDIA/notes.xyz")).unwrap();
        File::create(dir.path().join(".Trash/hidden.jpg")).unwrap();
        let source = FilesystemSource::new(dir.path()).unwrap();
        let mut found = Vec::new();
        source
            .enumerate(&scan(), &mut |item| {
                found.push(item);
                Ok(())
            })
            .unwrap();
        assert_eq!(found.len(), 3);
        assert!(
            found
                .iter()
                .any(|item| item.source_path == "DCIM/100MEDIA/DSC0001.ARW"
                    && item.media_type == MediaType::Raw
                    && item.size == 3)
        );
        assert!(found.iter().any(|item| item.media_type == MediaType::Video));
        assert!(
            found
                .iter()
                .any(|item| item.media_type == MediaType::Unknown)
        );
        assert_eq!(
            source
                .read_range(&MediaLocator("DCIM/100MEDIA/DSC0001.ARW".into()), 1, 2)
                .unwrap(),
            b"aw"
        );
    }

    #[test]
    fn cancellation_stops_stream_and_rejects_path_escape() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..20 {
            File::create(dir.path().join(format!("{i}.jpg"))).unwrap();
        }
        let source = FilesystemSource::new(dir.path()).unwrap();
        let scan = scan();
        let token = scan.cancellation().clone();
        let mut count = 0;
        let result = source.enumerate(&scan, &mut |_| {
            count += 1;
            token.cancel();
            Ok(())
        });
        assert_eq!(result, Err(SourceError::Cancelled));
        assert_eq!(count, 1);
        assert!(
            source
                .open_stream(&MediaLocator("../escape".into()))
                .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn does_not_follow_links_during_scan_or_open() {
        use std::os::unix::fs::symlink;
        let source_dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        File::create(outside.path().join("outside.jpg")).unwrap();
        symlink(outside.path(), source_dir.path().join("linked-dir")).unwrap();
        symlink(
            outside.path().join("outside.jpg"),
            source_dir.path().join("linked.jpg"),
        )
        .unwrap();
        let source = FilesystemSource::new(source_dir.path()).unwrap();
        let mut count = 0;
        source
            .enumerate(&scan(), &mut |_| {
                count += 1;
                Ok(())
            })
            .unwrap();
        assert_eq!(count, 0);
        assert!(
            source
                .open_stream(&MediaLocator("linked.jpg".into()))
                .is_err()
        );
        assert!(
            source
                .open_stream(&MediaLocator("linked-dir/outside.jpg".into()))
                .is_err()
        );
    }
}
