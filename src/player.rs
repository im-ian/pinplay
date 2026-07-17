use std::env;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use thiserror::Error;

use crate::model::PlaybackRequest;

#[derive(Clone, Debug)]
pub struct MpvCommand {
    program: PathBuf,
    args: Vec<OsString>,
    extra_path_entries: Vec<PathBuf>,
}

impl MpvCommand {
    #[must_use]
    pub fn new(program: PathBuf, request: PlaybackRequest, yt_dlp: Option<PathBuf>) -> Self {
        Self::with_helpers(program, request, yt_dlp, None)
    }

    #[must_use]
    pub fn with_helpers(
        program: PathBuf,
        request: PlaybackRequest,
        yt_dlp: Option<PathBuf>,
        deno: Option<PathBuf>,
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
            "--auto-window-resize=no".into(),
            "--keepaspect-window=no".into(),
            "--keep-open=no".into(),
            "--ytdl=yes".into(),
            "--title=pinplay".into(),
            format!("--speed={}", options.speed).into(),
            format!("--mute={}", yes_no(options.mute)).into(),
        ];

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
        if let Some(path) = &deno {
            let mut option = OsString::from("--ytdl-raw-options-append=js-runtimes=deno:");
            option.push(path.as_os_str());
            args.push(option);
        }

        #[cfg(target_os = "macos")]
        args.extend(["--focus-on=never".into(), "--on-all-workspaces".into()]);

        args.push("--".into());
        args.push(request.source.as_os_str().to_owned());

        let mut extra_path_entries = [yt_dlp.as_deref(), deno.as_deref()]
            .into_iter()
            .flatten()
            .filter_map(Path::parent)
            .filter(|path| !path.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .collect::<Vec<_>>();
        extra_path_entries.dedup();

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
    #[error("failed to launch mpv at {program}: {source}", program = .program.display())]
    Spawn {
        program: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to build PATH for yt-dlp: {0}")]
    InvalidPath(#[from] env::JoinPathsError),
}
