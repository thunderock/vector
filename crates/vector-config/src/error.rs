//! Config parse error — line/col addressed (Pitfall 2: never byte offsets).

#[derive(Debug, thiserror::Error)]
#[error("config error at line {line}, column {col}: {message}")]
pub struct ConfigError {
    pub line: usize,
    pub col: usize,
    pub message: String,
}
