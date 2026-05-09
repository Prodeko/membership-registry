use serde::Serialize;
use serde_with::serde_as;

use crate::application::ports::data_export_port::DataExportError;
use crate::application::ports::marketing_list_port::MarketingListError;
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
    /// External system is reachable but configured wrong (e.g. missing
    /// Keycloak client scope). Distinct from IdpError which is transient.
    Misconfigured(String),
    /// The local write succeeded but the matching IdP write failed; the two
    /// systems are now out of sync until reconciled. Carries a description
    /// the admin can act on (which subjects/providers failed).
    PartialSync(String),
    TokenExpired,
    InvalidIdpUserId,
    UserNotFound,
    ProviderAlreadyLinked,
    ProviderNotFound,
    CannotUnlinkLastProvider,
    InvalidTemplate,
    ExportFailed,
    MarketingSyncFailed(String),
}

impl From<MarketingListError> for ServiceError {
    fn from(e: MarketingListError) -> Self {
        Self::MarketingSyncFailed(format!("{e:?}"))
    }
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
