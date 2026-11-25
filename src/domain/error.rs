use thiserror::Error;

/// Every failure the application can report. The presentation layer maps each variant to exactly
/// one HTTP status (see `presentation::http::error`), so a status code always means the same thing.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AppError {
    /// The referenced server or client does not exist (404).
    #[error("{0}")]
    NotFound(String),
    /// The input breaks a domain rule (400).
    #[error("{0}")]
    Validation(String),
    /// The NATS connection for the server isn't usable right now (503).
    #[error("{0}")]
    Unavailable(String),
    /// NATS refused the operation for this user (403).
    #[error("{0}")]
    PermissionDenied(String),
    /// An upstream system (NATS, monitoring endpoint) failed (502).
    #[error("{0}")]
    Upstream(String),
    /// A bug or storage failure (500).
    #[error("{0}")]
    Internal(String),
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn not_found(what: &str, id: i64) -> Self {
        AppError::NotFound(format!("{what} {id} not found"))
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        AppError::Validation(msg.into())
    }
}
