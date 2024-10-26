use ory_client::apis::Error as OryError;
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
    DatabaseError,
    Unauthorized,
    Forbidden,
    OryError,
}

pub type ServiceResult<T> = Result<T, ServiceError>;

impl From<sqlx::Error> for ServiceError {
    fn from(val: sqlx::Error) -> Self {
        match val {
            sqlx::Error::RowNotFound => Self::NotFound,
            sqlx::Error::Database(ref err) if err.constraint().is_some() => {
                Self::Constraint
            }
            _ => Self::DatabaseError,
        }
    }
}

impl<T> From<OryError<T>> for ServiceError {
    fn from(val: OryError<T>) -> Self {
        match val {
            OryError::ResponseError(e) => {
                if e.status == 404 {
                    Self::NotFound
                } else if e.status == 401 {
                    Self::Unauthorized
                } else if e.status == 403 {
                    Self::Forbidden
                } else {
                    Self::OryError
                }
            },
            _ => Self::OryError,
        }
    }
}
