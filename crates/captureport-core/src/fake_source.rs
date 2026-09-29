use crate::{
    MediaItem, MediaLocator, MediaSource, ScanContext, SourceError, SourceId, SourceIdentity,
    SourceType,
};
use std::{
    io::{self, Read},
    time::Duration,
};

/// Repeatable conditions used by demos and integration tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FakeSourceScenario {
    NormalCamera,
    LargeCard,
    SlowCard,
    DuplicateCard,
    ReformattedCamera,
    DisconnectingCamera,
    CorruptSource,
    TimestampBrokenCamera,
}

/// Builder for deterministic sources that exercise the same edge cases as
/// removable media, without requiring a camera or a large fixture on disk.
#[derive(Clone, Debug)]
pub struct FakeSourceBuilder {
    scenario: FakeSourceScenario,
    source_id: SourceId,
    item_count: usize,
    bytes_per_item: u64,
    read_latency: Option<Duration>,
    disconnect_after_items: Option<usize>,
    disconnect_after_bytes: Option<u64>,
    corrupt_after_bytes: Option<u64>,
}

impl FakeSourceBuilder {
    pub fn new() -> Self {
        Self {
            scenario: FakeSourceScenario::NormalCamera,
            source_id: SourceId(1),
            item_count: 100,
            bytes_per_item: 1024,
            read_latency: None,
            disconnect_after_items: None,
            disconnect_after_bytes: None,
            corrupt_after_bytes: None,
        }
    }

    pub fn scenario(mut self, scenario: FakeSourceScenario) -> Self {
        self.scenario = scenario;
        match scenario {
            FakeSourceScenario::LargeCard => self.item_count = 10_000,
            FakeSourceScenario::SlowCard => self.read_latency = Some(Duration::from_millis(2)),
            FakeSourceScenario::DisconnectingCamera => {
                self.disconnect_after_items = Some(25);
                self.disconnect_after_bytes = Some(4096);
            }
            FakeSourceScenario::CorruptSource => self.corrupt_after_bytes = Some(128),
            FakeSourceScenario::TimestampBrokenCamera
            | FakeSourceScenario::NormalCamera
            | FakeSourceScenario::DuplicateCard
            | FakeSourceScenario::ReformattedCamera => {}
        }
        self
    }

    pub fn source_id(mut self, source_id: SourceId) -> Self {
        self.source_id = source_id;
        self
    }

    pub fn files(mut self, item_count: usize) -> Self {
        self.item_count = item_count;
        self
    }

    pub fn bytes_per_item(mut self, bytes: u64) -> Self {
        self.bytes_per_item = bytes;
        self
    }

    pub fn read_latency(mut self, latency: Duration) -> Self {
        self.read_latency = Some(latency);
        self
    }

    pub fn disconnect_after_items(mut self, count: usize) -> Self {
        self.disconnect_after_items = Some(count);
        self
    }

    pub fn disconnect_after_bytes(mut self, bytes: u64) -> Self {
        self.disconnect_after_bytes = Some(bytes);
        self
    }

    pub fn corrupt_after_bytes(mut self, bytes: u64) -> Self {
        self.corrupt_after_bytes = Some(bytes);
        self
    }

    pub fn build(self) -> FakeMediaSource {
        let stable_prefix = match self.scenario {
            FakeSourceScenario::DuplicateCard => "fake:duplicate",
            FakeSourceScenario::ReformattedCamera => "fake:reformatted",
            _ => "fake",
        };
        FakeMediaSource {
            identity: SourceIdentity {
                id: self.source_id,
                source_type: SourceType::Fake,
                stable_id: Some(format!("{stable_prefix}:{}", self.source_id.0)),
                serial: None,
                manufacturer: Some("CapturePort".into()),
                model: Some("Synthetic source".into()),
                volume_uuid: None,
                display_name: Some(format!("Demo · {:?}", self.scenario)),
            },
            scenario: self.scenario,
            item_count: self.item_count,
            bytes_per_item: self.bytes_per_item,
            read_latency: self.read_latency,
            disconnect_after_items: self.disconnect_after_items,
            disconnect_after_bytes: self.disconnect_after_bytes,
            corrupt_after_bytes: self.corrupt_after_bytes,
            seed: match self.scenario {
                FakeSourceScenario::ReformattedCamera => 0xA5,
                FakeSourceScenario::DuplicateCard => 0,
                _ => 0,
            },
        }
    }
}

impl Default for FakeSourceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Deterministic, allocation-light source for UI and domain development.
#[derive(Clone, Debug)]
pub struct FakeMediaSource {
    identity: SourceIdentity,
    scenario: FakeSourceScenario,
    item_count: usize,
    bytes_per_item: u64,
    read_latency: Option<Duration>,
    disconnect_after_items: Option<usize>,
    disconnect_after_bytes: Option<u64>,
    corrupt_after_bytes: Option<u64>,
    seed: u8,
}
impl FakeMediaSource {
    pub fn new(item_count: usize) -> Self {
        FakeSourceBuilder::new().files(item_count).build()
    }
    pub fn with_id(source_id: SourceId, item_count: usize) -> Self {
        FakeSourceBuilder::new()
            .source_id(source_id)
            .files(item_count)
            .build()
    }
    pub fn with_item_size(mut self, size: u64) -> Self {
        self.bytes_per_item = size;
        self
    }
    fn path(index: usize) -> String {
        let n = index / 4 + 1;
        let extension = match index % 4 {
            0 => "ARW",
            1 => "JPG",
            2 => "MP4",
            _ => "XMP",
        };
        format!("DCIM/100MEDIA/DSC{n:05}.{extension}")
    }
}
impl MediaSource for FakeMediaSource {
    fn identity(&self) -> SourceIdentity {
        self.identity.clone()
    }
    fn enumerate(
        &self,
        scan: &ScanContext,
        emit: &mut dyn FnMut(MediaItem) -> Result<(), SourceError>,
    ) -> Result<(), SourceError> {
        for index in 0..self.item_count {
            if scan.is_cancelled() {
                return Err(SourceError::Cancelled);
            }
            let item = MediaItem::new(
                crate::MediaId(index as u64 + 1),
                self.identity.id,
                Self::path(index),
                self.bytes_per_item,
            );
            if self
                .disconnect_after_items
                .is_some_and(|limit| index >= limit)
            {
                return Err(SourceError::Io("synthetic source disconnected".into()));
            }
            let mut item = item;
            if self.scenario == FakeSourceScenario::TimestampBrokenCamera {
                item.metadata =
                    crate::MetadataState::Failed("synthetic camera timestamp is invalid".into());
            }
            emit(item)?;
        }
        Ok(())
    }
    /// A deterministic image per item so the demo fixtures drive the real
    /// thumbnail pipeline. Without this a fixture can only ever render its
    /// empty state, which is not what "camera" or "10,000 items" means.
    ///
    /// The format is uncompressed 24-bit BMP: no encoder dependency, decodable
    /// by the same `image` path a camera preview uses.
    fn preview(&self, item: &MediaLocator) -> Result<Option<Vec<u8>>, SourceError> {
        Ok(Some(synthetic_preview(self.index(item)?)))
    }
    fn open_stream(&self, item: &MediaLocator) -> Result<Box<dyn Read + Send>, SourceError> {
        let index = self.index(item)?;
        if self.disconnect_after_bytes == Some(0) {
            return Err(SourceError::Io("synthetic source disconnected".into()));
        }
        Ok(Box::new(FakeReader {
            remaining: self.bytes_per_item,
            seed: (index as u8).wrapping_add(self.seed),
            position: 0,
            read_latency: self.read_latency,
            disconnect_after_bytes: self.disconnect_after_bytes,
            corrupt_after_bytes: self.corrupt_after_bytes,
        }))
    }
    fn read_range(
        &self,
        item: &MediaLocator,
        offset: u64,
        length: usize,
    ) -> Result<Vec<u8>, SourceError> {
        let index = self.index(item)?;
        if self
            .disconnect_after_bytes
            .is_some_and(|limit| offset >= limit)
        {
            return Err(SourceError::Io("synthetic source disconnected".into()));
        }
        if self
            .disconnect_after_bytes
            .is_some_and(|limit| offset.saturating_add(length as u64) > limit)
        {
            return Err(SourceError::Io("synthetic source disconnected".into()));
        }
        if let Some(latency) = self.read_latency {
            std::thread::sleep(latency);
        }
        let remaining = self
            .bytes_per_item
            .saturating_sub(offset)
            .min(length as u64) as usize;
        Ok((0..remaining)
            .map(|i| {
                let position = offset as usize + i;
                let mut byte = position
                    .wrapping_add(index)
                    .wrapping_add(self.seed as usize) as u8;
                if self
                    .corrupt_after_bytes
                    .is_some_and(|limit| position as u64 >= limit)
                {
                    byte ^= 0xFF;
                }
                byte
            })
            .collect())
    }
}
impl FakeMediaSource {
    fn index(&self, locator: &MediaLocator) -> Result<usize, SourceError> {
        (0..self.item_count)
            .find(|index| Self::path(*index) == locator.0)
            .ok_or_else(|| SourceError::MissingItem(locator.0.clone()))
    }
}
struct FakeReader {
    remaining: u64,
    seed: u8,
    position: u64,
    read_latency: Option<Duration>,
    disconnect_after_bytes: Option<u64>,
    corrupt_after_bytes: Option<u64>,
}
impl Read for FakeReader {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if self
            .disconnect_after_bytes
            .is_some_and(|limit| self.position >= limit)
        {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "synthetic source disconnected",
            ));
        }
        if let Some(latency) = self.read_latency {
            std::thread::sleep(latency);
        }
        let mut count = output
            .len()
            .min(self.remaining.min(usize::MAX as u64) as usize);
        if let Some(limit) = self.disconnect_after_bytes {
            count = count.min(limit.saturating_sub(self.position) as usize);
        }
        for (i, byte) in output[..count].iter_mut().enumerate() {
            *byte = self
                .seed
                .wrapping_add(self.position.wrapping_add(i as u64) as u8);
            if self
                .corrupt_after_bytes
                .is_some_and(|limit| self.position + i as u64 >= limit)
            {
                *byte ^= 0xFF;
            }
        }
        self.remaining -= count as u64;
        self.position += count as u64;
        Ok(count)
    }
}

/// Build one deterministic 64x48 BMP. The ramp is seeded by item index, so a
/// contact sheet of these is visually distinct tile by tile while every run
/// produces identical bytes.
pub(crate) fn synthetic_preview(index: usize) -> Vec<u8> {
    const WIDTH: usize = 64;
    const HEIGHT: usize = 48;
    const FILE_HEADER: usize = 14;
    const INFO_HEADER: usize = 40;
    let row_bytes = WIDTH * 3;
    let pixel_bytes = row_bytes * HEIGHT;
    let mut out = Vec::with_capacity(FILE_HEADER + INFO_HEADER + pixel_bytes);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&((FILE_HEADER + INFO_HEADER + pixel_bytes) as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&((FILE_HEADER + INFO_HEADER) as u32).to_le_bytes());
    out.extend_from_slice(&(INFO_HEADER as u32).to_le_bytes());
    out.extend_from_slice(&(WIDTH as i32).to_le_bytes());
    out.extend_from_slice(&(HEIGHT as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(pixel_bytes as u32).to_le_bytes());
    out.extend_from_slice(&2835i32.to_le_bytes());
    out.extend_from_slice(&2835i32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    // BMP rows run bottom-up and store channels as blue, green, red.
    for y in (0..HEIGHT).rev() {
        for x in 0..WIDTH {
            let red = (x * 4 + index * 37) as u8;
            let green = (y * 5 + index * 53) as u8;
            let blue = (x + y + index * 11) as u8;
            out.extend_from_slice(&[blue, green, red]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CancellationToken, ScanGeneration};

    #[test]
    fn synthetic_preview_is_a_well_formed_distinct_bmp_per_item() {
        let first = synthetic_preview(0);
        let second = synthetic_preview(1);
        assert_eq!(&first[..2], b"BM");
        // Declared file size must match what we actually produced.
        let declared = u32::from_le_bytes(first[2..6].try_into().unwrap()) as usize;
        assert_eq!(declared, first.len());
        assert_eq!(first.len(), 54 + 64 * 48 * 3);
        assert_ne!(first, second, "each item needs its own preview");
        assert_eq!(
            first,
            synthetic_preview(0),
            "fixtures must be deterministic"
        );
    }

    #[test]
    fn every_fake_source_serves_a_preview_for_its_items() {
        let source = FakeMediaSource::new(3);
        let locator = MediaLocator(FakeMediaSource::path(1));
        let bytes = source.preview(&locator).unwrap().expect("fixture preview");
        assert_eq!(&bytes[..2], b"BM");
        assert!(source.preview(&MediaLocator("missing.ARW".into())).is_err());
    }
    #[test]
    fn enumerates_large_sources_incrementally_and_cancels() {
        for count in [1, 100, 10_000] {
            let source = FakeMediaSource::new(count);
            let scan = ScanContext::new(ScanGeneration(7), CancellationToken::new());
            let mut found = 0;
            source
                .enumerate(&scan, &mut |_| {
                    found += 1;
                    Ok(())
                })
                .unwrap();
            assert_eq!(found, count);
        }
        let source = FakeMediaSource::new(10_000);
        let token = CancellationToken::new();
        let scan = ScanContext::new(ScanGeneration(8), token.clone());
        let mut found = 0;
        let result = source.enumerate(&scan, &mut |_| {
            found += 1;
            if found == 25 {
                token.cancel();
            }
            Ok(())
        });
        assert_eq!(result, Err(SourceError::Cancelled));
        assert_eq!(found, 25);
    }
    #[test]
    fn fake_source_can_stream_and_read_ranges() {
        let source = FakeMediaSource::new(4).with_item_size(100);
        let loc = MediaLocator("DCIM/100MEDIA/DSC00001.ARW".into());
        // Fixtures must be able to fill the grid, so a preview is part of the
        // fake source's contract rather than an absent capability.
        let preview = source.preview(&loc).unwrap().expect("fixture preview");
        assert_eq!(&preview[..2], b"BM");
        assert_eq!(source.read_range(&loc, 95, 20).unwrap().len(), 5);
        let mut stream = source.open_stream(&loc).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        assert_eq!(data.len(), 100);
        assert_eq!(
            source.read_range(&MediaLocator("missing".into()), 0, 1),
            Err(SourceError::MissingItem("missing".into()))
        );
    }

    #[test]
    fn builder_covers_documented_failure_scenarios() {
        let large = FakeSourceBuilder::new()
            .scenario(FakeSourceScenario::LargeCard)
            .build();
        assert_eq!(large.item_count, 10_000);

        let broken = FakeSourceBuilder::new()
            .scenario(FakeSourceScenario::TimestampBrokenCamera)
            .files(1)
            .build();
        let scan = ScanContext::new(ScanGeneration(1), CancellationToken::new());
        let mut item = None;
        broken
            .enumerate(&scan, &mut |found| {
                item = Some(found);
                Ok(())
            })
            .unwrap();
        assert!(matches!(
            item.expect("one synthetic item").metadata,
            crate::MetadataState::Failed(_)
        ));

        let disconnecting = FakeSourceBuilder::new()
            .scenario(FakeSourceScenario::DisconnectingCamera)
            .files(30)
            .build();
        let mut found = 0;
        let error = disconnecting.enumerate(&scan, &mut |_| {
            found += 1;
            Ok(())
        });
        assert_eq!(found, 25);
        assert!(matches!(error, Err(SourceError::Io(_))));
    }

    #[test]
    fn reformatted_and_corrupt_sources_are_deterministic() {
        let scan = ScanContext::new(ScanGeneration(1), CancellationToken::new());
        let first = FakeSourceBuilder::new().files(1).build();
        let reformatted = FakeSourceBuilder::new()
            .scenario(FakeSourceScenario::ReformattedCamera)
            .files(1)
            .build();
        let locator = MediaLocator("DCIM/100MEDIA/DSC00001.ARW".into());
        assert_ne!(
            first.read_range(&locator, 0, 32).unwrap(),
            reformatted.read_range(&locator, 0, 32).unwrap()
        );
        let corrupt = FakeSourceBuilder::new()
            .scenario(FakeSourceScenario::CorruptSource)
            .bytes_per_item(256)
            .build();
        let mut stream = corrupt.open_stream(&locator).unwrap();
        let mut bytes = Vec::new();
        let error = stream.read_to_end(&mut bytes).unwrap();
        assert_eq!(error, 256);
        assert_ne!(bytes[128], 128);
        let mut found = 0;
        corrupt
            .enumerate(&scan, &mut |_| {
                found += 1;
                Ok(())
            })
            .unwrap();
        assert_eq!(found, 100);
    }
}
