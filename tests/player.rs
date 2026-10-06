use std::ffi::OsString;
use std::path::{Path, PathBuf};
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
    let command = MpvCommand::new(
        PathBuf::from("/usr/bin/mpv"),
        request,
        None,
        Path::new("/tmp/pinplay-controls/pinplay.lua"),
    );
    let args = args_as_strings(&command);

    assert_eq!(command.program(), PathBuf::from("/usr/bin/mpv"));
    assert!(args.contains(&"--no-config".to_owned()));
    assert!(args.contains(&"--ontop".to_owned()));
    assert!(args.contains(&"--border=no".to_owned()));
    assert!(args.contains(&"--osc=no".to_owned()));
    assert!(args.contains(&"--geometry=320x180-24-24".to_owned()));
    assert!(args.contains(&"--script=/tmp/pinplay-controls/pinplay.lua".to_owned()));
    assert!(args.contains(&"--script-opts-append=pinplay-margin=24".to_owned()));
    assert!(args.contains(&"--script-opts-append=pinplay-position=bottom-right".to_owned()));
    assert!(args.contains(&"--window-dragging=yes".to_owned()));
    assert!(args.contains(&"--input-dragging-deadzone=100000".to_owned()));
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
        Path::new("pinplay.lua"),
    );
    let args = args_as_strings(&command);

    assert!(args.contains(&"--geometry=240x135+24+24".to_owned()));
    assert!(args.contains(&"--script-opts-append=pinplay-position=top-left".to_owned()));
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
fn controls_script_is_installed_without_rewriting_identical_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let first = pinplay::player::install_controls_script_at(directory.path()).unwrap();
    let first_modified = std::fs::metadata(&first).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    let second = pinplay::player::install_controls_script_at(directory.path()).unwrap();

    let script = std::fs::read_to_string(&second).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        std::fs::metadata(&second).unwrap().modified().unwrap(),
        first_modified
    );
    assert!(script.contains("MBTN_LEFT"));
    assert!(script.contains("MBTN_RIGHT"));
    assert!(script.contains("begin-vo-dragging"));
    assert!(script.contains("일시정지"));
    assert!(script.contains("닫기"));
    assert!(script.contains("320x180"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(directory.path())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700);
    }
}

#[cfg(unix)]
#[test]
fn controls_script_replaces_a_symlink_without_writing_through_it() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let outside = root.path().join("outside.lua");
    std::fs::write(&outside, b"replaced").unwrap();
    let directory = root.path().join("runtime");
    std::fs::create_dir(&directory).unwrap();
    symlink(&outside, directory.join("pinplay.lua")).unwrap();

    let installed = pinplay::player::install_controls_script_at(&directory).unwrap();

    assert!(
        !std::fs::symlink_metadata(&installed)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(
        std::fs::read_to_string(&installed)
            .unwrap()
            .contains("begin-vo-dragging")
    );
    assert_eq!(std::fs::read(&outside).unwrap(), b"replaced");
}

#[cfg(unix)]
#[test]
fn controls_script_rejects_a_symlinked_directory() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let real = root.path().join("real");
    std::fs::create_dir(&real).unwrap();
    let link = root.path().join("link");
    symlink(&real, &link).unwrap();

    let error = pinplay::player::install_controls_script_at(&link).unwrap_err();

    assert!(error.to_string().contains("not a private directory"));
    assert!(std::fs::read_dir(&real).unwrap().next().is_none());
}

#[test]
fn macos_bundle_name_is_pinplay_and_reused() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("mpv");
    std::fs::write(&source, b"player-v1").unwrap();
    let cache = root.path().join("cache");

    let first = pinplay::player::named_player_executable_at(&source, &cache).unwrap();
    #[cfg(target_os = "macos")]
    {
        assert!(first.ends_with("Pinplay.app/Contents/MacOS/pinplay"));
        let plist = std::fs::read_to_string(cache.join("Pinplay.app/Contents/Info.plist")).unwrap();
        assert!(plist.contains("<string>pinplay</string>"));
        assert!(plist.contains("com.im-ian.pinplay"));
        assert!(plist.contains("<key>NSHighResolutionCapable</key>"));
        let modified = std::fs::metadata(&first).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let second = pinplay::player::named_player_executable_at(&source, &cache).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            std::fs::metadata(&second).unwrap().modified().unwrap(),
            modified
        );

        std::fs::remove_file(&source).unwrap();
        std::fs::write(&source, b"player-v2").unwrap();
        let third = pinplay::player::named_player_executable_at(&source, &cache).unwrap();
        assert_eq!(std::fs::read(third).unwrap(), b"player-v2");
    }
    #[cfg(not(target_os = "macos"))]
    {
        assert_eq!(first, source);
        assert!(!cache.exists());
    }
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
        Path::new("pinplay.lua"),
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
