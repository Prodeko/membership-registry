use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::services::errors::{ServiceError, ServiceResult};

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
        match self {
            ServiceError::AlreadyExists => {
                (StatusCode::BAD_REQUEST, "Application already exists").into_response()
            }
            ServiceError::Constraint => {
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
            ServiceError::DatabaseError => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
            }
            ServiceError::OryError => (StatusCode::BAD_GATEWAY, "Ory error").into_response(),
        }
    }
}

impl From<ServiceError> for ApiError {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::AlreadyExists => ApiError::ServiceError(ServiceError::AlreadyExists),
            ServiceError::Constraint => ApiError::ServiceError(ServiceError::Constraint),
            ServiceError::InvalidInput => ApiError::ServiceError(ServiceError::InvalidInput),
            ServiceError::ApplicationAlreadyProcessed => {
                ApiError::ServiceError(ServiceError::ApplicationAlreadyProcessed)
            }
            ServiceError::InvalidStatus => ApiError::ServiceError(ServiceError::InvalidStatus),
            ServiceError::NotFound => ApiError::NotFound,
            ServiceError::Unauthorized => ApiError::Unauthorized,
            ServiceError::Forbidden => ApiError::Forbidden,
            ServiceError::DatabaseError => ApiError::InternalServerError,
            ServiceError::OryError => ApiError::ServiceError(ServiceError::OryError),
        }
    }
}

impl core::fmt::Display for ServiceError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}
