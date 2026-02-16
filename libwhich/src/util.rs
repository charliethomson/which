use std::path::Path;
use std::path::PathBuf;

use crate::error::WhichError;

#[cfg_attr(feature = "tracing", tracing::instrument(level = "debug"))]
pub fn extract_search_paths() -> Result<Vec<PathBuf>, WhichError> {
    let path = std::env::var_os("PATH").ok_or(WhichError::MissingPathVariable)?;
    let paths: Vec<PathBuf> = std::env::split_paths(&path).collect();
    #[cfg(feature = "tracing")]
    tracing::debug!(path_count = paths.len(), "resolved search paths");
    Ok(paths)
}

#[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", fields(path = %path.as_ref().display(), name = %name.as_ref().display())))]
pub fn is_valid_executable_split<P1: AsRef<Path>, P2: AsRef<Path>>(
    path: P1,
    name: P2,
) -> Option<PathBuf> {
    is_valid_executable(path.as_ref().join(name.as_ref()).canonicalize().ok()?)
}

#[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", fields(path = %path.as_ref().display())))]
pub fn is_valid_executable<P: AsRef<Path>>(path: P) -> Option<PathBuf> {
    if is_valid_executable_impl(&path.as_ref().to_path_buf()) {
        #[cfg(feature = "tracing")]
        tracing::debug!(path = %path.as_ref().display(), "found");
        Some(path.as_ref().to_path_buf())
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn is_valid_executable_impl(path: &Path) -> bool {
    if !std::env::var(crate::ENV_SUPPRESS_WARNINGS_KEY).is_ok() {
        eprintln!("[!] Windows validation is not yet supported, incorrect results may appear");
    }
    return true;
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn is_valid_executable_impl(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };

    if !metadata.is_file() {
        return false;
    }

    #[cfg(feature = "libc")]
    {
        let is_superuser = unsafe { libc::getuid() == 0 };

        #[cfg(target_os = "linux")]
        let mode: u32 = std::os::unix::fs::MetadataExt::mode(&metadata);

        #[cfg(target_os = "macos")]
        let mode: u32 = std::os::darwin::fs::MetadataExt::st_mode(&metadata);

        let is_executable = (mode & (libc::S_IXUSR | libc::S_IXGRP | libc::S_IXOTH) as u32) != 0;

        if !is_superuser && !is_executable {
            return false;
        }
    }

    true
}
