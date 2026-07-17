use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::cli::DoctorRequest;
use crate::locator::{Tool, locate};

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub schema_version: u8,
    pub ready: bool,
    pub pinplay_version: &'static str,
    pub platform: PlatformReport,
    pub capabilities: CapabilityReport,
    pub dependencies: Vec<DependencyReport>,
    pub hints: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PlatformReport {
    pub os: &'static str,
    pub arch: &'static str,
    pub window_positioning: &'static str,
}

#[derive(Debug, Serialize)]
pub struct CapabilityReport {
    pub local_playback: bool,
    pub web_playback: bool,
    pub youtube_runtime: bool,
}

#[derive(Debug, Serialize)]
pub struct DependencyReport {
    pub name: &'static str,
    pub required: bool,
    pub found: bool,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
}

impl DoctorReport {
    #[must_use]
    pub fn inspect(request: &DoctorRequest) -> Self {
        let mpv = dependency(Tool::Mpv, request.mpv.as_deref(), true);
        let yt_dlp = dependency(Tool::YtDlp, request.yt_dlp.as_deref(), true);
        let deno = dependency(Tool::Deno, request.deno.as_deref(), false);
        let local_playback = mpv.found;
        let web_playback = mpv.found && yt_dlp.found;
        let youtube_runtime = web_playback && deno.found;
        let ready = local_playback && web_playback;

        let mut hints = Vec::new();
        if !mpv.found || !yt_dlp.found {
            hints.push(install_hint());
        }
        if !deno.found {
            hints.push(
                "Install deno for the most complete current YouTube support in yt-dlp.".to_owned(),
            );
        }

        Self {
            schema_version: 1,
            ready,
            pinplay_version: env!("CARGO_PKG_VERSION"),
            platform: PlatformReport {
                os: std::env::consts::OS,
                arch: std::env::consts::ARCH,
                window_positioning: positioning_support(),
            },
            capabilities: CapabilityReport {
                local_playback,
                web_playback,
                youtube_runtime,
            },
            dependencies: vec![mpv, yt_dlp, deno],
            hints,
        }
    }

    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        if self.ready { 0 } else { 3 }
    }

    #[must_use]
    pub fn human_readable(&self) -> String {
        let mut lines = vec![format!("pinplay {}", self.pinplay_version)];
        lines.extend(self.dependencies.iter().map(|dependency| {
            let symbol = if dependency.found { "✓" } else { "✗" };
            let path = dependency
                .path
                .as_deref()
                .map_or_else(|| "not found".to_owned(), |path| path.display().to_string());
            let version = dependency
                .version
                .as_deref()
                .map_or_else(String::new, |version| format!(" — {version}"));
            format!("{symbol} {:<7} {path}{version}", dependency.name)
        }));
        lines.push(format!(
            "{} Local playback",
            readiness_symbol(self.capabilities.local_playback)
        ));
        lines.push(format!(
            "{} Web playback",
            readiness_symbol(self.capabilities.web_playback)
        ));
        lines.extend(self.hints.iter().map(|hint| format!("Hint: {hint}")));
        lines.join("\n")
    }
}

fn dependency(tool: Tool, explicit: Option<&Path>, required: bool) -> DependencyReport {
    let path = locate(tool, explicit);
    let version = path.as_deref().and_then(read_version);
    DependencyReport {
        name: tool.name(),
        required,
        found: path.is_some(),
        path,
        version,
    }
}

fn read_version(path: &Path) -> Option<String> {
    let output = Command::new(path).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    stdout
        .lines()
        .chain(stderr.lines())
        .find(|line| !line.trim().is_empty())
        .map(sanitize_version)
}

fn sanitize_version(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .take(200)
        .collect()
}

fn install_hint() -> String {
    if cfg!(target_os = "macos") {
        "Install dependencies with `brew install mpv yt-dlp`.".to_owned()
    } else if cfg!(target_os = "windows") {
        "Install mpv and yt-dlp, then make sure both are on PATH.".to_owned()
    } else {
        "Install mpv and yt-dlp with your system package manager.".to_owned()
    }
}

const fn positioning_support() -> &'static str {
    if cfg!(target_os = "linux") {
        "best-effort"
    } else {
        "supported"
    }
}

const fn readiness_symbol(ready: bool) -> &'static str {
    if ready { "✓" } else { "✗" }
}
