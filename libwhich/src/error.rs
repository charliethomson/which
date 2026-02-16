use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum WhichError {
    #[error("PATH is unset")]
    MissingPathVariable,
}
