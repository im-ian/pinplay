use std::path::PathBuf;

use clap::Parser;
use pinplay::cli::{Action, Cli};
use pinplay::model::{Position, Size};
use tempfile::NamedTempFile;

#[test]
fn playback_options_can_follow_the_source() {
    let cli = Cli::try_parse_from([
        "pinplay",
        "https://example.test/video",
        "--start",
        "1:30",
        "--mute",
        "--speed",
        "1.5",
        "--size",
        "nano",
        "--position",
        "top-left",
        "--volume",
        "42",
        "--loop",
        "--controls",
    ])
    .unwrap();

    let Action::Play(request) = cli.into_action().unwrap() else {
        panic!("expected play action");
    };
    assert_eq!(request.options.size, Size::new(96, 54).unwrap());
    assert_eq!(request.options.position, Position::TopLeft);
    assert_eq!(request.options.start.unwrap().as_str(), "1:30");
    assert!(request.options.mute);
    assert_eq!(request.options.speed.get(), 1.5);
    assert_eq!(request.options.volume, Some(42));
    assert!(request.options.loop_video);
    assert!(request.options.controls);
}

#[test]
fn default_playback_is_small_borderless_and_bottom_right() {
    let cli = Cli::try_parse_from(["pinplay", "https://example.test/video"]).unwrap();
    let Action::Play(request) = cli.into_action().unwrap() else {
        panic!("expected play action");
    };

    assert_eq!(request.options.size, Size::new(320, 180).unwrap());
    assert_eq!(request.options.position, Position::BottomRight);
    assert_eq!(request.options.margin, 24);
    assert_eq!(request.options.speed.get(), 1.0);
    assert!(!request.options.border);
    assert!(!request.options.controls);
    assert!(!request.options.detach);
}

#[test]
fn doctor_is_an_offline_subcommand() {
    let cli = Cli::try_parse_from([
        "pinplay",
        "--mpv",
        "/tmp/mpv",
        "--yt-dlp",
        "/tmp/yt-dlp",
        "doctor",
        "--json",
    ])
    .unwrap();

    let Action::Doctor(request) = cli.into_action().unwrap() else {
        panic!("expected doctor action");
    };
    assert!(request.json);
    assert_eq!(request.mpv, Some(PathBuf::from("/tmp/mpv")));
    assert_eq!(request.yt_dlp, Some(PathBuf::from("/tmp/yt-dlp")));
}

#[test]
fn deno_is_not_a_pinplay_option() {
    assert!(Cli::try_parse_from([
        "pinplay",
        "--mpv",
        "/tools/mpv",
        "--yt-dlp",
        "/tools/yt-dlp",
        "--deno",
        "/tools/deno",
        "https://example.test/video",
    ])
    .is_err());
}

#[test]
fn a_file_named_doctor_can_be_played_with_an_explicit_path() {
    let file = NamedTempFile::new().unwrap();
    let cli = Cli::try_parse_from([
        "pinplay".into(),
        file.path().as_os_str().to_owned(),
        "--mute".into(),
    ])
    .unwrap();

    assert!(matches!(cli.into_action().unwrap(), Action::Play(_)));
}

#[test]
fn missing_source_is_rejected() {
    let cli = Cli::try_parse_from(["pinplay"]).unwrap();
    assert!(cli.into_action().is_err());
}

#[test]
fn invalid_numeric_options_are_rejected_by_clap() {
    for args in [
        vec!["pinplay", "https://example.test", "--speed", "0"],
        vec!["pinplay", "https://example.test", "--volume", "101"],
        vec!["pinplay", "https://example.test", "--margin", "5000"],
        vec!["pinplay", "https://example.test", "--size", "0x0"],
    ] {
        assert!(Cli::try_parse_from(args).is_err());
    }
}
