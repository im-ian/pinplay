use std::env;
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Clone, Copy, Debug)]
pub enum Tool {
    Mpv,
    YtDlp,
    Deno,
}

impl Tool {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Mpv => "mpv",
            Self::YtDlp => "yt-dlp",
            Self::Deno => "deno",
        }
    }

    fn common_paths(self) -> Vec<PathBuf> {
        let name = self.name();
        #[cfg(windows)]
        {
            vec![PathBuf::from(format!(
                r"C:\Program Files\{name}\{name}.exe"
            ))]
        }
        #[cfg(not(windows))]
        {
            vec![
                PathBuf::from(format!("/opt/homebrew/bin/{name}")),
                PathBuf::from(format!("/usr/local/bin/{name}")),
                PathBuf::from(format!("/usr/bin/{name}")),
            ]
        }
    }
}

#[must_use]
pub fn locate(tool: Tool, explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return locate_explicit(path);
    }

    find_on_path(tool.name())
        .or_else(|| {
            tool.common_paths()
                .into_iter()
                .find(|path| is_executable(path))
        })
        .map(canonical_or_original)
}

pub fn require(tool: Tool, explicit: Option<&Path>) -> Result<PathBuf, LocateError> {
    locate(tool, explicit).ok_or(LocateError { tool })
}

fn locate_explicit(path: &Path) -> Option<PathBuf> {
    if is_executable(path) {
        return Some(canonical_or_original(path.to_path_buf()));
    }
    if path.components().count() == 1 {
        return find_on_path(path.to_string_lossy().as_ref());
    }
    None
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .flat_map(|directory| executable_candidates(&directory, name))
        .find(|candidate| is_executable(candidate))
        .map(canonical_or_original)
}

#[cfg(windows)]
fn executable_candidates(directory: &Path, name: &str) -> Vec<PathBuf> {
    let base = directory.join(name);
    if base.extension().is_some() {
        vec![base]
    } else {
        vec![
            base.clone(),
            base.with_extension("exe"),
            base.with_extension("cmd"),
        ]
    }
}

#[cfg(not(windows))]
fn executable_candidates(directory: &Path, name: &str) -> Vec<PathBuf> {
    vec![directory.join(name)]
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

fn canonical_or_original(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

#[derive(Debug, Error)]
#[error(
    "{name} was not found; run `pinplay doctor` for details and install {name} first",
    name = .tool.name()
)]
pub struct LocateError {
    tool: Tool,
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::has_safe_windows_executable_extension;

    #[test]
    fn windows_only_executes_native_exe_files() {
        assert!(has_safe_windows_executable_extension(Path::new("mpv.exe")));
        assert!(has_safe_windows_executable_extension(Path::new("MPV.EXE")));
        assert!(!has_safe_windows_executable_extension(Path::new("mpv.cmd")));
        assert!(!has_safe_windows_executable_extension(Path::new("mpv.bat")));
        assert!(!has_safe_windows_executable_extension(Path::new("mpv")));
    }
}
