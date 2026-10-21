use thiserror::Error;

/// What can go wrong looking something up. Serialisable with the `serde` feature, where
/// each variant carries a stable reverse-DNS key as its `$type`
/// (<https://standards.thmsn.dev/d/error-handling>).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(
        rename_all = "camelCase",
        rename_all_fields = "camelCase",
        tag = "$type",
        content = "context"
    )
)]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub enum WhichError {
    /// `PATH` is not set, so there is nowhere to look.
    #[error("PATH is unset")]
    #[cfg_attr(
        feature = "serde",
        serde(rename = "dev.thmsn.which.search_paths.error.unset")
    )]
    MissingPathVariable,
}

pub type WhichResult<T> = Result<T, WhichError>;
