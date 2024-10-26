use axum::{http::StatusCode, response::{IntoResponse, Response}};

use crate::services::errors::ServiceError;


pub type HttpResult<T> = Result<T, Response>;

impl IntoResponse for ServiceError {
  fn into_response(self) -> Response {
      match self {
          ServiceError::ApplicationAlreadyExists => {
              (StatusCode::BAD_REQUEST, "Application already exists").into_response()
          },
          ServiceError::ApplicationAlreadyProcessed => {
              (StatusCode::BAD_REQUEST, "Application already processed").into_response()
          },
          ServiceError::InvalidStatus => {
              (StatusCode::BAD_REQUEST, "Invalid status").into_response()
          },
          ServiceError::NotFound => {
              (StatusCode::NOT_FOUND, "Not found").into_response()
          },
          ServiceError::DatabaseError => {
              (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
          },
      }
  }
}

impl core::fmt::Display for ServiceError {
  fn fmt(&self, fmt: &mut std::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
      write!(fmt, "{self:?}")
  }
}