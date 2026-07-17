use clap::Parser;
use thiserror::Error;

use crate::cli::{Action, Cli, CliError};
use crate::doctor::DoctorReport;
use crate::locator::{LocateError, Tool, locate, require};
use crate::player::{MpvCommand, PlayerError};

pub fn run_from_env() -> Result<u8, AppError> {
    run(Cli::parse())
}

pub fn run(cli: Cli) -> Result<u8, AppError> {
    let runtime = cli.runtime_paths();
    match cli.into_action()? {
        Action::Play(request) => {
            let mpv = require(Tool::Mpv, runtime.mpv.as_deref())?;
            let yt_dlp = if let Some(explicit) = runtime.yt_dlp.as_deref() {
                Some(require(Tool::YtDlp, Some(explicit))?)
            } else {
                locate(Tool::YtDlp, None)
            };
            let deno = if let Some(explicit) = runtime.deno.as_deref() {
                Some(require(Tool::Deno, Some(explicit))?)
            } else {
                locate(Tool::Deno, None)
            };
            let detach = request.options.detach;
            let dry_run = request.options.dry_run;
            let command = MpvCommand::with_helpers(mpv, request, yt_dlp, deno);
            if dry_run {
                println!("{}", command.display());
                Ok(0)
            } else {
                let code = command.run(detach)?;
                Ok(u8::try_from(code).unwrap_or(1))
            }
        }
        Action::Doctor(request) => {
            let report = DoctorReport::inspect(&request);
            if request.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", report.human_readable());
            }
            Ok(report.exit_code())
        }
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Cli(#[from] CliError),
    #[error(transparent)]
    Locate(#[from] LocateError),
    #[error(transparent)]
    Player(#[from] PlayerError),
    #[error("failed to serialize doctor report: {0}")]
    Json(#[from] serde_json::Error),
}
