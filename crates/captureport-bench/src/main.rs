//! Reproducible, dependency-light performance probes for the ingest layers.
//!
//! This is intentionally a normal binary instead of a statistical benchmark
//! framework.  It exercises the same public APIs used by the application and
//! emits one machine-readable TSV row per operation, making it useful in CI
//! and on modest machines.

use captureport_catalog::{
    CatalogHandle, CatalogPath, MediaIdentity, MediaType as CatalogMediaType,
};
use captureport_core::{
    FakeMediaSource, MediaItem, MediaSource, MediaType, ScanContext, ScanGeneration, SourceId,
};
use captureport_ingest::{
    ImportPlanner, ImportPreset, PlanInput, Template, TemplateContext, digest_reader, quick_path,
};
use captureport_metadata::{ThumbnailPipeline, ThumbnailRequest};
use chrono::{FixedOffset, TimeZone};
use std::{
    fs,
    hint::black_box,
    path::Path,
    time::{Duration, Instant},
};

const PHOTO_COUNT: usize = 10_000;
const VIDEO_COUNT: usize = 500;
const ITEM_COUNT: usize = PHOTO_COUNT + VIDEO_COUNT;
const VIRTUAL_TOTAL_BYTES: u64 = 1_099_511_627_776;

#[derive(Clone, Copy)]
struct Sample {
    name: &'static str,
    elapsed: Duration,
    units: usize,
    bytes: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("operation\tseconds\titems\tbytes\titems_per_second\tbytes_per_second");

    let (source, items, enumeration) = benchmark_enumeration()?;
    print_sample(enumeration);
    print_sample(benchmark_catalog(&items)?);
    print_sample(benchmark_templates(&items)?);
    print_sample(benchmark_planner(&source, &items)?);
    print_sample(benchmark_thumbnails(&items)?);
    let (quick, full) = benchmark_hashing()?;
    print_sample(quick);
    print_sample(full);
    Ok(())
}

fn benchmark_enumeration()
-> Result<(FakeMediaSource, Vec<MediaItem>, Sample), Box<dyn std::error::Error>> {
    let source = FakeMediaSource::with_id(SourceId(7), ITEM_COUNT).with_item_size(1024);
    let scan = ScanContext::new(
        ScanGeneration(1),
        captureport_core::CancellationToken::new(),
    );
    let started = Instant::now();
    let mut items = Vec::with_capacity(ITEM_COUNT);
    source.enumerate(&scan, &mut |mut item| {
        // Keep the source's deterministic enumeration cost while shaping the
        // virtual card to the benchmark's documented 10,000/500 mix.
        if items.len() >= PHOTO_COUNT {
            item.media_type = MediaType::Video;
        } else {
            item.media_type = MediaType::Jpeg;
        }
        item.size = VIRTUAL_TOTAL_BYTES / ITEM_COUNT as u64;
        items.push(item);
        Ok(())
    })?;
    let elapsed = started.elapsed();
    debug_assert_eq!(items.len(), ITEM_COUNT);
    Ok((
        source,
        items,
        Sample {
            name: "fake_enumeration_10k_photos_500_videos_1tb",
            elapsed,
            units: ITEM_COUNT,
            bytes: 0,
        },
    ))
}

fn benchmark_catalog(items: &[MediaItem]) -> Result<Sample, Box<dyn std::error::Error>> {
    let catalog = CatalogHandle::open(CatalogPath::Memory)?;
    let source = catalog.upsert_source(
        captureport_catalog::SourceIdentity {
            stable_id: Some("bench:card".into()),
            source_type: "fake".into(),
            manufacturer: Some("CapturePort".into()),
            model: Some("Benchmark card".into()),
            serial: None,
            volume_uuid: None,
            alias: None,
        },
        "2026-01-01T00:00:00Z",
    )?;
    let started = Instant::now();
    for item in items {
        catalog.upsert_media(
            MediaIdentity {
                source_id: source.id,
                source_path: item.source_path.clone(),
                source_filename: item.source_name.clone(),
                source_size: item.size,
                capture_time: Some("2026-01-01T00:00:00Z".into()),
            },
            match item.media_type {
                MediaType::Video => CatalogMediaType::Video,
                MediaType::Raw => CatalogMediaType::Raw,
                MediaType::Sidecar => CatalogMediaType::Sidecar,
                _ => CatalogMediaType::Photo,
            },
            "2026-01-01T00:00:00Z",
        )?;
    }
    for item in items {
        black_box(catalog.lookup_media(MediaIdentity {
            source_id: source.id,
            source_path: item.source_path.clone(),
            source_filename: item.source_name.clone(),
            source_size: item.size,
            capture_time: Some("2026-01-01T00:00:00Z".into()),
        })?);
    }
    Ok(Sample {
        name: "catalog_upsert_and_lookup",
        elapsed: started.elapsed(),
        units: items.len() * 2,
        bytes: 0,
    })
}

fn benchmark_templates(items: &[MediaItem]) -> Result<Sample, Box<dyn std::error::Error>> {
    let template = Template::parse("{year}/{date}/{date}_{time}_{sequence:04}.{extension}")?;
    let time = FixedOffset::east_opt(0)
        .expect("UTC")
        .with_ymd_and_hms(2026, 1, 1, 12, 0, 0)
        .single()
        .expect("valid timestamp");
    let started = Instant::now();
    for (index, item) in items.iter().enumerate() {
        let extension = Path::new(&item.source_name)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        black_box(template.render(&TemplateContext {
            timestamp: time,
            camera: "Benchmark",
            camera_make: "CapturePort",
            camera_model: "Synthetic",
            camera_serial: "",
            original_name: &item.source_name,
            original_stem: &item.source_name,
            extension,
            media_type: if item.media_type == MediaType::Video {
                "video"
            } else {
                "photo"
            },
            sequence: index as u64 + 1,
            session: 1,
            metadata: None,
            file_size: item.size,
            source_name: "Benchmark",
        }));
    }
    Ok(Sample {
        name: "template_generation",
        elapsed: started.elapsed(),
        units: items.len(),
        bytes: 0,
    })
}

fn benchmark_planner(
    source: &FakeMediaSource,
    items: &[MediaItem],
) -> Result<Sample, Box<dyn std::error::Error>> {
    let destination = tempfile::tempdir()?;
    let mut preset = ImportPreset::everyday(destination.path());
    preset.photo.root = destination.path().to_path_buf();
    preset.video.root = destination.path().to_path_buf();
    let time = FixedOffset::east_opt(0)
        .expect("UTC")
        .with_ymd_and_hms(2026, 1, 1, 12, 0, 0)
        .single()
        .expect("valid timestamp");
    let input = items
        .iter()
        .cloned()
        .map(|item| PlanInput {
            item,
            capture_time: time,
        })
        .collect();
    let started = Instant::now();
    let plan = ImportPlanner::build(source, input, &preset);
    black_box(plan.items.len());
    Ok(Sample {
        name: "planner_10k_photos_500_videos",
        elapsed: started.elapsed(),
        units: items.len(),
        bytes: 0,
    })
}

fn benchmark_thumbnails(items: &[MediaItem]) -> Result<Sample, Box<dyn std::error::Error>> {
    let cache = tempfile::tempdir()?;
    let pipeline = ThumbnailPipeline::new(cache.path(), 4, 128)?;
    let started = Instant::now();
    let mut completed = 0;
    for (index, item) in items.iter().enumerate() {
        let request = ThumbnailRequest {
            source_id: "bench:card".into(),
            relative_path: item.source_path.clone(),
            size: item.size,
            modified_unix: 1,
            path: cache.path().join(&item.source_name),
            media_type: MediaType::Sidecar,
            priority: if index < 64 { 3 } else { 0 },
        };
        loop {
            match pipeline.submit(request.clone()) {
                Ok(()) => break,
                Err(captureport_metadata::ThumbnailError::QueueFull) => {
                    if pipeline.try_recv().is_some() {
                        completed += 1;
                    } else {
                        std::thread::yield_now();
                    }
                }
                Err(error) => return Err(Box::new(error)),
            }
        }
    }
    while completed < items.len() {
        if pipeline.try_recv().is_some() {
            completed += 1;
        } else {
            std::thread::yield_now();
        }
    }
    Ok(Sample {
        name: "thumbnail_queue_placeholder_jobs",
        elapsed: started.elapsed(),
        units: completed,
        bytes: 0,
    })
}

fn benchmark_hashing() -> Result<(Sample, Sample), Box<dyn std::error::Error>> {
    let path = tempfile::NamedTempFile::new()?;
    let bytes = vec![0x5a; 8 * 1024 * 1024];
    fs::write(path.path(), &bytes)?;
    let quick_started = Instant::now();
    black_box(quick_path(path.path())?);
    let quick = Sample {
        name: "quick_hash_8mib",
        elapsed: quick_started.elapsed(),
        units: 1,
        bytes: bytes.len() as u64,
    };

    let full_started = Instant::now();
    black_box(digest_reader(fs::File::open(path.path())?)?.full());
    let full = Sample {
        name: "full_hash_8mib",
        elapsed: full_started.elapsed(),
        units: 1,
        bytes: bytes.len() as u64,
    };
    Ok((quick, full))
}

fn print_sample(sample: Sample) {
    let seconds = sample.elapsed.as_secs_f64();
    let item_rate = sample.units as f64 / seconds.max(f64::MIN_POSITIVE);
    let byte_rate = sample.bytes as f64 / seconds.max(f64::MIN_POSITIVE);
    println!(
        "{}\t{seconds:.6}\t{}\t{}\t{item_rate:.1}\t{byte_rate:.1}",
        sample.name, sample.units, sample.bytes
    );
}
