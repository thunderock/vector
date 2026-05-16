//! Theme error type — wraps plist + io errors for the `.itermcolors` importer.

#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("plist error: {0}")]
    Plist(#[from] plist::Error),
    #[error("plist value is not a dictionary")]
    NotADict,
    #[error("invalid component for key {0}")]
    Field(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
