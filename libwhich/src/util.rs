use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::error::{WhichError, WhichResult};

/// What `cmd.exe` uses when `PATHEXT` is unset.
const DEFAULT_PATHEXT: &str = ".COM;.EXE;.BAT;.CMD;.VBS;.VBE;.JS;.JSE;.WSF;.WSH;.MSC";

/// `PATH`'s value split into directories; `None` is an unset `PATH`.
pub(crate) fn search_paths_from(value: Option<&OsStr>) -> WhichResult<Vec<PathBuf>> {
    let value = value.ok_or(WhichError::MissingPathVariable)?;
    Ok(std::env::split_paths(value).map(normalise_dir).collect())
}

/// An empty `PATH` entry is the current directory (`which.c` does the same).
pub(crate) fn normalise_dir(dir: impl Into<PathBuf>) -> PathBuf {
    let dir = dir.into();
    if dir.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        dir
    }
}

/// The extensions a bare name is tried with. Empty everywhere but Windows, where it is
/// `PATHEXT` (or `cmd.exe`'s default when that is unset).
pub(crate) fn pathext() -> Vec<String> {
    if cfg!(windows) {
        parse_pathext(std::env::var_os("PATHEXT").as_deref())
    } else {
        Vec::new()
    }
}

/// Split a `PATHEXT` value on `;`. Entries are trimmed, a missing leading dot is added,
/// blanks are dropped, and an unset or blank value is the `cmd.exe` default.
pub(crate) fn parse_pathext(value: Option<&OsStr>) -> Vec<String> {
    let value = value
        .and_then(OsStr::to_str)
        .map(str::trim)
        .unwrap_or_default();
    let value = if value.is_empty() {
        DEFAULT_PATHEXT
    } else {
        value
    };
    value
        .split(';')
        .map(str::trim)
        .filter(|ext| !ext.is_empty() && *ext != ".")
        .map(|ext| {
            if ext.starts_with('.') {
                ext.to_owned()
            } else {
                format!(".{ext}")
            }
        })
        .collect()
}

/// The file names to try for `name`: the name itself, then the name with each extension
/// appended, skipping an extension the name already carries.
pub(crate) fn candidate_names(name: &str, exts: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(exts.len() + 1);
    out.push(name.to_owned());
    for ext in exts {
        if !ends_with_ignore_ascii_case(name, ext) {
            out.push(format!("{name}{ext}"));
        }
    }
    out
}

/// Whether `path`'s extension is one of `exts` (each given with its leading dot),
/// compared without regard to ASCII case.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
pub(crate) fn has_executable_extension(path: &Path, exts: &[String]) -> bool {
    let Some(ext) = path.extension().and_then(OsStr::to_str) else {
        return false;
    };
    exts.iter()
        .any(|e| e.strip_prefix('.').unwrap_or(e).eq_ignore_ascii_case(ext))
}

fn ends_with_ignore_ascii_case(s: &str, suffix: &str) -> bool {
    s.len() >= suffix.len()
        && s.is_char_boundary(s.len() - suffix.len())
        && s[s.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
}

/// Whether `name` points somewhere itself rather than naming a program to look up.
pub(crate) fn has_separator(name: &str) -> bool {
    name.chars().any(std::path::is_separator)
}

/// Strip the `\\?\` verbatim prefix `canonicalize` produces on Windows, so results look
/// like the paths people type: `\\?\C:\x` → `C:\x`, `\\?\UNC\host\share` → `\\host\share`.
/// Anything else is returned untouched.
pub(crate) fn simplify_verbatim(path: PathBuf) -> PathBuf {
    let Some(s) = path.to_str() else {
        return path;
    };
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        let mut chars = rest.chars();
        let is_drive = chars.next().is_some_and(|c| c.is_ascii_alphabetic())
            && chars.next() == Some(':')
            && matches!(chars.next(), Some('\\') | None);
        if is_drive {
            return PathBuf::from(rest);
        }
    }
    path
}

/// Canonicalise `candidate` and keep it if it is a valid executable.
#[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", fields(path = %candidate.display())))]
pub(crate) fn resolve(candidate: PathBuf) -> Option<PathBuf> {
    let found = is_valid_executable(candidate.canonicalize().ok()?)?;
    Some(if cfg!(windows) {
        simplify_verbatim(found)
    } else {
        found
    })
}

/// `path/name`, canonicalised, if it is a valid executable.
pub fn is_valid_executable_split<P1: AsRef<Path>, P2: AsRef<Path>>(
    path: P1,
    name: P2,
) -> Option<PathBuf> {
    resolve(path.as_ref().join(name.as_ref()))
}

/// `path` back if it is a regular file the current user may execute. The path is not
/// canonicalised; a dangling symlink is not a file and so is not valid.
#[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", fields(path = %path.as_ref().display())))]
pub fn is_valid_executable<P: AsRef<Path>>(path: P) -> Option<PathBuf> {
    let path = path.as_ref();
    if is_valid_executable_impl(path) {
        #[cfg(feature = "tracing")]
        tracing::debug!(path = %path.display(), "found");
        Some(path.to_path_buf())
    } else {
        None
    }
}

/// Windows has no execute bit: a program is a regular file with an extension `PATHEXT`
/// lists, which is also what `cmd.exe` and `CreateProcess` go by.
#[cfg(windows)]
fn is_valid_executable_impl(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.is_file()) && has_executable_extension(path, &pathext())
}

/// `which.c`'s `is_there`: a regular file that `access(2)` says is executable. `access`
/// answers yes to root for any file, so root additionally needs an execute bit set.
/// Without the `libc` feature the mode bits alone decide, which ignores ownership and
/// ACLs.
#[cfg(unix)]
fn is_valid_executable_impl(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    let any_exec_bit = metadata.permissions().mode() & 0o111 != 0;

    #[cfg(feature = "libc")]
    {
        use std::os::unix::ffi::OsStrExt;

        let Ok(c_path) = std::ffi::CString::new(path.as_os_str().as_bytes()) else {
            return false;
        };
        // SAFETY: `c_path` is a NUL-terminated string that outlives the call; `access`
        // and `getuid` read nothing else.
        let (accessible, is_root) = unsafe {
            (
                libc::access(c_path.as_ptr(), libc::X_OK) == 0,
                libc::getuid() == 0,
            )
        };
        accessible && (!is_root || any_exec_bit)
    }
    #[cfg(not(feature = "libc"))]
    {
        any_exec_bit
    }
}

#[cfg(not(any(unix, windows)))]
fn is_valid_executable_impl(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.is_file())
}
