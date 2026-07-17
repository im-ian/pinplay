use std::ffi::OsString;
use std::str::FromStr;

use pinplay::model::{PlaybackSpeed, Position, Size, Source, StartTime};
use tempfile::NamedTempFile;

#[test]
fn size_presets_include_an_extremely_small_window() {
    assert_eq!(Size::from_str("nano").unwrap(), Size::new(96, 54).unwrap());
    assert_eq!(Size::from_str("tiny").unwrap(), Size::new(160, 90).unwrap());
    assert_eq!(Size::from_str("small").unwrap(), Size::new(320, 180).unwrap());
    assert_eq!(Size::from_str("medium").unwrap(), Size::new(480, 270).unwrap());
}

#[test]
fn custom_size_is_parsed_case_insensitively() {
    assert_eq!(Size::from_str("240X135").unwrap(), Size::new(240, 135).unwrap());
}

#[test]
fn invalid_or_unreasonable_sizes_are_rejected() {
    for value in ["", "tiny-ish", "0x90", "31x18", "8193x1080", "100x"] {
        assert!(Size::from_str(value).is_err(), "{value} should be invalid");
    }
}

#[test]
fn start_time_accepts_mpv_time_formats() {
    for value in ["90", "1:30", "01:02:03.5", "25%", "#3", "-10"] {
        assert!(StartTime::from_str(value).is_ok(), "{value} should be valid");
    }
}

#[test]
fn start_time_rejects_malformed_values() {
    for value in ["", "later", "1:60", "1:2:60", "101%", "#0", "1:2:3:4"] {
        assert!(StartTime::from_str(value).is_err(), "{value} should be invalid");
    }
}

#[test]
fn playback_speed_enforces_mpv_range() {
    assert_eq!(PlaybackSpeed::new(1.5).unwrap().get(), 1.5);
    assert!(PlaybackSpeed::new(0.009).is_err());
    assert!(PlaybackSpeed::new(100.1).is_err());
    assert!(PlaybackSpeed::new(f64::NAN).is_err());
}

#[test]
fn existing_local_file_is_canonicalized() {
    let file = NamedTempFile::new().unwrap();
    let source = Source::parse(file.path().as_os_str().to_owned()).unwrap();

    assert_eq!(source.as_os_str(), file.path().canonicalize().unwrap());
    assert!(!source.is_remote());
}

#[test]
fn protocol_url_is_preserved_exactly() {
    let raw = "https://example.test/watch?v=a;b=2";
    let source = Source::parse(OsString::from(raw)).unwrap();

    assert_eq!(source.as_os_str(), raw);
    assert!(source.is_remote());
}

#[test]
fn missing_local_path_is_rejected() {
    assert!(Source::parse(OsString::from("definitely-not-a-video.mp4")).is_err());
}

#[test]
fn geometry_matches_each_supported_corner() {
    let size = Size::new(320, 180).unwrap();

    assert_eq!(Position::TopLeft.geometry(size, 24), "320x180+24+24");
    assert_eq!(Position::TopRight.geometry(size, 24), "320x180-24+24");
    assert_eq!(Position::BottomLeft.geometry(size, 24), "320x180+24-24");
    assert_eq!(Position::BottomRight.geometry(size, 24), "320x180-24-24");
}

