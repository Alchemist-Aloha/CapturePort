use crate::VerificationMode;
use captureport_core::{MediaLocator, MediaSource, SourceError};
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::Path,
};

pub const QUICK_ALGORITHM: &str = "quick-blake3-v1";
pub const FULL_ALGORITHM: &str = "blake3-v1";
const SAMPLE: usize = 256 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fingerprint {
    pub algorithm: &'static str,
    pub hex: String,
}

/// Accumulates verification data while the source is copied in one pass.
pub struct CopyDigest {
    full: blake3::Hasher,
    first: Vec<u8>,
    last: Vec<u8>,
    small: Option<Vec<u8>>,
    size: u64,
}

impl Default for CopyDigest {
    fn default() -> Self {
        Self::new()
    }
}

impl CopyDigest {
    pub fn new() -> Self {
        Self {
            full: blake3::Hasher::new(),
            first: Vec::with_capacity(SAMPLE),
            last: Vec::with_capacity(SAMPLE),
            small: Some(Vec::new()),
            size: 0,
        }
    }

    pub fn update(&mut self, bytes: &[u8]) {
        self.full.update(bytes);
        self.size += bytes.len() as u64;
        if self.first.len() < SAMPLE {
            self.first
                .extend_from_slice(&bytes[..bytes.len().min(SAMPLE - self.first.len())]);
        }
        if let Some(small) = &mut self.small {
            if self.size <= (SAMPLE * 2) as u64 {
                small.extend_from_slice(bytes);
            } else {
                self.small = None;
            }
        }
        if bytes.len() >= SAMPLE {
            self.last.clear();
            self.last.extend_from_slice(&bytes[bytes.len() - SAMPLE..]);
        } else {
            let excess = self
                .last
                .len()
                .saturating_add(bytes.len())
                .saturating_sub(SAMPLE);
            if excess > 0 {
                self.last.drain(..excess);
            }
            self.last.extend_from_slice(bytes);
        }
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn quick(&self) -> Fingerprint {
        let mut hasher = blake3::Hasher::new();
        hasher.update(QUICK_ALGORITHM.as_bytes());
        hasher.update(&self.size.to_le_bytes());
        if let Some(small) = &self.small {
            hasher.update(small);
        } else {
            hasher.update(&self.first);
            hasher.update(&self.last);
        }
        Fingerprint {
            algorithm: QUICK_ALGORITHM,
            hex: hasher.finalize().to_hex().to_string(),
        }
    }

    pub fn full(&self) -> Fingerprint {
        Fingerprint {
            algorithm: FULL_ALGORITHM,
            hex: self.full.finalize().to_hex().to_string(),
        }
    }
}

pub fn digest_reader(mut reader: impl Read) -> io::Result<CopyDigest> {
    let mut digest = CopyDigest::new();
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(digest)
}

pub fn quick_source(
    source: &dyn MediaSource,
    locator: &MediaLocator,
    size: u64,
) -> Result<Fingerprint, SourceError> {
    if size <= (SAMPLE * 2) as u64 {
        let whole = source.read_range(locator, 0, size as usize)?;
        if whole.len() != size as usize {
            return Err(SourceError::Io(
                "Source changed while fingerprinting".into(),
            ));
        }
        Ok(quick_samples(size, &whole, &[]))
    } else {
        let first = source.read_range(locator, 0, SAMPLE)?;
        if first.len() != SAMPLE {
            return Err(SourceError::Io(
                "Source changed while fingerprinting".into(),
            ));
        }
        let last = source.read_range(locator, size - SAMPLE as u64, SAMPLE)?;
        if last.len() != SAMPLE {
            return Err(SourceError::Io(
                "Source changed while fingerprinting".into(),
            ));
        }
        Ok(quick_samples(size, &first, &last))
    }
}

pub fn quick_path(path: &Path) -> io::Result<Fingerprint> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    if size <= (SAMPLE * 2) as u64 {
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        return Ok(quick_samples(size, &bytes, &[]));
    }
    let mut first = vec![0; SAMPLE];
    file.read_exact(&mut first)?;
    file.seek(SeekFrom::Start(size - SAMPLE as u64))?;
    let mut last = vec![0; SAMPLE];
    file.read_exact(&mut last)?;
    Ok(quick_samples(size, &first, &last))
}

fn quick_samples(size: u64, first: &[u8], last: &[u8]) -> Fingerprint {
    let mut hasher = blake3::Hasher::new();
    hasher.update(QUICK_ALGORITHM.as_bytes());
    hasher.update(&size.to_le_bytes());
    hasher.update(first);
    hasher.update(last);
    Fingerprint {
        algorithm: QUICK_ALGORITHM,
        hex: hasher.finalize().to_hex().to_string(),
    }
}

pub fn verify_copy(
    mode: VerificationMode,
    source: &CopyDigest,
    destination: &Path,
) -> io::Result<bool> {
    let metadata = destination.metadata()?;
    if metadata.len() != source.size() {
        return Ok(false);
    }
    if mode == VerificationMode::Fast {
        return Ok(true);
    }
    let destination = digest_reader(File::open(destination)?)?;
    Ok(match mode {
        VerificationMode::Fast => true,
        VerificationMode::Standard => source.quick() == destination.quick(),
        VerificationMode::Strict => source.full() == destination.full(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_and_file_digest_agree_at_sample_boundaries() {
        for length in [
            0,
            1,
            SAMPLE - 1,
            SAMPLE,
            SAMPLE * 2,
            SAMPLE * 2 + 1,
            SAMPLE * 4,
        ] {
            let bytes: Vec<u8> = (0..length).map(|index| (index % 251) as u8).collect();
            let mut first = CopyDigest::new();
            for chunk in bytes.chunks(17_121) {
                first.update(chunk);
            }
            let second = digest_reader(bytes.as_slice()).unwrap();
            assert_eq!(first.quick(), second.quick());
            assert_eq!(first.full(), second.full());
        }
    }

    #[test]
    fn strict_verification_catches_same_size_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("copy");
        let data = b"photograph";
        let source = digest_reader(data.as_slice()).unwrap();
        std::fs::write(&file, data).unwrap();
        assert!(verify_copy(VerificationMode::Strict, &source, &file).unwrap());
        std::fs::write(&file, b"xhotograph").unwrap();
        assert!(verify_copy(VerificationMode::Fast, &source, &file).unwrap());
        assert!(!verify_copy(VerificationMode::Standard, &source, &file).unwrap());
        assert!(!verify_copy(VerificationMode::Strict, &source, &file).unwrap());
    }
}
