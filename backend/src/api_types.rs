use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_with::{serde_as};

pub type ApiResult<T> = core::result::Result<T, ApiError>;

#[serde_as]
#[derive(Serialize, Debug)]
pub enum ApiError {
    InternalServerError,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        println!("{:?}", self);
        match self {
            ApiError::InternalServerError => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Unhandled server error").into_response()
            }
        }
    }
}

impl core::fmt::Display for ApiError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(_val: sqlx::Error) -> Self {
        Self::InternalServerError
    }
}
