use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartialFile {
    pub path: PathBuf,
    pub size: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RecoveryReport {
    pub partials: Vec<PartialFile>,
    pub errors: Vec<String>,
}

/// Find only CapturePort-owned temporary files. Inspection never removes media.
pub fn inspect_partials(roots: &[PathBuf]) -> RecoveryReport {
    let mut report = RecoveryReport::default();
    let mut pending = roots.to_vec();
    while let Some(path) = pending.pop() {
        let metadata = match fs::symlink_metadata(&path) {
            Ok(value) => value,
            Err(error) => {
                report.errors.push(format!("{}: {error}", path.display()));
                continue;
            }
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            match fs::read_dir(&path) {
                Ok(entries) => {
                    pending.extend(entries.filter_map(Result::ok).map(|entry| entry.path()))
                }
                Err(error) => report.errors.push(format!("{}: {error}", path.display())),
            }
        } else if metadata.is_file() && is_captureport_partial(&path) {
            report.partials.push(PartialFile {
                path,
                size: metadata.len(),
            });
        }
    }
    report.partials.sort_by(|a, b| a.path.cmp(&b.path));
    report
}

/// A separate, explicit cleanup action for a file returned by inspection.
pub fn cleanup_partial(root: &Path, candidate: &Path) -> io::Result<()> {
    if !is_captureport_partial(candidate) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Not a CapturePort partial file",
        ));
    }
    let canonical_root = fs::canonicalize(root)?;
    let canonical_parent = fs::canonicalize(
        candidate
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Missing parent"))?,
    )?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Partial is outside destination root",
        ));
    }
    if !fs::symlink_metadata(candidate)?.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Partial is not a regular file",
        ));
    }
    fs::remove_file(candidate)
}

fn is_captureport_partial(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| {
            name.starts_with(".captureport-")
                && name.ends_with(".partial")
                && name
                    .trim_end_matches(".partial")
                    .rsplit_once('.')
                    .is_some_and(|(_, id)| uuid::Uuid::parse_str(id).is_ok())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inspection_and_cleanup_ignore_unrelated_partials() {
        let root = tempfile::tempdir().unwrap();
        let owned = root
            .path()
            .join(".captureport-photo.00000000-0000-4000-8000-000000000001.partial");
        let foreign = root.path().join("other.partial");
        fs::write(&owned, b"incomplete").unwrap();
        fs::write(&foreign, b"keep").unwrap();
        let report = inspect_partials(&[root.path().into()]);
        assert_eq!(report.partials.len(), 1);
        assert_eq!(report.partials[0].path, owned);
        assert!(cleanup_partial(root.path(), &foreign).is_err());
        cleanup_partial(root.path(), &owned).unwrap();
        assert!(!owned.exists());
        assert!(foreign.exists());
    }
}
