use std::collections::HashMap;

use axum::{
    debug_handler,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
    Extension, Json, Router,
};

use crate::{
    application::services::authentication_service::AuthenticatedUser,
    domain::{AttributeName, AttributeValue},
    infrastructure::http::{
        dto::attribute::{editable_for_self, MemberAttributeDTO, SetMemberAttributeDTO},
        errors::{ApiError, ApiResult},
    },
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/me", get(get_my_attributes))
        .route(
            "/me/{name}",
            put(set_my_attribute).delete(clear_my_attribute),
        )
        .with_state(state)
}

#[debug_handler]
async fn get_my_attributes(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<MemberAttributeDTO>>> {
    let user = user_info.ok_or(ApiError::Unauthorized)?;
    let defs = state.attribute_service.list_definitions().await?;
    let values: HashMap<String, String> = state
        .attribute_service
        .fetch_for_member(user.user_id)
        .await?
        .into_iter()
        .map(|a| (a.name.into_inner(), a.value.into_inner()))
        .collect();
    let dtos: Vec<MemberAttributeDTO> = defs
        .into_iter()
        .map(|d| MemberAttributeDTO {
            editable: editable_for_self(&d),
            value: values.get(d.name.as_str()).cloned(),
            allowed_values: d.allowed_values.clone(),
            description: d.description.clone(),
            name: d.name.into_inner(),
        })
        .collect();
    Ok(Json(dtos))
}

#[debug_handler]
async fn set_my_attribute(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(body): Json<SetMemberAttributeDTO>,
) -> ApiResult<StatusCode> {
    let user = user_info.ok_or(ApiError::Unauthorized)?;
    let n = AttributeName::new(name).map_err(|_| ApiError::BadRequest)?;
    let v = AttributeValue::new(body.value).map_err(|_| ApiError::BadRequest)?;
    state.attribute_service.set_as_self(user.user_id, &n, v).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[debug_handler]
async fn clear_my_attribute(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    let user = user_info.ok_or(ApiError::Unauthorized)?;
    let n = AttributeName::new(name).map_err(|_| ApiError::BadRequest)?;
    state.attribute_service.clear_as_self(user.user_id, &n).await?;
    Ok(StatusCode::NO_CONTENT)
}
