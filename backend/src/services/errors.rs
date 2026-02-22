use serde::Serialize;
use serde_with::serde_as;

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
    InvalidIdpUserId,
    UserNotFound,
    ProviderAlreadyLinked,
    ProviderNotFound,
    CannotUnlinkLastProvider,
    InvalidTemplate,
}

pub type ServiceResult<T> = Result<T, ServiceError>;

impl From<sqlx::Error> for ServiceError {
    fn from(val: sqlx::Error) -> Self {
        match val {
            sqlx::Error::RowNotFound => Self::NotFound,
            sqlx::Error::Database(ref err) if err.constraint().is_some() => Self::Constraint,
            _ => Self::DatabaseError,
        }
    }
}
