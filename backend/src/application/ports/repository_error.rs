/// Shared error type for all repository ports.
#[derive(Debug)]
pub enum RepositoryError {
    NotFound,
    AlreadyExists,
    Constraint(String),
    Unexpected(String),
}

impl From<sqlx::Error> for RepositoryError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => Self::NotFound,
            sqlx::Error::Database(ref db_err) if db_err.constraint().is_some() => {
                Self::Constraint(db_err.to_string())
            }
            other => Self::Unexpected(other.to_string()),
        }
    }
}
