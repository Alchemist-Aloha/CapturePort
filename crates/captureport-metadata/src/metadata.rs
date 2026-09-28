use captureport_core::{MediaMetadata, MediaType, Orientation};
use exif::{In, Reader, Tag, Value};
use std::{
    fs::File,
    io::BufReader,
    path::Path,
    process::Command,
    time::{Duration, Instant, SystemTime},
};

pub use captureport_core::TimestampSource;

#[derive(Clone, Debug, PartialEq)]
pub struct NormalizedMetadata {
    pub capture_time: Option<String>,
    pub filesystem_time: Option<String>,
    pub timestamp_source: Option<TimestampSource>,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub camera_serial: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub orientation: Option<Orientation>,
    pub duration_millis: Option<u64>,
    pub lens: Option<String>,
    pub gps: Option<(f64, f64)>,
    pub media_type: MediaType,
}

impl NormalizedMetadata {
    pub fn core_metadata(&self) -> MediaMetadata {
        MediaMetadata {
            capture_time: self.capture_time.clone(),
            filesystem_time: self.filesystem_time.clone(),
            timestamp_source: self.timestamp_source,
            camera_make: self.camera_make.clone(),
            camera_model: self.camera_model.clone(),
            camera_serial: self.camera_serial.clone(),
            width: self.width,
            height: self.height,
            orientation: self.orientation,
            duration_millis: self.duration_millis,
            lens: self.lens.clone(),
            gps_e7: self.gps.map(|(lat, lon)| {
                (
                    (lat * 10_000_000.0).round() as i32,
                    (lon * 10_000_000.0).round() as i32,
                )
            }),
        }
    }
}

#[derive(Debug)]
pub enum MetadataError {
    Io(String),
    Parse(String),
    Unsupported(String),
}
impl std::fmt::Display for MetadataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for MetadataError {}

pub struct MetadataService;

impl MetadataService {
    pub fn extract(
        path: impl AsRef<Path>,
        media_type: MediaType,
    ) -> Result<NormalizedMetadata, MetadataError> {
        let path = path.as_ref();
        let filesystem_time = std::fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok())
            .map(format_system_time);
        let mut result = NormalizedMetadata {
            capture_time: None,
            filesystem_time,
            timestamp_source: None,
            camera_make: None,
            camera_model: None,
            camera_serial: None,
            width: None,
            height: None,
            orientation: None,
            duration_millis: None,
            lens: None,
            gps: None,
            media_type,
        };
        if matches!(
            media_type,
            MediaType::Jpeg | MediaType::Tiff | MediaType::Raw
        ) && let Ok(file) = File::open(path)
        {
            let mut reader = BufReader::new(file);
            if let Ok(exif) = Reader::new().read_from_container(&mut reader) {
                read_exif(&exif, &mut result);
            }
        }
        if media_type == MediaType::Video
            && let Some((duration, width, height, timestamp)) = ffprobe(path)
        {
            result.duration_millis = duration;
            result.width = width;
            result.height = height;
            if timestamp.is_some() {
                result.capture_time = timestamp;
                result.timestamp_source = Some(TimestampSource::QuickTime);
            }
        }
        if result.capture_time.is_none() {
            result.capture_time = result.filesystem_time.clone();
            result.timestamp_source = result
                .capture_time
                .as_ref()
                .map(|_| TimestampSource::Filesystem);
        }
        Ok(result)
    }
}

type ProbeResult = (Option<u64>, Option<u32>, Option<u32>, Option<String>);
fn ffprobe(path: &Path) -> Option<ProbeResult> {
    let output = run_command(
        "ffprobe",
        &[
            "-v",
            "error",
            "-show_entries",
            "format=duration:format_tags=creation_time:stream=width,height",
            "-of",
            "default=noprint_wrappers=1",
            path.to_str()?,
        ],
        Duration::from_secs(5),
        16 * 1024,
    )
    .ok()?;
    let mut duration = None;
    let mut width = None;
    let mut height = None;
    let mut timestamp = None;
    for line in String::from_utf8_lossy(&output).lines() {
        if let Some(value) = line.strip_prefix("duration=") {
            duration = value
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite() && *v >= 0.0)
                .map(|v| (v * 1000.0).round() as u64);
        }
        if let Some(value) = line.strip_prefix("width=") {
            width = value.parse().ok();
        }
        if let Some(value) = line.strip_prefix("height=") {
            height = value.parse().ok();
        }
        if let Some(value) = line.strip_prefix("TAG:creation_time=") {
            timestamp = Some(value.to_owned());
        }
    }
    Some((duration, width, height, timestamp))
}

fn run_command(
    program: &str,
    args: &[&str],
    timeout: Duration,
    max_output: usize,
) -> Result<Vec<u8>, MetadataError> {
    let mut child = Command::new(program)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| MetadataError::Unsupported(e.to_string()))?;
    let start = Instant::now();
    loop {
        if child
            .try_wait()
            .map_err(|e| MetadataError::Io(e.to_string()))?
            .is_some()
        {
            break;
        }
        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(MetadataError::Parse("ffprobe timed out".into()));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let mut output = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        use std::io::Read;
        stdout
            .take((max_output + 1) as u64)
            .read_to_end(&mut output)
            .map_err(|e| MetadataError::Io(e.to_string()))?;
    }
    if output.len() > max_output {
        return Err(MetadataError::Parse("ffprobe output exceeded limit".into()));
    }
    Ok(output)
}

fn read_exif(exif: &exif::Exif, out: &mut NormalizedMetadata) {
    out.camera_make = text(exif, Tag::Make);
    out.camera_model = text(exif, Tag::Model);
    out.camera_serial = text(exif, Tag::BodySerialNumber);
    out.lens = text(exif, Tag::LensModel).or_else(|| text(exif, Tag::LensMake));
    out.width = number(exif, Tag::PixelXDimension).or_else(|| number(exif, Tag::ImageWidth));
    out.height = number(exif, Tag::PixelYDimension).or_else(|| number(exif, Tag::ImageLength));
    out.orientation = number(exif, Tag::Orientation).and_then(|n| match n {
        1 => Some(Orientation::Normal),
        2 => Some(Orientation::MirrorHorizontal),
        3 => Some(Orientation::Rotate180),
        4 => Some(Orientation::MirrorVertical),
        6 => Some(Orientation::Rotate90),
        8 => Some(Orientation::Rotate270),
        _ => None,
    });
    if let Some(raw) =
        text(exif, Tag::DateTimeOriginal).or_else(|| text(exif, Tag::DateTimeDigitized))
    {
        // EXIF has no timezone. Keep its wall-clock value explicit and stable.
        let mut normalized = raw;
        if normalized.len() >= 10 {
            normalized.replace_range(4..5, "-");
            normalized.replace_range(7..8, "-");
        }
        if chrono::NaiveDateTime::parse_from_str(&normalized, "%Y-%m-%d %H:%M:%S").is_ok() {
            out.capture_time = Some(normalized);
            out.timestamp_source = Some(TimestampSource::ExifOriginal);
        }
    }
    if let (Some(lat), Some(lon)) = (
        gps(
            exif,
            Tag::GPSLatitude,
            Tag::GPSLatitudeRef,
            b'N',
            b'S',
            90.0,
        ),
        gps(
            exif,
            Tag::GPSLongitude,
            Tag::GPSLongitudeRef,
            b'E',
            b'W',
            180.0,
        ),
    ) {
        out.gps = Some((lat, lon));
    }
}

fn text(exif: &exif::Exif, tag: Tag) -> Option<String> {
    exif.get_field(tag, In::PRIMARY)
        .map(|f| {
            f.display_value()
                .with_unit(f)
                .to_string()
                .trim()
                .trim_matches('"')
                .to_owned()
        })
        .filter(|s| !s.is_empty())
}
fn number(exif: &exif::Exif, tag: Tag) -> Option<u32> {
    exif.get_field(tag, In::PRIMARY)
        .and_then(|f| match &f.value {
            Value::Short(v) => v.first().copied().map(u32::from),
            Value::Long(v) => v.first().copied(),
            Value::Byte(v) => v.first().copied().map(u32::from),
            _ => None,
        })
}
fn gps(
    exif: &exif::Exif,
    tag: Tag,
    reference: Tag,
    positive: u8,
    negative: u8,
    maximum: f64,
) -> Option<f64> {
    let coordinate = &exif.get_field(tag, In::PRIMARY)?.value;
    let reference = &exif.get_field(reference, In::PRIMARY)?.value;
    gps_coordinate(coordinate, reference, positive, negative, maximum)
}

fn gps_coordinate(
    coordinate: &Value,
    reference: &Value,
    positive: u8,
    negative: u8,
    maximum: f64,
) -> Option<f64> {
    let Value::Rational(parts) = coordinate else {
        return None;
    };
    let Value::Ascii(reference) = reference else {
        return None;
    };
    // EXIF degrees are unsigned: no hemisphere means the sign is unknown.
    let [hemisphere] = reference.first()?.as_slice() else {
        return None;
    };
    let sign = if *hemisphere == positive {
        1.0
    } else if *hemisphere == negative {
        -1.0
    } else {
        return None;
    };
    if parts.len() != 3 || parts.iter().any(|part| part.denom == 0) {
        return None;
    }
    let degrees = parts[0].to_f64();
    let minutes = parts[1].to_f64();
    let seconds = parts[2].to_f64();
    if minutes >= 60.0 || seconds >= 60.0 {
        return None;
    }
    let value = degrees + minutes / 60.0 + seconds / 3600.0;
    (value.is_finite() && value <= maximum).then_some(sign * value)
}

fn format_system_time(t: SystemTime) -> String {
    let dt: chrono::DateTime<chrono::Utc> = t.into();
    dt.to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coordinate(degrees: u32, minutes: u32, seconds: u32) -> Value {
        Value::Rational(
            [degrees, minutes, seconds]
                .into_iter()
                .map(|num| exif::Rational { num, denom: 1 })
                .collect(),
        )
    }

    #[test]
    fn gps_hemispheres_and_coordinate_validation() {
        let latitude = coordinate(33, 30, 0);
        let longitude = coordinate(151, 15, 0);
        let reference = |value: &[u8]| Value::Ascii(vec![value.to_vec()]);
        assert_eq!(
            gps_coordinate(&latitude, &reference(b"N"), b'N', b'S', 90.0),
            Some(33.5)
        );
        assert_eq!(
            gps_coordinate(&latitude, &reference(b"S"), b'N', b'S', 90.0),
            Some(-33.5)
        );
        assert_eq!(
            gps_coordinate(&longitude, &reference(b"E"), b'E', b'W', 180.0),
            Some(151.25)
        );
        assert_eq!(
            gps_coordinate(&longitude, &reference(b"W"), b'E', b'W', 180.0),
            Some(-151.25)
        );
        for invalid in [b"".as_slice(), b"E", b"?", b"North"] {
            assert_eq!(
                gps_coordinate(&latitude, &reference(invalid), b'N', b'S', 90.0),
                None
            );
        }
        assert_eq!(
            gps_coordinate(&latitude, &Value::Ascii(vec![]), b'N', b'S', 90.0),
            None
        );
        for invalid in [
            coordinate(91, 0, 0),
            coordinate(90, 0, 1),
            coordinate(33, 60, 0),
            coordinate(33, 0, 60),
            Value::Rational(vec![exif::Rational { num: 1, denom: 0 }; 3]),
        ] {
            assert_eq!(
                gps_coordinate(&invalid, &reference(b"N"), b'N', b'S', 90.0),
                None
            );
        }
    }

    #[test]
    fn exif_gps_reads_reference_tags_and_requires_both_axes() {
        // Little-endian TIFF with a GPS IFD: refs inline, three rationals per axis.
        let exif = |latitude_ref: u8, longitude_ref: u8| {
            let mut data = b"II\x2a\x00\x08\x00\x00\x00".to_vec();
            data.extend_from_slice(&1u16.to_le_bytes());
            for value in [0x8825u16, 4] {
                data.extend_from_slice(&value.to_le_bytes());
            }
            data.extend_from_slice(&1u32.to_le_bytes());
            data.extend_from_slice(&26u32.to_le_bytes());
            data.extend_from_slice(&0u32.to_le_bytes());
            data.extend_from_slice(&4u16.to_le_bytes());
            for (tag, kind, count, value) in [
                (1u16, 2u16, 2u32, u32::from(latitude_ref)),
                (2, 5, 3, 80),
                (3, 2, 2, u32::from(longitude_ref)),
                (4, 5, 3, 104),
            ] {
                data.extend_from_slice(&tag.to_le_bytes());
                data.extend_from_slice(&kind.to_le_bytes());
                data.extend_from_slice(&count.to_le_bytes());
                data.extend_from_slice(&value.to_le_bytes());
            }
            data.extend_from_slice(&0u32.to_le_bytes());
            for num in [33u32, 30, 0, 151, 15, 0] {
                data.extend_from_slice(&num.to_le_bytes());
                data.extend_from_slice(&1u32.to_le_bytes());
            }
            Reader::new().read_raw(data).unwrap()
        };
        let sample = exif(b'S', b'W');
        assert_eq!(
            gps(
                &sample,
                Tag::GPSLatitude,
                Tag::GPSLatitudeRef,
                b'N',
                b'S',
                90.0
            ),
            Some(-33.5)
        );
        assert_eq!(
            gps(
                &sample,
                Tag::GPSLongitude,
                Tag::GPSLongitudeRef,
                b'E',
                b'W',
                180.0
            ),
            Some(-151.25)
        );
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut normalized = MetadataService::extract(file.path(), MediaType::Tiff).unwrap();
        read_exif(&sample, &mut normalized);
        assert_eq!(normalized.gps, Some((-33.5, -151.25)));
        assert_eq!(
            normalized.core_metadata().gps_e7,
            Some((-335_000_000, -1_512_500_000))
        );
        let uncertain = exif(0, b'W');
        normalized.gps = None;
        read_exif(&uncertain, &mut normalized);
        assert_eq!(normalized.gps, None);
        assert_eq!(
            gps(
                &uncertain,
                Tag::GPSLatitude,
                Tag::GPSLatitudeRef,
                b'N',
                b'S',
                90.0
            ),
            None
        );
    }

    #[test]
    fn core_conversion_preserves_common_fields() {
        let n = NormalizedMetadata {
            capture_time: Some("2026-01-01 10:00:00".into()),
            filesystem_time: None,
            timestamp_source: Some(TimestampSource::ExifOriginal),
            camera_make: Some("Acme".into()),
            camera_model: None,
            camera_serial: None,
            width: Some(12),
            height: Some(8),
            orientation: Some(Orientation::Rotate90),
            duration_millis: None,
            lens: None,
            gps: None,
            media_type: MediaType::Jpeg,
        };
        let c = n.core_metadata();
        assert_eq!(c.width, Some(12));
        assert_eq!(c.orientation, Some(Orientation::Rotate90));
    }

    #[test]
    fn video_probe_reads_duration_when_ffmpeg_tools_are_available() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.mp4");
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=16x12:d=0.2",
                "-y",
            ])
            .arg(&path)
            .status();
        if status.is_err() || !status.unwrap().success() || !path.exists() {
            return;
        }
        let metadata = MetadataService::extract(&path, MediaType::Video).unwrap();
        assert!(metadata.duration_millis.is_some());
        assert_eq!(metadata.width, Some(16));
        assert_eq!(metadata.height, Some(12));
    }
}
