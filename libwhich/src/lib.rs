//! Locate executables on `PATH`, after FreeBSD `which(1)`.
//!
//! [`which`] reads `PATH` from the environment; [`which_in`] searches directories you hand
//! it and touches no environment at all. Both walk the names in the order given and, for
//! each name, the directories in order, so the first result for a name is what a shell
//! would run. A name containing a path separator is checked where it points instead of
//! being joined onto every directory.
//!
//! A hit is a regular file the current user may execute: on Unix that is `access(2)` with
//! `X_OK` (root additionally needs at least one execute bit, as `which.c` does); on
//! Windows it is a file whose extension is listed in `PATHEXT`, and a bare name is also
//! tried with every `PATHEXT` extension appended. Results are canonical absolute paths.

use std::path::PathBuf;

mod error;
mod util;

#[cfg(test)]
mod tests;

pub use crate::error::{WhichError, WhichResult};
pub use crate::util::{is_valid_executable, is_valid_executable_split};

/// Every match for every name, searching the directories in `PATH`.
///
/// Results come grouped by name, in the order the names were given, and within a name in
/// `PATH` order. The search is lazy: nothing is stat'd until the iterator is pulled, so
/// `.next()` costs one hit.
///
/// # Errors
///
/// [`WhichError::MissingPathVariable`] when `PATH` is not set.
#[cfg_attr(feature = "tracing", tracing::instrument(level = "debug", skip_all, fields(name_count = names.len())))]
pub fn which<S: AsRef<str>>(names: &[S]) -> WhichResult<impl Iterator<Item = PathBuf>> {
    Ok(which_in(names, search_paths()?))
}

/// Every match for every name, searching `dirs` instead of `PATH`.
///
/// Same ordering and laziness as [`which`]. An empty directory entry means the current
/// directory, as it does in `PATH`.
#[cfg_attr(feature = "tracing", tracing::instrument(level = "debug", skip_all, fields(name_count = names.len())))]
pub fn which_in<S, I>(names: &[S], dirs: I) -> impl Iterator<Item = PathBuf>
where
    S: AsRef<str>,
    I: IntoIterator,
    I::Item: Into<PathBuf>,
{
    let dirs: Vec<PathBuf> = dirs.into_iter().map(util::normalise_dir).collect();
    let exts = util::pathext();
    let names: Vec<String> = names.iter().map(|s| s.as_ref().to_owned()).collect();
    #[cfg(feature = "tracing")]
    tracing::debug!(dir_count = dirs.len(), ext_count = exts.len(), "searching");

    names.into_iter().flat_map(move |name| {
        let candidates = util::candidate_names(&name, &exts);
        let paths: Vec<PathBuf> = if util::has_separator(&name) {
            candidates.into_iter().map(PathBuf::from).collect()
        } else {
            dirs.iter()
                .flat_map(|dir| candidates.iter().map(move |c| dir.join(c)))
                .collect()
        };
        paths.into_iter().filter_map(util::resolve)
    })
}

/// The directories `PATH` names, in order. An empty entry is the current directory.
///
/// # Errors
///
/// [`WhichError::MissingPathVariable`] when `PATH` is not set.
#[cfg_attr(feature = "tracing", tracing::instrument(level = "debug"))]
pub fn search_paths() -> WhichResult<Vec<PathBuf>> {
    let paths = util::search_paths_from(std::env::var_os("PATH").as_deref())?;
    #[cfg(feature = "tracing")]
    tracing::debug!(path_count = paths.len(), "resolved search paths");
    Ok(paths)
}
