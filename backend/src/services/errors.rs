use serde::Serialize;
use serde_with::serde_as;

#[serde_as]
#[derive(Serialize, Debug)]
pub enum ServiceError {
    ApplicationAlreadyExists,
    InvalidStatus,
    ApplicationAlreadyProcessed,
    NotFound,
    DatabaseError,
}

pub type ServiceResult<T> = Result<T, ServiceError>;

impl From<sqlx::Error> for ServiceError {
    fn from(val: sqlx::Error) -> Self {
        match val {
            sqlx::Error::RowNotFound => Self::NotFound,
            _ => Self::DatabaseError,
        }
    }
}
