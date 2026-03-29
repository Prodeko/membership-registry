use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::application::ports::repository_error::RepositoryError;
use crate::application::services::errors::{ServiceError, ServiceResult};
use crate::application::services::template_admin_service::TemplateAdminError;

pub enum ApiError {
    ServiceError(ServiceError),
    NotFound,
    Unauthorized,
    Forbidden,
    BadRequest,
    InternalServerError,
}

pub type ApiResult<T> = Result<T, ApiError>;

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::ServiceError(err) => err.into_response(),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Not found").into_response(),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized").into_response(),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden").into_response(),
            ApiError::BadRequest => (StatusCode::BAD_REQUEST, "Bad request").into_response(),
            ApiError::InternalServerError => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
        }
    }
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        tracing::error!("ServiceError: {:?}", self);

        match self {
            ServiceError::AlreadyExists => {
                (StatusCode::BAD_REQUEST, "Application already exists").into_response()
            }
            ServiceError::Constraint(_) => {
                (StatusCode::BAD_REQUEST, "Constraint violation").into_response()
            }
            ServiceError::InvalidInput => {
                (StatusCode::BAD_REQUEST, "Invalid input").into_response()
            }
            ServiceError::ApplicationAlreadyProcessed => {
                (StatusCode::BAD_REQUEST, "Application already processed").into_response()
            }
            ServiceError::InvalidStatus => {
                (StatusCode::BAD_REQUEST, "Invalid status").into_response()
            }
            ServiceError::NotFound => (StatusCode::NOT_FOUND, "Not found").into_response(),
            ServiceError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
            }
            ServiceError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden").into_response(),
            ServiceError::DatabaseError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
            }
            ServiceError::NotActive => {
                (StatusCode::BAD_REQUEST, "Role is not active").into_response()
            }
            ServiceError::IdpError => {
                (StatusCode::BAD_GATEWAY, "Identity provider error").into_response()
            }
            ServiceError::TokenExpired => {
                (StatusCode::UNAUTHORIZED, "Token expired").into_response()
            }
            ServiceError::InvalidIdpUserId => {
                (StatusCode::BAD_REQUEST, "Invalid identity provider user ID").into_response()
            }
            ServiceError::UserNotFound => (StatusCode::NOT_FOUND, "User not found").into_response(),
            ServiceError::ProviderAlreadyLinked => {
                (StatusCode::BAD_REQUEST, "Provider already linked").into_response()
            }
            ServiceError::ProviderNotFound => {
                (StatusCode::NOT_FOUND, "Provider not found").into_response()
            }
            ServiceError::CannotUnlinkLastProvider => {
                (StatusCode::BAD_REQUEST, "Cannot unlink last provider").into_response()
            }
            ServiceError::InvalidTemplate => (
                StatusCode::BAD_REQUEST,
                "Invalid template: unknown placeholder",
            )
                .into_response(),
            ServiceError::ExportFailed => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Export failed").into_response()
            }
        }
    }
}

impl From<ServiceError> for ApiError {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::AlreadyExists => ApiError::ServiceError(ServiceError::AlreadyExists),
            ServiceError::Constraint(msg) => ApiError::ServiceError(ServiceError::Constraint(msg)),
            ServiceError::InvalidInput => ApiError::ServiceError(ServiceError::InvalidInput),
            ServiceError::ApplicationAlreadyProcessed => {
                ApiError::ServiceError(ServiceError::ApplicationAlreadyProcessed)
            }
            ServiceError::InvalidStatus => ApiError::ServiceError(ServiceError::InvalidStatus),
            ServiceError::NotFound => ApiError::NotFound,
            ServiceError::Unauthorized => ApiError::Unauthorized,
            ServiceError::Forbidden => ApiError::Forbidden,
            ServiceError::DatabaseError(_) => ApiError::InternalServerError,
            ServiceError::NotActive => ApiError::BadRequest,
            ServiceError::IdpError => ApiError::ServiceError(ServiceError::IdpError),
            ServiceError::TokenExpired => ApiError::Unauthorized,
            ServiceError::InvalidIdpUserId => ApiError::BadRequest,
            ServiceError::UserNotFound => ApiError::NotFound,
            ServiceError::ProviderAlreadyLinked => ApiError::BadRequest,
            ServiceError::ProviderNotFound => ApiError::NotFound,
            ServiceError::CannotUnlinkLastProvider => ApiError::BadRequest,
            ServiceError::InvalidTemplate => ApiError::BadRequest,
            ServiceError::ExportFailed => ApiError::InternalServerError,
        }
    }
}

impl From<TemplateAdminError> for ApiError {
    fn from(err: TemplateAdminError) -> Self {
        match err {
            TemplateAdminError::InvalidPlaceholder(_) => ApiError::BadRequest,
            TemplateAdminError::Repository(repo_err) => match repo_err {
                RepositoryError::NotFound => ApiError::NotFound,
                RepositoryError::AlreadyExists => {
                    ApiError::ServiceError(ServiceError::AlreadyExists)
                }
                RepositoryError::Constraint(msg) => {
                    ApiError::ServiceError(ServiceError::Constraint(msg))
                }
                RepositoryError::Unexpected(_) => ApiError::InternalServerError,
            },
        }
    }
}

impl core::fmt::Display for ServiceError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}
