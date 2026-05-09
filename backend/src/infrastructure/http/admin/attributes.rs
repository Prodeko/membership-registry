use std::collections::HashMap;

use axum::{
    debug_handler,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};
use uuid::Uuid;

use crate::{
    application::{
        ports::attribute_repository_port::{CreateAttributeDefinition, UpdateAttributeDefinition},
        services::{
            attribute_service::{AttributeSyncStatus, SyncMissingAttributesSummary},
            authentication_service::AuthenticatedUser,
        },
    },
    domain::{AttributeName, AttributeValue},
    infrastructure::http::{
        dto::attribute::{
            editable_for_admin, parse_allowed_values, AttributeDefinitionDTO,
            CreateAttributeDefinitionDTO, MemberAttributeDTO, SetMemberAttributeDTO,
            UpdateAttributeDefinitionDTO,
        },
        errors::{ApiError, ApiResult},
    },
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_definitions).post(create_definition))
        .route("/sync-status", get(get_sync_status))
        .route("/sync-missing", post(sync_missing))
        .route(
            "/{name}",
            get(get_definition)
                .put(update_definition)
                .delete(delete_definition),
        )
        .with_state(state)
}

fn parse_name(s: String) -> ApiResult<AttributeName> {
    AttributeName::new(s).map_err(|_| ApiError::BadRequest)
}

#[debug_handler]
async fn list_definitions(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<AttributeDefinitionDTO>>> {
    let defs = state.attribute_service.list_definitions().await?;
    Ok(Json(defs.into_iter().map(Into::into).collect()))
}

#[debug_handler]
async fn get_definition(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<Json<AttributeDefinitionDTO>> {
    let n = parse_name(name)?;
    let def = state
        .attribute_service
        .get_definition(&n)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(def.into()))
}

#[debug_handler]
async fn create_definition(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Json(body): Json<CreateAttributeDefinitionDTO>,
) -> ApiResult<Json<AttributeDefinitionDTO>> {
    let actor_id = user_info.map(|u| u.user_id);
    let name = parse_name(body.name)?;
    let allowed_values =
        parse_allowed_values(body.allowed_values).map_err(|_| ApiError::BadRequest)?;
    let created = state
        .attribute_service
        .create_definition(
            CreateAttributeDefinition {
                name,
                description: body.description,
                allowed_values,
                sync_to_keycloak: body.sync_to_keycloak,
                editable_by: body.editable_by.into(),
            },
            actor_id,
        )
        .await?;
    Ok(Json(created.into()))
}

#[debug_handler]
async fn update_definition(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(body): Json<UpdateAttributeDefinitionDTO>,
) -> ApiResult<Json<AttributeDefinitionDTO>> {
    let actor_id = user_info.map(|u| u.user_id);
    let n = parse_name(name)?;
    let allowed_values =
        parse_allowed_values(body.allowed_values).map_err(|_| ApiError::BadRequest)?;
    let updated = state
        .attribute_service
        .update_definition(
            &n,
            UpdateAttributeDefinition {
                description: body.description,
                allowed_values,
                sync_to_keycloak: body.sync_to_keycloak,
                editable_by: body.editable_by.into(),
            },
            actor_id,
        )
        .await?;
    Ok(Json(updated.into()))
}

#[debug_handler]
async fn delete_definition(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    let actor_id = user_info.map(|u| u.user_id);
    let n = parse_name(name)?;
    state
        .attribute_service
        .delete_definition(&n, actor_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[debug_handler]
async fn get_sync_status(State(state): State<AppState>) -> ApiResult<Json<AttributeSyncStatus>> {
    Ok(Json(
        state.attribute_service.get_keycloak_sync_status().await?,
    ))
}

#[debug_handler]
async fn sync_missing(
    State(state): State<AppState>,
) -> ApiResult<Json<SyncMissingAttributesSummary>> {
    Ok(Json(
        state.attribute_service.sync_missing_to_keycloak().await?,
    ))
}

// ---------------------------------------------------------------------------
// Per-member attribute routes (mounted under /admin/members/{user_id}/attributes)
// ---------------------------------------------------------------------------

pub fn member_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/{user_id}/attributes", get(get_member_attributes_admin))
        .route(
            "/{user_id}/attributes/{name}",
            axum::routing::put(set_member_attribute_admin).delete(clear_member_attribute_admin),
        )
        .with_state(state)
}

#[debug_handler]
async fn get_member_attributes_admin(
    Path(user_id): Path<Uuid>,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<MemberAttributeDTO>>> {
    let defs = state.attribute_service.list_definitions().await?;
    let values: HashMap<String, String> = state
        .attribute_service
        .fetch_for_member(user_id)
        .await?
        .into_iter()
        .map(|a| (a.name.into_inner(), a.value.into_inner()))
        .collect();

    let dtos: Vec<MemberAttributeDTO> = defs
        .into_iter()
        .map(|d| MemberAttributeDTO {
            editable: editable_for_admin(&d),
            value: values.get(d.name().as_str()).cloned(),
            allowed_values: d
                .allowed_values()
                .map(|vs| vs.iter().map(|v| v.as_str().to_string()).collect()),
            description: d.description().map(str::to_string),
            name: d.name().clone().into_inner(),
        })
        .collect();
    Ok(Json(dtos))
}

#[debug_handler]
async fn set_member_attribute_admin(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Path((user_id, name)): Path<(Uuid, String)>,
    Json(body): Json<SetMemberAttributeDTO>,
) -> ApiResult<StatusCode> {
    let actor_id = user_info.map(|u| u.user_id);
    let n = parse_name(name)?;
    let v = AttributeValue::new(body.value).map_err(|_| ApiError::BadRequest)?;
    state
        .attribute_service
        .set_as_admin(user_id, &n, v, actor_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[debug_handler]
async fn clear_member_attribute_admin(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Path((user_id, name)): Path<(Uuid, String)>,
) -> ApiResult<StatusCode> {
    let actor_id = user_info.map(|u| u.user_id);
    let n = parse_name(name)?;
    state
        .attribute_service
        .clear_as_admin(user_id, &n, actor_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
