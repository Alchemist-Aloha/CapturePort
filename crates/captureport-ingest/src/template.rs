use captureport_core::{MediaMetadata, Orientation, TimestampSource};
use chrono::{
    DateTime, Datelike, FixedOffset, Timelike,
    format::{Item, StrftimeItems},
};
use std::{
    fmt,
    path::{Component, Path},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemplateError {
    UnclosedToken,
    UnknownToken(String),
    InvalidFormat(String),
    InvalidPath(String),
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnclosedToken => write!(f, "Template contains an unclosed token"),
            Self::UnknownToken(token) => write!(f, "Unknown template token: {token}"),
            Self::InvalidFormat(token) => write!(f, "Invalid template format: {token}"),
            Self::InvalidPath(path) => write!(f, "Template generates an invalid path: {path}"),
        }
    }
}
impl std::error::Error for TemplateError {}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Token {
    Year,
    Month,
    Day,
    Date,
    Time,
    Datetime,
    Camera,
    CameraMake,
    CameraModel,
    CameraSerial,
    OriginalName,
    OriginalStem,
    Extension,
    MediaType,
    Sequence(usize),
    Session(usize),
    Hour,
    Minute,
    Second,
    FormattedTime(String),
    Metadata(String),
    FileSize,
    SourceName,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Part {
    Literal(String),
    Token(Token),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Template {
    parts: Vec<Part>,
}

#[derive(Clone, Debug)]
pub struct TemplateContext<'a> {
    pub timestamp: DateTime<FixedOffset>,
    pub camera: &'a str,
    pub camera_make: &'a str,
    pub camera_model: &'a str,
    pub camera_serial: &'a str,
    pub original_name: &'a str,
    pub original_stem: &'a str,
    pub extension: &'a str,
    pub media_type: &'a str,
    pub sequence: u64,
    pub session: u32,
    pub metadata: Option<&'a MediaMetadata>,
    pub file_size: u64,
    pub source_name: &'a str,
}

/// Shared reference used by the settings editor and template parser tests.
#[derive(Clone, Copy, Debug)]
pub struct TemplateTokenHelp {
    pub group: &'static str,
    pub syntax: &'static str,
    pub description: &'static str,
}

pub const TEMPLATE_TOKENS: &[TemplateTokenHelp] = &[
    TemplateTokenHelp {
        group: "Date and time",
        syntax: "{year}",
        description: "Four-digit corrected capture year",
    },
    TemplateTokenHelp {
        group: "Date and time",
        syntax: "{month}",
        description: "Two-digit month",
    },
    TemplateTokenHelp {
        group: "Date and time",
        syntax: "{day}",
        description: "Two-digit day",
    },
    TemplateTokenHelp {
        group: "Date and time",
        syntax: "{hour}",
        description: "Two-digit hour, 00–23",
    },
    TemplateTokenHelp {
        group: "Date and time",
        syntax: "{minute}",
        description: "Two-digit minute",
    },
    TemplateTokenHelp {
        group: "Date and time",
        syntax: "{second}",
        description: "Two-digit second",
    },
    TemplateTokenHelp {
        group: "Date and time",
        syntax: "{date}",
        description: "Corrected capture date: YYYYMMDD",
    },
    TemplateTokenHelp {
        group: "Date and time",
        syntax: "{time}",
        description: "Corrected capture time: HHMMSS",
    },
    TemplateTokenHelp {
        group: "Date and time",
        syntax: "{datetime}",
        description: "Corrected date and time: YYYYMMDD_HHMMSS",
    },
    TemplateTokenHelp {
        group: "Date and time",
        syntax: "{date:%Y-%m-%d}",
        description: "Custom date format; also supported by time and datetime. %Y year, %m month, %d day, %H hour, %M minute, %S second, %% percent",
    },
    TemplateTokenHelp {
        group: "Camera and source",
        syntax: "{camera}",
        description: "Embedded camera model, then source model or display name",
    },
    TemplateTokenHelp {
        group: "Camera and source",
        syntax: "{camera_make}",
        description: "Embedded manufacturer, then source manufacturer",
    },
    TemplateTokenHelp {
        group: "Camera and source",
        syntax: "{camera_model}",
        description: "Embedded camera model, then source model",
    },
    TemplateTokenHelp {
        group: "Camera and source",
        syntax: "{camera_serial}",
        description: "Embedded body serial, then source serial",
    },
    TemplateTokenHelp {
        group: "Camera and source",
        syntax: "{source_name}",
        description: "Source display name",
    },
    TemplateTokenHelp {
        group: "Camera and source",
        syntax: "{lens}",
        description: "Lens model or make",
    },
    TemplateTokenHelp {
        group: "File and numbering",
        syntax: "{original_name}",
        description: "Original filename including extension",
    },
    TemplateTokenHelp {
        group: "File and numbering",
        syntax: "{original_stem}",
        description: "Original filename without extension",
    },
    TemplateTokenHelp {
        group: "File and numbering",
        syntax: "{extension}",
        description: "Original extension without dot",
    },
    TemplateTokenHelp {
        group: "File and numbering",
        syntax: "{media_type}",
        description: "photo or video",
    },
    TemplateTokenHelp {
        group: "File and numbering",
        syntax: "{file_size}",
        description: "File size in bytes",
    },
    TemplateTokenHelp {
        group: "File and numbering",
        syntax: "{sequence}",
        description: "One-based import sequence",
    },
    TemplateTokenHelp {
        group: "File and numbering",
        syntax: "{sequence:04}",
        description: "Zero-pad sequence to 4 digits; widths 1–12",
    },
    TemplateTokenHelp {
        group: "File and numbering",
        syntax: "{session}",
        description: "Shooting-session number, padded to 2 digits",
    },
    TemplateTokenHelp {
        group: "File and numbering",
        syntax: "{session:03}",
        description: "Zero-pad session to 3 digits; widths 1–12",
    },
    TemplateTokenHelp {
        group: "Media metadata",
        syntax: "{width}",
        description: "Width in pixels",
    },
    TemplateTokenHelp {
        group: "Media metadata",
        syntax: "{height}",
        description: "Height in pixels",
    },
    TemplateTokenHelp {
        group: "Media metadata",
        syntax: "{dimensions}",
        description: "Width x height, for example 6000x4000",
    },
    TemplateTokenHelp {
        group: "Media metadata",
        syntax: "{orientation}",
        description: "normal, rotate90, rotate180, rotate270, mirror_horizontal or mirror_vertical",
    },
    TemplateTokenHelp {
        group: "Media metadata",
        syntax: "{duration_millis}",
        description: "Video duration in milliseconds",
    },
    TemplateTokenHelp {
        group: "Media metadata",
        syntax: "{duration_seconds}",
        description: "Video duration in seconds with 3 decimal places",
    },
    TemplateTokenHelp {
        group: "Media metadata",
        syntax: "{gps_latitude}",
        description: "Latitude in decimal degrees with 7 decimal places",
    },
    TemplateTokenHelp {
        group: "Media metadata",
        syntax: "{gps_longitude}",
        description: "Longitude in decimal degrees with 7 decimal places",
    },
    TemplateTokenHelp {
        group: "Media metadata",
        syntax: "{capture_time}",
        description: "Recorded capture timestamp, before clock correction",
    },
    TemplateTokenHelp {
        group: "Media metadata",
        syntax: "{filesystem_time}",
        description: "Recorded filesystem timestamp",
    },
    TemplateTokenHelp {
        group: "Media metadata",
        syntax: "{timestamp_source}",
        description: "exif_original, quicktime, camera or filesystem",
    },
];

impl Template {
    pub fn parse(source: &str) -> Result<Self, TemplateError> {
        let mut parts = Vec::new();
        let mut rest = source;
        while let Some(open) = rest.find('{') {
            if rest[..open].contains('}') {
                return Err(TemplateError::InvalidFormat("stray closing brace".into()));
            }
            if open > 0 {
                parts.push(Part::Literal(rest[..open].to_owned()));
            }
            rest = &rest[open + 1..];
            let close = rest.find('}').ok_or(TemplateError::UnclosedToken)?;
            let token = &rest[..close];
            if token.is_empty() || token.contains('{') {
                return Err(TemplateError::InvalidFormat(
                    "tokens must use {name} or {name:format}".into(),
                ));
            }
            parts.push(Part::Token(parse_token(token)?));
            rest = &rest[close + 1..];
        }
        if rest.contains('}') {
            return Err(TemplateError::InvalidFormat("stray closing brace".into()));
        }
        if !rest.is_empty() {
            parts.push(Part::Literal(rest.to_owned()));
        }
        Ok(Self { parts })
    }

    pub fn render(&self, context: &TemplateContext<'_>) -> String {
        let mut output = String::new();
        for part in &self.parts {
            match part {
                Part::Literal(value) => output.push_str(value),
                Part::Token(token) => {
                    output.push_str(&sanitize_value(&token_value(token, context)))
                }
            }
        }
        output
    }

    pub fn render_filename(&self, context: &TemplateContext<'_>) -> Result<String, TemplateError> {
        let name = self.render(context);
        validate_component(&name)?;
        Ok(name)
    }

    pub fn render_relative_path(
        &self,
        context: &TemplateContext<'_>,
    ) -> Result<String, TemplateError> {
        let path = self.render(context);
        if path.is_empty() {
            return Ok(path);
        }
        let candidate = Path::new(&path);
        if candidate.is_absolute()
            || path.contains("//")
            || path.split('/').any(|part| part == "." || part == "..")
        {
            return Err(TemplateError::InvalidPath(path));
        }
        for component in candidate.components() {
            match component {
                Component::Normal(value) => validate_component(&value.to_string_lossy())?,
                _ => return Err(TemplateError::InvalidPath(path)),
            }
        }
        Ok(path)
    }
}

fn parse_token(name: &str) -> Result<Token, TemplateError> {
    let token = match name {
        "year" => Token::Year,
        "month" => Token::Month,
        "day" => Token::Day,
        "date" => Token::Date,
        "time" => Token::Time,
        "datetime" => Token::Datetime,
        "camera" => Token::Camera,
        "camera_make" => Token::CameraMake,
        "camera_model" => Token::CameraModel,
        "camera_serial" => Token::CameraSerial,
        "original_name" => Token::OriginalName,
        "original_stem" => Token::OriginalStem,
        "extension" => Token::Extension,
        "media_type" => Token::MediaType,
        "session" => Token::Session(2),
        "hour" => Token::Hour,
        "minute" => Token::Minute,
        "second" => Token::Second,
        "file_size" => Token::FileSize,
        "source_name" => Token::SourceName,
        "width" | "height" | "dimensions" | "orientation" | "duration_millis"
        | "duration_seconds" | "lens" | "gps_latitude" | "gps_longitude" | "capture_time"
        | "filesystem_time" | "timestamp_source" => Token::Metadata(name.into()),
        "sequence" => Token::Sequence(0),
        _ if name.starts_with("sequence:") || name.starts_with("session:") => {
            let (base, format) = name.split_once(':').expect("formatted token");
            if format.is_empty() || !format.bytes().all(|b| b.is_ascii_digit()) {
                return Err(TemplateError::InvalidFormat(format!(
                    "{name}: padding width must be 1–12 digits, for example sequence:04"
                )));
            }
            let width = format
                .parse::<usize>()
                .map_err(|_| TemplateError::InvalidFormat(name.into()))?;
            if !(1..=12).contains(&width) {
                return Err(TemplateError::InvalidFormat(format!(
                    "{name}: padding width must be between 1 and 12"
                )));
            }
            if base == "sequence" {
                Token::Sequence(width)
            } else {
                Token::Session(width)
            }
        }
        _ if name.starts_with("date:")
            || name.starts_with("time:")
            || name.starts_with("datetime:") =>
        {
            let (_, format) = name.split_once(':').expect("formatted token");
            if format.is_empty()
                || StrftimeItems::new(format).any(|item| matches!(item, Item::Error))
                || chrono::DateTime::from_timestamp(0, 0)
                    .expect("epoch")
                    .format(format)
                    .write_to(&mut String::new())
                    .is_err()
            {
                return Err(TemplateError::InvalidFormat(format!(
                    "{name}: use a valid date/time format such as %Y-%m-%d"
                )));
            }
            Token::FormattedTime(format.into())
        }
        _ => return Err(TemplateError::UnknownToken(name.into())),
    };
    Ok(token)
}

fn token_value(token: &Token, context: &TemplateContext<'_>) -> String {
    let time = &context.timestamp;
    match token {
        Token::Year => format!("{:04}", time.year()),
        Token::Month => format!("{:02}", time.month()),
        Token::Day => format!("{:02}", time.day()),
        Token::Date => format!("{:04}{:02}{:02}", time.year(), time.month(), time.day()),
        Token::Time => format!("{:02}{:02}{:02}", time.hour(), time.minute(), time.second()),
        Token::Datetime => format!(
            "{:04}{:02}{:02}_{:02}{:02}{:02}",
            time.year(),
            time.month(),
            time.day(),
            time.hour(),
            time.minute(),
            time.second()
        ),
        Token::Camera => context.camera.into(),
        Token::CameraMake => context.camera_make.into(),
        Token::CameraModel => context.camera_model.into(),
        Token::CameraSerial => context.camera_serial.into(),
        Token::OriginalName => context.original_name.into(),
        Token::OriginalStem => context.original_stem.into(),
        Token::Extension => context.extension.into(),
        Token::MediaType => context.media_type.into(),
        Token::Sequence(width) => format!("{:0width$}", context.sequence, width = *width),
        Token::Session(width) => format!("{:0width$}", context.session, width = *width),
        Token::Hour => format!("{:02}", time.hour()),
        Token::Minute => format!("{:02}", time.minute()),
        Token::Second => format!("{:02}", time.second()),
        Token::FormattedTime(format) => {
            let mut value = String::new();
            // parse() validates formatting directives; avoid Display panics even if that changes.
            if time.format(format).write_to(&mut value).is_err() {
                return "unknown".into();
            }
            value
        }
        Token::Metadata(name) => {
            metadata_value(name, context.metadata).unwrap_or_else(|| "unknown".into())
        }
        Token::FileSize => context.file_size.to_string(),
        Token::SourceName => context.source_name.into(),
    }
}

// Values represent one segment, even if camera metadata contains separators.
fn sanitize_value(value: &str) -> String {
    let mut value: String = value
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    while value.ends_with('.') || value.ends_with(' ') {
        value.pop();
        value.push('_');
    }
    if value == "."
        || value == ".."
        || value.starts_with(".captureport-")
        || reserved_component(&value)
    {
        value.insert(0, '_');
    }
    value
}

fn metadata_value(name: &str, metadata: Option<&MediaMetadata>) -> Option<String> {
    let m = metadata?;
    match name {
        "width" => m.width.map(|v| v.to_string()),
        "height" => m.height.map(|v| v.to_string()),
        "dimensions" => Some(format!("{}x{}", m.width?, m.height?)),
        "orientation" => m.orientation.map(|v| {
            match v {
                Orientation::Normal => "normal",
                Orientation::Rotate90 => "rotate90",
                Orientation::Rotate180 => "rotate180",
                Orientation::Rotate270 => "rotate270",
                Orientation::MirrorHorizontal => "mirror_horizontal",
                Orientation::MirrorVertical => "mirror_vertical",
            }
            .into()
        }),
        "duration_millis" => m.duration_millis.map(|v| v.to_string()),
        "duration_seconds" => m
            .duration_millis
            .map(|v| format!("{}.{:03}", v / 1000, v % 1000)),
        "lens" => m.lens.clone().filter(|v| !v.is_empty()),
        "gps_latitude" => m
            .gps_e7
            .map(|(v, _)| format!("{:.7}", f64::from(v) / 10_000_000.0)),
        "gps_longitude" => m
            .gps_e7
            .map(|(_, v)| format!("{:.7}", f64::from(v) / 10_000_000.0)),
        "capture_time" => m.capture_time.clone().filter(|v| !v.is_empty()),
        "filesystem_time" => m.filesystem_time.clone().filter(|v| !v.is_empty()),
        "timestamp_source" => m.timestamp_source.map(|v| {
            match v {
                TimestampSource::ExifOriginal => "exif_original",
                TimestampSource::QuickTime => "quicktime",
                TimestampSource::Camera => "camera",
                TimestampSource::Filesystem => "filesystem",
            }
            .into()
        }),
        _ => None,
    }
}

fn reserved_component(value: &str) -> bool {
    let base = value
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || base
            .strip_prefix("COM")
            .or_else(|| base.strip_prefix("LPT"))
            .is_some_and(|n| n.len() == 1 && matches!(n.as_bytes()[0], b'1'..=b'9'))
}

fn validate_component(value: &str) -> Result<(), TemplateError> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.ends_with('.')
        || value.ends_with(' ')
        || reserved_component(value)
        || value.starts_with(".captureport-")
        || value.chars().any(|character| {
            matches!(
                character,
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
            ) || character.is_control()
        })
    {
        return Err(TemplateError::InvalidPath(value.into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn context() -> TemplateContext<'static> {
        TemplateContext {
            timestamp: FixedOffset::east_opt(0)
                .unwrap()
                .with_ymd_and_hms(2026, 9, 27, 14, 20, 33)
                .unwrap(),
            camera: "Sony",
            camera_make: "Sony",
            camera_model: "A7C II",
            camera_serial: "123",
            original_name: "DSC0001.ARW",
            original_stem: "DSC0001",
            extension: "ARW",
            media_type: "photo",
            sequence: 1,
            session: 2,
            metadata: None,
            file_size: 1024,
            source_name: "SD Card",
        }
    }

    #[test]
    fn parses_and_renders_deterministically() {
        let template =
            Template::parse("{year}/{date}/{date}_{time}_{sequence:04}.{extension}").unwrap();
        assert_eq!(
            template.render_relative_path(&context()).unwrap(),
            "2026/20260927/20260927_142033_0001.ARW"
        );
        assert_eq!(
            template.render_relative_path(&context()).unwrap(),
            "2026/20260927/20260927_142033_0001.ARW"
        );
    }

    #[test]
    fn metadata_and_custom_formats_are_available_in_both_paths() {
        let metadata = MediaMetadata {
            width: Some(6000),
            height: Some(4000),
            orientation: Some(Orientation::Rotate90),
            lens: Some("FE 24/70mm".into()),
            gps_e7: Some((-337654321, 1511234567)),
            duration_millis: Some(120034),
            timestamp_source: Some(TimestampSource::ExifOriginal),
            ..MediaMetadata::default()
        };
        let mut ctx = context();
        ctx.metadata = Some(&metadata);
        let template = Template::parse("{date:%Y-%m-%d}/{lens}/{dimensions}_{orientation}_{duration_seconds}_{gps_latitude}_{gps_longitude}_{timestamp_source}_{file_size}_{session:03}.{extension}").unwrap();
        assert_eq!(
            template.render_relative_path(&ctx).unwrap(),
            "2026-09-27/FE 24_70mm/6000x4000_rotate90_120.034_-33.7654321_151.1234567_exif_original_1024_002.ARW"
        );
        assert_eq!(
            Template::parse("{time:%H-%M-%S}_{hour}{minute}{second}.{extension}")
                .unwrap()
                .render_filename(&ctx)
                .unwrap(),
            "14-20-33_142033.ARW"
        );
    }

    #[test]
    fn missing_metadata_keeps_folder_segments_visible() {
        let template = Template::parse("{lens}/{width}/{original_name}").unwrap();
        assert_eq!(
            template.render_relative_path(&context()).unwrap(),
            "unknown/unknown/DSC0001.ARW"
        );
    }

    #[test]
    fn unsafe_token_values_cannot_create_extra_folders_or_traverse() {
        let mut ctx = context();
        ctx.camera_model = "../A\\B:Model\n";
        let template = Template::parse("{camera_model}/{original_name}").unwrap();
        assert_eq!(
            template.render_relative_path(&ctx).unwrap(),
            ".._A_B_Model_/DSC0001.ARW"
        );
        ctx.camera_model = "..";
        assert_eq!(
            template.render_relative_path(&ctx).unwrap(),
            "._/DSC0001.ARW"
        );
        assert!(
            Template::parse("a/./b")
                .unwrap()
                .render_relative_path(&ctx)
                .is_err()
        );
        assert!(
            Template::parse("a/../b")
                .unwrap()
                .render_relative_path(&ctx)
                .is_err()
        );
    }

    #[test]
    fn reference_examples_parse_and_bad_formats_report_errors() {
        for help in TEMPLATE_TOKENS {
            Template::parse(help.syntax).unwrap();
        }
        for source in [
            "{}",
            "{{year}",
            "{date:%Q}",
            "{datetime:%#z}",
            "{date:}",
            "{sequence:+4}",
            "{session:0}",
            "{session:13}",
        ] {
            assert!(
                matches!(
                    Template::parse(source),
                    Err(TemplateError::InvalidFormat(_))
                ),
                "{source}"
            );
        }
    }

    #[test]
    fn rejects_bad_tokens_and_unsafe_names() {
        assert!(matches!(
            Template::parse("{unknown}"),
            Err(TemplateError::UnknownToken(_))
        ));
        assert!(matches!(
            Template::parse("{sequence:99}"),
            Err(TemplateError::InvalidFormat(_))
        ));
        assert!(matches!(
            Template::parse("{date"),
            Err(TemplateError::UnclosedToken)
        ));
        assert!(
            Template::parse("../{original_name}")
                .unwrap()
                .render_relative_path(&context())
                .is_err()
        );
        for value in [
            "a//b",
            "a/./b",
            "CON",
            "aux.jpg",
            "bad:name",
            "bad?name",
            "trailing ",
        ] {
            assert!(
                Template::parse(value)
                    .unwrap()
                    .render_relative_path(&context())
                    .is_err(),
                "{value}"
            );
        }
        assert!(
            Template::parse("a/{original_name}")
                .unwrap()
                .render_filename(&context())
                .is_err()
        );
    }
}
