use std::path::PathBuf;

use crate::util::is_correct;

mod error;
mod util;

#[allow(unused)]
const ENV_SUPPRESS_WARNINGS_KEY: &str = "LIBWHICH_SUPPRESS_WARNINGS";

pub use crate::error::WhichError;
pub fn which<S: AsRef<str>>(names: &[S]) -> Result<impl Iterator<Item = PathBuf>, WhichError> {
    let paths = util::extract_search_paths()?;

    let names = names
        .iter()
        .map(|s| s.as_ref().to_string())
        .chain({
            if cfg!(target_os = "windows") {
                Box::new(names.iter().map(|name| format!("{}.exe", name.as_ref())))
                    as Box<dyn Iterator<Item = String>>
            } else {
                Box::new(std::iter::empty::<String>()) as Box<dyn Iterator<Item = String>>
            }
        })
        .collect::<Vec<_>>();

    let sets = paths.into_iter().flat_map(move |path| {
        names
            .clone()
            .into_iter()
            .map(move |name| (path.clone(), name.clone()))
    });

    Ok(sets
        .into_iter()
        .filter_map(|(path, name)| is_correct(&path, &name)))
}
