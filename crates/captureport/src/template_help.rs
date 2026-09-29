//! Illustrative template examples. Actual import destinations come from ImportPlan.
use captureport_core::{MediaMetadata, Orientation, TimestampSource};
use captureport_ingest::{Template, TemplateContext, TemplateError};
use chrono::{FixedOffset, TimeZone};

pub fn example(source: &str, filename: bool, video: bool) -> Result<String, TemplateError> {
    let metadata = MediaMetadata {
        capture_time: Some("2026-09-27T14:20:33+00:00".into()),
        filesystem_time: Some("2026-09-27T15:00:00+00:00".into()),
        timestamp_source: Some(if video {
            TimestampSource::QuickTime
        } else {
            TimestampSource::ExifOriginal
        }),
        camera_make: Some("Sony".into()),
        camera_model: Some("A7C II".into()),
        camera_serial: Some("123456".into()),
        width: Some(if video { 3840 } else { 6000 }),
        height: Some(if video { 2160 } else { 4000 }),
        orientation: Some(Orientation::Normal),
        duration_millis: video.then_some(12500),
        lens: Some("FE 35mm F1.8".into()),
        gps_e7: Some((377749000, -1224194000)),
    };
    let context = TemplateContext {
        timestamp: FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2026, 9, 27, 14, 20, 33)
            .unwrap(),
        camera: "A7C II",
        camera_make: "Sony",
        camera_model: "A7C II",
        camera_serial: "123456",
        original_name: if video { "DSC0001.MP4" } else { "DSC0001.ARW" },
        original_stem: "DSC0001",
        extension: if video { "MP4" } else { "ARW" },
        media_type: if video { "video" } else { "photo" },
        sequence: 1,
        session: 2,
        session_name: "Iceland trip",
        metadata: Some(&metadata),
        file_size: 24000000,
        source_name: "Camera card",
    };
    let template = Template::parse(source)?;
    if filename {
        template.render_filename(&context)
    } else {
        template.render_relative_path(&context)
    }
}
