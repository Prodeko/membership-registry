/// Shared error type for all repository ports.
#[derive(Debug)]
pub enum RepositoryError {
    NotFound,
    AlreadyExists,
    Constraint(String),
    Unexpected(String),
}
