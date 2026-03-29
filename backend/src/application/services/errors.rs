use serde::Serialize;
use serde_with::serde_as;

use crate::application::ports::data_export_port::DataExportError;
use crate::application::ports::repository_error::RepositoryError;

#[serde_as]
#[derive(Serialize, Debug)]
pub enum ServiceError {
    Constraint(String),
    AlreadyExists,
    InvalidStatus,
    InvalidInput,
    ApplicationAlreadyProcessed,
    NotFound,
    NotActive,
    DatabaseError(String),
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
    ExportFailed,
}

impl From<DataExportError> for ServiceError {
    fn from(_: DataExportError) -> Self {
        Self::ExportFailed
    }
}

pub type ServiceResult<T> = Result<T, ServiceError>;

impl From<RepositoryError> for ServiceError {
    fn from(val: RepositoryError) -> Self {
        match val {
            RepositoryError::NotFound => Self::NotFound,
            RepositoryError::AlreadyExists => Self::AlreadyExists,
            RepositoryError::Constraint(msg) => Self::Constraint(msg),
            RepositoryError::Unexpected(msg) => Self::DatabaseError(msg),
        }
    }
}
