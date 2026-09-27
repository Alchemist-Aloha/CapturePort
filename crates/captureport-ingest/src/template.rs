use chrono::{DateTime, Datelike, FixedOffset, Timelike};
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
    Session,
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
}

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
                Part::Token(token) => output.push_str(&token_value(token, context)),
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
        if candidate.is_absolute() {
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
        "session" => Token::Session,
        "sequence" => Token::Sequence(0),
        _ if name.starts_with("sequence:") => {
            let width = name["sequence:".len()..]
                .parse::<usize>()
                .map_err(|_| TemplateError::InvalidFormat(name.into()))?;
            if !(1..=12).contains(&width) {
                return Err(TemplateError::InvalidFormat(name.into()));
            }
            Token::Sequence(width)
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
        Token::Session => format!("{:02}", context.session),
    }
}

fn validate_component(value: &str) -> Result<(), TemplateError> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.ends_with('.')
        || value.starts_with(".captureport-")
        || value
            .chars()
            .any(|character| character == '/' || character == '\\' || character.is_control())
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
        assert!(
            Template::parse("a/{original_name}")
                .unwrap()
                .render_filename(&context())
                .is_err()
        );
    }
}
