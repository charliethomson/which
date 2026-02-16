use thiserror::Error;

#[derive(Debug, Clone, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub enum WhichError {
    #[error("PATH is unset")]
    MissingPathVariable,
}
