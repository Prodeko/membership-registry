use serde::Serialize;
use serde_with::serde_as;

use crate::application::ports::repository_error::RepositoryError;

#[serde_as]
#[derive(Serialize, Debug)]
pub enum ServiceError {
    Constraint,
    AlreadyExists,
    InvalidStatus,
    InvalidInput,
    ApplicationAlreadyProcessed,
    NotFound,
    NotActive,
    DatabaseError,
    Unauthorized,
    Forbidden,
    IdpError,
    TokenExpired,
    InvalidIdpUserId,
    UserNotFound,
    ProviderAlreadyLinked,
    ProviderNotFound,
    CannotUnlinkLastProvider,
    InvalidTemplate,
}

pub type ServiceResult<T> = Result<T, ServiceError>;

impl From<RepositoryError> for ServiceError {
    fn from(val: RepositoryError) -> Self {
        match val {
            RepositoryError::NotFound => Self::NotFound,
            RepositoryError::AlreadyExists => Self::AlreadyExists,
            RepositoryError::Constraint(_) => Self::Constraint,
            RepositoryError::Unexpected(_) => Self::DatabaseError,
        }
    }
}