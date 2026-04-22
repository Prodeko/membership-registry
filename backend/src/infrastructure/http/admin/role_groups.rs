use axum::{
    debug_handler,
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use chrono::NaiveDate;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    application::services::authentication_service::AuthenticatedUser,
    domain::RoleName,
    infrastructure::http::{
        dto::role_group::{
            AssignRoleGroupRequestDTO, CreateRoleGroupRequestDTO, RoleGroupDTO,
            RoleGroupMembershipDTO, SetRoleGroupRolesDTO, UpdateRoleGroupDTO,
        },
        errors::ApiResult,
    },
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(get_role_groups))
        .route("/", post(post_role_group))
        .route("/{id}", get(get_role_group))
        .route("/{id}", put(update_role_group))
        .route("/{id}", delete(delete_role_group))
        .route("/{id}/roles", put(put_role_group_roles))
        .route("/{id}/members", get(get_role_group_members))
        .route("/{id}/members", post(post_role_group_member))
        .route("/{id}/members/{user_id}", delete(delete_role_group_member))
        .with_state(state)
}

async fn get_role_groups(State(state): State<AppState>) -> ApiResult<Json<Vec<RoleGroupDTO>>> {
    let groups = state
        .role_group_service
        .get_all_groups()
        .await?
        .into_iter()
        .map(RoleGroupDTO::from)
        .collect();
    Ok(Json(groups))
}

#[debug_handler]
async fn post_role_group(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Json(body): Json<CreateRoleGroupRequestDTO>,
) -> ApiResult<Json<RoleGroupDTO>> {
    let actor_id = user_info.map(|u| u.user_id);
    let role_names: Vec<RoleName> = body.role_names.into_iter().map(RoleName).collect();
    let group = state
        .role_group_service
        .create_group(
            &body.name,
            body.description.as_deref(),
            role_names,
            actor_id,
        )
        .await
        .map(RoleGroupDTO::from)
        .map(Json)?;
    Ok(group)
}

async fn get_role_group(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> ApiResult<Json<RoleGroupDTO>> {
    let group = state
        .role_group_service
        .get_group(&id)
        .await
        .map(RoleGroupDTO::from)
        .map(Json)?;
    Ok(group)
}

#[debug_handler]
async fn update_role_group(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(body): Json<UpdateRoleGroupDTO>,
) -> ApiResult<Json<RoleGroupDTO>> {
    let actor_id = user_info.map(|u| u.user_id);
    let group = state
        .role_group_service
        .update_group(&id, &body.name, body.description.as_deref(), actor_id)
        .await
        .map(RoleGroupDTO::from)
        .map(Json)?;
    Ok(group)
}

#[debug_handler]
async fn delete_role_group(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> ApiResult<()> {
    let actor_id = user_info.map(|u| u.user_id);
    state.role_group_service.delete_group(&id, actor_id).await?;
    Ok(())
}

#[debug_handler]
async fn put_role_group_roles(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(body): Json<SetRoleGroupRolesDTO>,
) -> ApiResult<Json<RoleGroupDTO>> {
    let actor_id = user_info.map(|u| u.user_id);
    let role_names: Vec<RoleName> = body.role_names.into_iter().map(RoleName).collect();
    let group = state
        .role_group_service
        .set_group_roles(&id, role_names, actor_id)
        .await
        .map(RoleGroupDTO::from)
        .map(Json)?;
    Ok(group)
}

async fn get_role_group_members(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<RoleGroupMembershipDTO>>> {
    let members = state
        .role_group_service
        .get_group_members(&id)
        .await?
        .into_iter()
        .map(RoleGroupMembershipDTO::from)
        .collect();
    Ok(Json(members))
}

#[debug_handler]
async fn post_role_group_member(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(body): Json<AssignRoleGroupRequestDTO>,
) -> ApiResult<()> {
    let actor_id = user_info.map(|u| u.user_id);
    let valid_from = body
        .valid_from
        .unwrap_or_else(|| chrono::Local::now().date_naive());
    state
        .role_group_service
        .assign_group(&id, body.user_id, valid_from, body.valid_until, actor_id)
        .await?;
    Ok(())
}

#[derive(Deserialize)]
struct DeleteMemberQuery {
    valid_from: Option<NaiveDate>,
}

#[debug_handler]
async fn delete_role_group_member(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<DeleteMemberQuery>,
    State(state): State<AppState>,
) -> ApiResult<()> {
    let actor_id = user_info.map(|u| u.user_id);
    let valid_from = query
        .valid_from
        .unwrap_or_else(|| chrono::Local::now().date_naive());
    state
        .role_group_service
        .remove_group_assignment(&id, user_id, valid_from, actor_id)
        .await?;
    Ok(())
}
