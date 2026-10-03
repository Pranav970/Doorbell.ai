#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

impl From<RepoError> for gateway_common::AppError {
    fn from(e: RepoError) -> Self {
        match e {
            RepoError::NotFound => gateway_common::AppError::NotFound,
            RepoError::Conflict(msg) => gateway_common::AppError::Conflict(msg),
            RepoError::Database(err) => gateway_common::AppError::Internal(err.to_string()),
        }
    }
}

/// Maps a Postgres unique-violation into `RepoError::Conflict(message)`,
/// passing through any other error unchanged.
pub(crate) fn map_unique_violation(e: sqlx::Error, message: &str) -> RepoError {
    if let sqlx::Error::Database(ref db_err) = e {
        if db_err.kind() == sqlx::error::ErrorKind::UniqueViolation {
            return RepoError::Conflict(message.to_string());
        }
    }
    RepoError::Database(e)
}
