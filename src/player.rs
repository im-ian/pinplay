use std::env;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
#[cfg(target_os = "macos")]
use std::time::UNIX_EPOCH;

#[cfg(unix)]
unsafe extern "C" {
    fn getuid() -> u32;
}

use thiserror::Error;

use crate::model::PlaybackRequest;

const CONTROLS_SCRIPT: &[u8] = include_bytes!("pinplay.lua");

#[derive(Clone, Debug)]
pub struct MpvCommand {
    program: PathBuf,
    args: Vec<OsString>,
    extra_path_entries: Vec<PathBuf>,
}

/// Install the window-control script into a user-private directory.
///
/// mpv loads the file at startup. Identical bytes stay in place so a running
/// player keeps the copy it already opened.
pub fn install_controls_script() -> Result<PathBuf, PlayerError> {
    install_controls_script_at(&runtime_dir())
}

pub fn install_controls_script_at(directory: &Path) -> Result<PathBuf, PlayerError> {
    let path = directory.join("pinplay.lua");
    ensure_private_dir(directory)
        .and_then(|()| publish_bytes(&path, CONTROLS_SCRIPT))
        .map_err(|source| PlayerError::ControlsScript {
            path: path.clone(),
            source,
        })?;
    Ok(path)
}

/// macOS shows the bundle name in the Dock and menu bar. `--title` does not.
pub fn named_player_executable(mpv: &Path) -> Result<PathBuf, PlayerError> {
    named_player_executable_at(mpv, &runtime_dir())
}

fn runtime_dir() -> PathBuf {
    if let Some(path) = env::var_os("XDG_RUNTIME_DIR") {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            return path.join("pinplay");
        }
    }
    fallback_runtime_dir()
}

#[cfg(unix)]
fn fallback_runtime_dir() -> PathBuf {
    env::temp_dir().join(format!("pinplay-{}", current_uid()))
}

#[cfg(not(unix))]
fn fallback_runtime_dir() -> PathBuf {
    env::temp_dir().join("pinplay")
}

#[cfg(unix)]
fn current_uid() -> u32 {
    // `getuid` has no preconditions and cannot fail.
    unsafe { getuid() }
}

fn ensure_private_dir(path: &Path) -> io::Result<()> {
    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return Err(io::Error::new(
                ErrorKind::AlreadyExists,
                "pinplay runtime path is not a private directory",
            ));
        }
        #[cfg(unix)]
        if meta.uid() != current_uid() {
            return Err(io::Error::new(
                ErrorKind::PermissionDenied,
                "pinplay runtime directory is owned by another user",
            ));
        }
    }
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    #[cfg(unix)]
    {
        let meta = fs::symlink_metadata(path)?;
        if meta.file_type().is_symlink() || !meta.is_dir() || meta.uid() != current_uid() {
            return Err(io::Error::new(
                ErrorKind::PermissionDenied,
                "pinplay runtime path is not a private directory",
            ));
        }
        let mut permissions = meta.permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

fn publish_bytes(path: &Path, bytes: &[u8]) -> io::Result<()> {
    remove_symlink(path)?;
    if fs::read(path).ok().as_deref() == Some(bytes) {
        return Ok(());
    }
    let tmp = path.with_file_name(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));
    fs::write(&tmp, bytes)?;
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&tmp)?.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(&tmp, permissions)?;
    }
    if let Err(error) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(error);
    }
    Ok(())
}

fn remove_symlink(path: &Path) -> io::Result<()> {
    if fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink()) {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn named_player_executable_at(mpv: &Path, directory: &Path) -> Result<PathBuf, PlayerError> {
    #[cfg(target_os = "macos")]
    {
        install_pinplay_bundle(mpv, directory)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = directory;
        Ok(mpv.to_path_buf())
    }
}

#[cfg(target_os = "macos")]
fn install_pinplay_bundle(mpv: &Path, directory: &Path) -> Result<PathBuf, PlayerError> {
    let contents = directory.join("Pinplay.app").join("Contents");
    let macos = contents.join("MacOS");
    let executable = macos.join("pinplay");
    ensure_private_dir(directory)
        .and_then(|()| create_real_dir(&directory.join("Pinplay.app")))
        .and_then(|()| create_real_dir(&contents))
        .and_then(|()| create_real_dir(&macos))
        .map_err(|source| PlayerError::NamedExecutable {
            path: macos.clone(),
            source,
        })?;

    let plist = contents.join("Info.plist");
    let plist_body = macos_info_plist();
    publish_bytes(&plist, plist_body.as_bytes()).map_err(|source| {
        PlayerError::NamedExecutable {
            path: plist,
            source,
        }
    })?;

    refresh_named_executable(mpv, &executable).map_err(|source| PlayerError::NamedExecutable {
        path: executable.clone(),
        source,
    })?;
    Ok(executable)
}

#[cfg(target_os = "macos")]
fn refresh_named_executable(source: &Path, executable: &Path) -> io::Result<()> {
    let stamp = executable.with_extension("source");
    remove_symlink(executable)?;
    remove_symlink(&stamp)?;
    if same_file(source, executable) {
        return Ok(());
    }

    let identity = source_identity(source)?;
    if executable.is_file() && fs::read_to_string(&stamp).ok().as_deref() == Some(identity.as_str())
    {
        return Ok(());
    }
    if executable.exists() {
        fs::remove_file(executable)?;
    }
    if fs::hard_link(source, executable).is_err() {
        fs::copy(source, executable)?;
    }
    fs::write(stamp, identity)
}

#[cfg(target_os = "macos")]
fn create_real_dir(path: &Path) -> io::Result<()> {
    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return Err(io::Error::new(
                ErrorKind::AlreadyExists,
                "pinplay runtime path is not a private directory",
            ));
        }
        return Ok(());
    }
    fs::create_dir(path)
}

#[cfg(target_os = "macos")]
fn same_file(left: &Path, right: &Path) -> bool {
    let Ok(left) = fs::metadata(left) else {
        return false;
    };
    let Ok(right) = fs::metadata(right) else {
        return false;
    };
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(target_os = "macos")]
fn source_identity(path: &Path) -> io::Result<String> {
    let meta = fs::metadata(path)?;
    let modified = meta
        .modified()?
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    Ok(format!(
        "{} {} {} {}",
        meta.dev(),
        meta.ino(),
        meta.len(),
        modified
    ))
}

#[cfg(target_os = "macos")]
fn macos_info_plist() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>pinplay</string>
    <key>CFBundleIdentifier</key>
    <string>com.im-ian.pinplay</string>
    <key>CFBundleName</key>
    <string>pinplay</string>
    <key>CFBundleDisplayName</key>
    <string>pinplay</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>{version}</string>
    <key>CFBundleVersion</key>
    <string>{version}</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
"#,
        version = env!("CARGO_PKG_VERSION")
    )
}

impl MpvCommand {
    #[must_use]
    pub fn new(
        program: PathBuf,
        request: PlaybackRequest,
        yt_dlp: Option<PathBuf>,
        controls_script: &Path,
    ) -> Self {
        let options = &request.options;
        let mut args = vec![
            "--no-config".into(),
            "--force-window=immediate".into(),
            "--ontop".into(),
            format!("--border={}", yes_no(options.border)).into(),
            format!("--osc={}", yes_no(options.controls)).into(),
            format!(
                "--geometry={}",
                options.position.geometry(options.size, options.margin)
            )
            .into(),
            "--window-dragging=yes".into(),
            "--input-dragging-deadzone=100000".into(),
            "--auto-window-resize=no".into(),
            "--keepaspect-window=no".into(),
            "--keep-open=no".into(),
            "--ytdl=yes".into(),
            "--title=pinplay".into(),
            format!("--speed={}", options.speed).into(),
            format!("--mute={}", yes_no(options.mute)).into(),
        ];
        let mut script = OsString::from("--script=");
        script.push(controls_script.as_os_str());
        args.push(script);
        args.push(format!("--script-opts-append=pinplay-margin={}", options.margin).into());
        args.push(
            format!(
                "--script-opts-append=pinplay-position={}",
                options.position.as_str()
            )
            .into(),
        );

        if let Some(volume) = options.volume {
            args.push(format!("--volume={volume}").into());
        }
        if let Some(start) = &options.start {
            args.push(format!("--start={}", start.as_str()).into());
        }
        if options.loop_video {
            args.push("--loop-file=inf".into());
        }
        if let Some(path) = &yt_dlp {
            let mut option = OsString::from("--script-opts-append=ytdl_hook-ytdl_path=");
            option.push(path.as_os_str());
            args.push(option);
        }
        #[cfg(target_os = "macos")]
        args.extend(["--focus-on=never".into(), "--on-all-workspaces".into()]);

        args.push("--".into());
        args.push(request.source.as_os_str().to_owned());

        let extra_path_entries = yt_dlp
            .as_deref()
            .and_then(Path::parent)
            .filter(|path| !path.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .into_iter()
            .collect::<Vec<_>>();

        Self {
            program,
            args,
            extra_path_entries,
        }
    }

    #[must_use]
    pub fn program(&self) -> &Path {
        &self.program
    }

    #[must_use]
    pub fn args(&self) -> &[OsString] {
        &self.args
    }

    #[must_use]
    pub fn extra_path_entries(&self) -> &[PathBuf] {
        &self.extra_path_entries
    }

    pub fn run(&self, detach: bool) -> Result<i32, PlayerError> {
        let mut command = Command::new(&self.program);
        command.args(&self.args);
        self.apply_path(&mut command)?;

        if detach {
            command
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            command.spawn().map_err(|source| PlayerError::Spawn {
                program: self.program.clone(),
                source,
            })?;
            return Ok(0);
        }

        let status = command.status().map_err(|source| PlayerError::Spawn {
            program: self.program.clone(),
            source,
        })?;
        Ok(status.code().unwrap_or(1))
    }

    #[must_use]
    pub fn display(&self) -> String {
        let mut output = format!("{:?}", self.program.as_os_str());
        for argument in &self.args {
            let _ = write!(output, " {:?}", argument);
        }
        output
    }

    fn apply_path(&self, command: &mut Command) -> Result<(), PlayerError> {
        if self.extra_path_entries.is_empty() {
            return Ok(());
        }

        let mut paths = self.extra_path_entries.clone();
        if let Some(existing) = env::var_os("PATH") {
            paths.extend(env::split_paths(&existing));
        }
        let combined = env::join_paths(paths).map_err(PlayerError::InvalidPath)?;
        command.env("PATH", combined);
        Ok(())
    }
}

const fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[derive(Debug, Error)]
pub enum PlayerError {
    #[error("failed to launch pinplay at {program}: {source}", program = .program.display())]
    Spawn {
        program: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to build PATH for yt-dlp: {0}")]
    InvalidPath(#[from] env::JoinPathsError),
    #[error("failed to install window controls at {path}: {source}", path = .path.display())]
    ControlsScript {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to prepare the pinplay app at {path}: {source}", path = .path.display())]
    NamedExecutable {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}
