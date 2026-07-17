use std::ffi::OsString;
use std::path::PathBuf;
use std::str::FromStr;

use pinplay::model::{PlaybackOptions, PlaybackRequest, Position, Size, Source, StartTime};
use pinplay::player::MpvCommand;

fn args_as_strings(command: &MpvCommand) -> Vec<String> {
    command
        .args()
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn mpv_arguments_encode_the_default_pip_window() {
    let request = PlaybackRequest {
        source: Source::parse(OsString::from("https://example.test/video")).unwrap(),
        options: PlaybackOptions::default(),
    };
    let command = MpvCommand::new(PathBuf::from("/usr/bin/mpv"), request, None);
    let args = args_as_strings(&command);

    assert_eq!(command.program(), PathBuf::from("/usr/bin/mpv"));
    assert!(args.contains(&"--no-config".to_owned()));
    assert!(args.contains(&"--ontop".to_owned()));
    assert!(args.contains(&"--border=no".to_owned()));
    assert!(args.contains(&"--osc=no".to_owned()));
    assert!(args.contains(&"--geometry=320x180-24-24".to_owned()));
    assert!(args.contains(&"--window-dragging=yes".to_owned()));
    assert!(args.contains(&"--auto-window-resize=no".to_owned()));
    assert_eq!(
        &args[args.len() - 2..],
        ["--", "https://example.test/video"]
    );
}

#[test]
fn requested_playback_controls_map_to_mpv_without_shell_text() {
    let options = PlaybackOptions {
        size: Size::new(240, 135).unwrap(),
        position: Position::TopLeft,
        start: Some(StartTime::from_str("1:30").unwrap()),
        mute: true,
        speed: pinplay::model::PlaybackSpeed::new(1.5).unwrap(),
        volume: Some(35),
        loop_video: true,
        border: true,
        controls: true,
        ..PlaybackOptions::default()
    };

    let source = "https://example.test/video;touch /tmp/pinplay-must-not-exist";
    let request = PlaybackRequest {
        source: Source::parse(OsString::from(source)).unwrap(),
        options,
    };
    let command = MpvCommand::new(
        PathBuf::from("mpv"),
        request,
        Some(PathBuf::from("/opt/homebrew/bin/yt-dlp")),
    );
    let args = args_as_strings(&command);

    assert!(args.contains(&"--geometry=240x135+24+24".to_owned()));
    assert!(args.contains(&"--start=1:30".to_owned()));
    assert!(args.contains(&"--mute=yes".to_owned()));
    assert!(args.contains(&"--speed=1.5".to_owned()));
    assert!(args.contains(&"--volume=35".to_owned()));
    assert!(args.contains(&"--loop-file=inf".to_owned()));
    assert!(args.contains(&"--border=yes".to_owned()));
    assert!(args.contains(&"--osc=yes".to_owned()));
    assert!(
        args.contains(
            &"--script-opts-append=ytdl_hook-ytdl_path=/opt/homebrew/bin/yt-dlp".to_owned()
        )
    );
    assert_eq!(&args[args.len() - 2..], ["--", source]);
    assert_eq!(
        command.extra_path_entries(),
        &[PathBuf::from("/opt/homebrew/bin")]
    );
}

#[test]
fn only_the_yt_dlp_directory_is_added_to_the_child_path() {
    let request = PlaybackRequest {
        source: Source::parse(OsString::from("https://example.test/video")).unwrap(),
        options: PlaybackOptions::default(),
    };
    let command = MpvCommand::new(
        PathBuf::from("mpv"),
        request,
        Some(PathBuf::from("/opt/tools/yt/bin/yt-dlp")),
    );

    assert_eq!(
        command.extra_path_entries(),
        &[PathBuf::from("/opt/tools/yt/bin")]
    );
    assert!(
        !args_as_strings(&command)
            .iter()
            .any(|argument| argument.contains("js-runtimes"))
    );
}
