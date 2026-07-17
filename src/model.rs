use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use clap::ValueEnum;
use thiserror::Error;

const MIN_WIDTH: u32 = 32;
const MIN_HEIGHT: u32 = 18;
const MAX_DIMENSION: u32 = 8192;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub fn new(width: u32, height: u32) -> Result<Self, SizeError> {
        if !(MIN_WIDTH..=MAX_DIMENSION).contains(&width) {
            return Err(SizeError::Width(width));
        }
        if !(MIN_HEIGHT..=MAX_DIMENSION).contains(&height) {
            return Err(SizeError::Height(height));
        }
        Ok(Self { width, height })
    }
}

impl FromStr for Size {
    type Err = SizeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "nano" => Self::new(96, 54),
            "tiny" => Self::new(160, 90),
            "small" => Self::new(320, 180),
            "medium" => Self::new(480, 270),
            "large" => Self::new(640, 360),
            custom => {
                let (width, height) = custom
                    .split_once('x')
                    .ok_or_else(|| SizeError::Format(value.to_owned()))?;
                if height.contains('x') {
                    return Err(SizeError::Format(value.to_owned()));
                }
                let width = width
                    .parse::<u32>()
                    .map_err(|_| SizeError::Format(value.to_owned()))?;
                let height = height
                    .parse::<u32>()
                    .map_err(|_| SizeError::Format(value.to_owned()))?;
                Self::new(width, height)
            }
        }
    }
}

impl fmt::Display for Size {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}x{}", self.width, self.height)
    }
}

#[derive(Debug, Error)]
pub enum SizeError {
    #[error("size must be nano, tiny, small, medium, large, or WIDTHxHEIGHT")]
    Format(String),
    #[error("width {0} is outside the supported range {MIN_WIDTH}..={MAX_DIMENSION}")]
    Width(u32),
    #[error("height {0} is outside the supported range {MIN_HEIGHT}..={MAX_DIMENSION}")]
    Height(u32),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum Position {
    #[value(alias = "tl")]
    TopLeft,
    #[value(alias = "tr")]
    TopRight,
    #[value(alias = "bl")]
    BottomLeft,
    #[default]
    #[value(alias = "br")]
    BottomRight,
}

impl Position {
    #[must_use]
    pub fn geometry(self, size: Size, margin: u16) -> String {
        let offset = match self {
            Self::TopLeft => format!("+{margin}+{margin}"),
            Self::TopRight => format!("-{margin}+{margin}"),
            Self::BottomLeft => format!("+{margin}-{margin}"),
            Self::BottomRight => format!("-{margin}-{margin}"),
        };
        format!("{size}{offset}")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartTime(String);

impl StartTime {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for StartTime {
    type Err = TimeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() || value.trim() != value {
            return Err(TimeError(value.to_owned()));
        }

        if let Some(percent) = value.strip_suffix('%') {
            let percent = finite_number(percent, value)?;
            if (0.0..=100.0).contains(&percent) {
                return Ok(Self(value.to_owned()));
            }
            return Err(TimeError(value.to_owned()));
        }

        if let Some(chapter) = value.strip_prefix('#') {
            if chapter.parse::<u32>().is_ok_and(|number| number > 0) {
                return Ok(Self(value.to_owned()));
            }
            return Err(TimeError(value.to_owned()));
        }

        let unsigned = value.strip_prefix(['+', '-']).unwrap_or(value);
        let parts: Vec<_> = unsigned.split(':').collect();
        let valid = match parts.as_slice() {
            [seconds] => valid_seconds(seconds, false),
            [minutes, seconds] => valid_integer(minutes) && valid_seconds(seconds, true),
            [hours, minutes, seconds] => {
                valid_integer(hours)
                    && valid_bounded_integer(minutes, 59)
                    && valid_seconds(seconds, true)
            }
            _ => false,
        };

        if valid {
            Ok(Self(value.to_owned()))
        } else {
            Err(TimeError(value.to_owned()))
        }
    }
}

fn finite_number(value: &str, original: &str) -> Result<f64, TimeError> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
        .ok_or_else(|| TimeError(original.to_owned()))
}

fn valid_seconds(value: &str, bounded: bool) -> bool {
    finite_number(value, value).is_ok_and(|seconds| seconds >= 0.0 && (!bounded || seconds < 60.0))
}

fn valid_integer(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|character| character.is_ascii_digit())
}

fn valid_bounded_integer(value: &str, maximum: u64) -> bool {
    valid_integer(value) && value.parse::<u64>().is_ok_and(|number| number <= maximum)
}

#[derive(Debug, Error)]
#[error("invalid start time {0:?}; use seconds, MM:SS, HH:MM:SS, a percentage, or #CHAPTER")]
pub struct TimeError(String);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaybackSpeed(f64);

impl PlaybackSpeed {
    pub fn new(value: f64) -> Result<Self, SpeedError> {
        if value.is_finite() && (0.01..=100.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(SpeedError(value))
        }
    }

    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

impl Default for PlaybackSpeed {
    fn default() -> Self {
        Self(1.0)
    }
}

impl FromStr for PlaybackSpeed {
    type Err = SpeedError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parsed = value.parse::<f64>().map_err(|_| SpeedError(f64::NAN))?;
        Self::new(parsed)
    }
}

impl fmt::Display for PlaybackSpeed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[derive(Debug, Error)]
#[error("playback speed {0} is outside mpv's supported range 0.01..=100")]
pub struct SpeedError(f64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Source {
    Local(PathBuf),
    Remote(String),
}

impl Source {
    pub fn parse(raw: OsString) -> Result<Self, SourceError> {
        if raw.is_empty() {
            return Err(SourceError::Empty);
        }

        let path = PathBuf::from(&raw);
        if path.exists() {
            if !path.is_file() {
                return Err(SourceError::NotAFile { path });
            }
            let canonical = path
                .canonicalize()
                .map_err(|source| SourceError::Canonicalize { path, source })?;
            return Ok(Self::Local(canonical));
        }

        if let Some(text) = raw.to_str() {
            if is_protocol_url(text) {
                return Ok(Self::Remote(text.to_owned()));
            }
        }

        Err(SourceError::Missing(format!("{raw:?}")))
    }

    #[must_use]
    pub fn as_os_str(&self) -> &OsStr {
        match self {
            Self::Local(path) => path.as_os_str(),
            Self::Remote(url) => url.as_ref(),
        }
    }

    #[must_use]
    pub const fn is_remote(&self) -> bool {
        matches!(self, Self::Remote(_))
    }
}

fn is_protocol_url(value: &str) -> bool {
    let Some((scheme, remainder)) = value.split_once("://") else {
        return false;
    };
    !remainder.is_empty()
        && scheme.starts_with(|character: char| character.is_ascii_alphabetic())
        && scheme.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
        })
}

#[derive(Debug, Error)]
pub enum SourceError {
    #[error("source cannot be empty")]
    Empty,
    #[error("source path does not exist: {0}")]
    Missing(String),
    #[error("source is not a regular file: {path}", path = .path.display())]
    NotAFile { path: PathBuf },
    #[error("failed to resolve source path {path}: {source}", path = .path.display())]
    Canonicalize {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlaybackOptions {
    pub size: Size,
    pub position: Position,
    pub margin: u16,
    pub start: Option<StartTime>,
    pub mute: bool,
    pub speed: PlaybackSpeed,
    pub volume: Option<u8>,
    pub loop_video: bool,
    pub border: bool,
    pub controls: bool,
    pub detach: bool,
    pub dry_run: bool,
}

impl Default for PlaybackOptions {
    fn default() -> Self {
        Self {
            size: Size {
                width: 320,
                height: 180,
            },
            position: Position::BottomRight,
            margin: 24,
            start: None,
            mute: false,
            speed: PlaybackSpeed::default(),
            volume: None,
            loop_video: false,
            border: false,
            controls: false,
            detach: false,
            dry_run: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlaybackRequest {
    pub source: Source,
    pub options: PlaybackOptions,
}
