use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use thiserror::Error;

use crate::model::{
    PlaybackOptions, PlaybackRequest, PlaybackSpeed, Position, Size, Source, SourceError, StartTime,
};

#[derive(Debug, Parser)]
#[command(
    name = "pinplay",
    version,
    about = "Play local and web video in a tiny always-on-top window",
    subcommand_precedence_over_arg = true
)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    playback: PlaybackArgs,

    /// Path to the mpv executable
    #[arg(long, value_name = "PATH", global = true)]
    mpv: Option<PathBuf>,

    /// Path to the yt-dlp executable
    #[arg(long = "yt-dlp", value_name = "PATH", global = true)]
    yt_dlp: Option<PathBuf>,
}

impl Cli {
    pub fn into_action(self) -> Result<Action, CliError> {
        match self.command {
            Some(Command::Doctor(arguments)) => {
                if self.playback.source.is_some() {
                    return Err(CliError::ConflictingAction);
                }
                Ok(Action::Doctor(DoctorRequest {
                    json: arguments.json,
                    mpv: self.mpv,
                    yt_dlp: self.yt_dlp,
                }))
            }
            None => {
                let source = self.playback.source.ok_or(CliError::MissingSource)?;
                let source = Source::parse(source)?;
                Ok(Action::Play(PlaybackRequest {
                    source,
                    options: PlaybackOptions {
                        size: self.playback.size,
                        position: self.playback.position,
                        margin: self.playback.margin,
                        start: self.playback.start,
                        mute: self.playback.mute,
                        speed: self.playback.speed,
                        volume: self.playback.volume,
                        loop_video: self.playback.loop_video,
                        border: self.playback.border,
                        controls: self.playback.controls,
                        detach: self.playback.detach,
                        dry_run: self.playback.dry_run,
                    },
                }))
            }
        }
    }

    #[must_use]
    pub fn runtime_paths(&self) -> RuntimePaths {
        RuntimePaths {
            mpv: self.mpv.clone(),
            yt_dlp: self.yt_dlp.clone(),
        }
    }
}

#[derive(Debug, Args)]
struct PlaybackArgs {
    /// Video URL or local media file
    #[arg(value_name = "SOURCE")]
    source: Option<OsString>,

    /// Start position: seconds, MM:SS, HH:MM:SS, PP%, or #CHAPTER
    #[arg(short, long, value_name = "TIME", allow_hyphen_values = true)]
    start: Option<StartTime>,

    /// Start muted
    #[arg(short, long)]
    mute: bool,

    /// Playback speed from 0.01 to 100
    #[arg(short = 'r', long, default_value = "1", value_name = "RATE")]
    speed: PlaybackSpeed,

    /// Initial volume from 0 to 100
    #[arg(long, value_name = "0-100", value_parser = parse_volume)]
    volume: Option<u8>,

    /// nano, tiny, small, medium, large, or WIDTHxHEIGHT
    #[arg(long, default_value = "small", value_name = "SIZE")]
    size: Size,

    /// Screen corner for the initial window
    #[arg(
        long,
        value_enum,
        default_value = "bottom-right",
        value_name = "CORNER"
    )]
    position: Position,

    /// Distance from the selected screen edges
    #[arg(long, default_value = "24", value_name = "PIXELS", value_parser = parse_margin)]
    margin: u16,

    /// Loop the video forever
    #[arg(long = "loop")]
    loop_video: bool,

    /// Show the native window border
    #[arg(long)]
    border: bool,

    /// Show mpv's on-screen controls
    #[arg(long)]
    controls: bool,

    /// Return to the terminal immediately after launching
    #[arg(long)]
    detach: bool,

    /// Print the mpv invocation without launching it
    #[arg(long)]
    dry_run: bool,
}

fn parse_volume(value: &str) -> Result<u8, String> {
    let volume = value
        .parse::<u8>()
        .map_err(|_| "volume must be an integer from 0 to 100".to_owned())?;
    if volume <= 100 {
        Ok(volume)
    } else {
        Err("volume must be an integer from 0 to 100".to_owned())
    }
}

fn parse_margin(value: &str) -> Result<u16, String> {
    let margin = value
        .parse::<u16>()
        .map_err(|_| "margin must be an integer from 0 to 4096".to_owned())?;
    if margin <= 4096 {
        Ok(margin)
    } else {
        Err("margin must be an integer from 0 to 4096".to_owned())
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Check local playback and web-video dependencies without using the network
    Doctor(DoctorArgs),
}

#[derive(Debug, Args)]
struct DoctorArgs {
    /// Print a stable machine-readable report
    #[arg(long)]
    json: bool,
}

#[derive(Clone, Debug)]
pub struct RuntimePaths {
    pub mpv: Option<PathBuf>,
    pub yt_dlp: Option<PathBuf>,
}

#[derive(Debug)]
pub enum Action {
    Play(PlaybackRequest),
    Doctor(DoctorRequest),
}

#[derive(Debug)]
pub struct DoctorRequest {
    pub json: bool,
    pub mpv: Option<PathBuf>,
    pub yt_dlp: Option<PathBuf>,
}

#[derive(Debug, Error)]
pub enum CliError {
    #[error("a SOURCE is required; run `pinplay --help` for usage")]
    MissingSource,
    #[error("choose either playback or the doctor command")]
    ConflictingAction,
    #[error(transparent)]
    InvalidSource(#[from] SourceError),
}
