use axum::{
    body::Body,
    debug_handler,
    extract::{Path, Query, State},
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    application::{
        ports::data_export_port::ExportedData, services::authentication_service::AuthenticatedUser,
    },
    domain::UpdatePersonData,
    infrastructure::http::{
        dto::{
            member::{
                KeycloakSyncStatusMapDTO, MemberDTO, MemberKeycloakSyncStatusDTO,
                MemberWithRolesDTO, UpdateMemberDTO,
            },
            role::RoleMembershipDTO,
            role_group::RoleGroupMembershipDTO,
        },
        errors::{ApiError, ApiResult},
    },
};

use super::AppState;

pub fn build_export_response(data: ExportedData) -> Result<Response<Body>, ApiError> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", &data.content_type)
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"data.{}\"", data.file_extension),
        )
        .body(Body::from(data.bytes))
        .map_err(|_| ApiError::InternalServerError)
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(get_members))
        .route("/", delete(delete_many))
        .route("/roles", get(get_members_with_roles))
        .route("/roles", post(add_many_roles))
        .route("/roles/count", get(count_members_with_roles))
        .route("/roles/export", post(export_members_with_roles))
        .route("/keycloak-sync-status", get(get_keycloak_sync_status))
        .route("/keycloak-sync", post(post_keycloak_sync))
        .route("/{user_id}", delete(delete_member))
        .route("/{user_id}", put(update_member))
        .route("/{user_id}/roles", post(add_role))
        .route("/{user_id}/roles", delete(remove_role))
        .route("/{user_id}/role-groups", get(get_member_role_groups))
        .with_state(state)
}

#[derive(Deserialize, Debug, TS)]
#[ts(export, rename = "MembersQuery")]
struct MembersQueryDTO {
    user_ids: Option<String>,
}
async fn get_members(
    State(state): State<AppState>,
    Query(query): Query<MembersQueryDTO>,
) -> ApiResult<Json<Vec<MemberDTO>>> {
    let user_ids = query
        .user_ids
        .clone()
        .map(|s| {
            s.split(',')
                .map(|uuid| Uuid::parse_str(uuid).map_err(|_| ApiError::BadRequest))
                .collect::<Result<Vec<Uuid>, _>>()
        })
        .transpose()?;

    tracing::debug!("User ids: {:?}", user_ids);
    tracing::debug!("user ids query: {:?}", query.user_ids.clone());

    let members: Vec<MemberDTO> = state
        .member_service
        .get_members_with_ids(user_ids)
        .await?
        .into_iter()
        .map(MemberDTO::from)
        .collect();

    Ok(Json(members))
}

#[derive(Deserialize, Debug, TS)]
#[ts(export, rename = "MembersWithRolesQuery")]
struct MembersWithRolesQueryDTO {
    page_size: Option<u64>,
    offset: Option<u64>,
    search: Option<String>,
    sorting: Option<String>,
    sort_desc: Option<bool>,
    roles: Option<String>,
    valid_from: Option<chrono::NaiveDate>,
    valid_until: Option<chrono::NaiveDate>,
}

#[debug_handler]
async fn get_members_with_roles(
    State(state): State<AppState>,
    Query(query): Query<MembersWithRolesQueryDTO>,
) -> ApiResult<Json<Vec<MemberWithRolesDTO>>> {
    let roles = query.roles.clone().map(|roles| {
        roles
            .split(',')
            .map(|role| role.to_string())
            .collect::<Vec<String>>()
    });

    let members: Vec<MemberWithRolesDTO> = state
        .member_service
        .get_members_with_roles(
            query.page_size,
            query.offset,
            roles,
            query.search,
            query.sorting,
            query.sort_desc,
            query.valid_from,
            query.valid_until,
        )
        .await?
        .into_iter()
        .map(MemberWithRolesDTO::from)
        .collect();

    Ok(Json(members))
}

#[derive(Serialize, Debug, TS)]
#[ts(export, rename = "MembersCount")]
struct MembersCountDTO {
    total: i64,
}

#[debug_handler]
async fn count_members_with_roles(
    State(state): State<AppState>,
    Query(query): Query<MembersWithRolesQueryDTO>,
) -> ApiResult<Json<MembersCountDTO>> {
    let roles = query.roles.clone().map(|roles| {
        roles
            .split(',')
            .map(|role| role.to_string())
            .collect::<Vec<String>>()
    });

    let total = state
        .member_service
        .count_members_with_roles(roles, query.search, query.valid_from, query.valid_until)
        .await?;

    Ok(Json(MembersCountDTO { total }))
}

#[debug_handler]
async fn export_members_with_roles(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Query(query): Query<MembersWithRolesQueryDTO>,
) -> ApiResult<Response<Body>> {
    let actor_id = user_info.map(|u| u.user_id);
    state.member_service.log_export(actor_id).await;

    let roles = query.roles.clone().map(|roles| {
        roles
            .split(',')
            .map(|role| role.to_string())
            .collect::<Vec<String>>()
    });

    let members = state
        .member_service
        .get_members_with_roles(
            None, // Ignore pagination
            None,
            roles,
            query.search,
            query.sorting,
            query.sort_desc,
            query.valid_from,
            query.valid_until,
        )
        .await?;

    let exported = state.export_service.export(&members)?;
    build_export_response(exported)
}

#[debug_handler]
async fn get_member(
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
) -> ApiResult<Json<MemberDTO>> {
    let member = state
        .member_service
        .get_member_with_user(user_id)
        .await
        .map(MemberDTO::from)
        .map(Json)?;

    Ok(member)
}

#[debug_handler]
async fn get_member_roles(
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
) -> ApiResult<Json<Vec<RoleMembershipDTO>>> {
    let roles: Vec<RoleMembershipDTO> = state
        .role_service
        .get_member_roles_with_prompts(user_id)
        .await?
        .into_iter()
        .map(RoleMembershipDTO::from)
        .collect();

    Ok(Json(roles))
}

#[debug_handler]
async fn update_member(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
    Json(body): Json<UpdateMemberDTO>,
) -> ApiResult<Json<MemberDTO>> {
    let actor_id = user_info.map(|u| u.user_id);
    let data = UpdatePersonData {
        first_name: body.first_name,
        last_name: body.last_name,
        home_municipality: body.home_municipality,
        email_notifications: body.email_notifications,
        language: body.language,
        email: body.email,
    };
    let person = state
        .member_service
        .update_member(user_id, data, actor_id)
        .await?;

    Ok(Json(MemberDTO::from(person)))
}

#[debug_handler]
async fn delete_member(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
) -> ApiResult<()> {
    let actor_id = user_info.map(|u| u.user_id);
    state
        .member_service
        .delete_member(user_id, actor_id)
        .await?;

    Ok(())
}

#[derive(Deserialize, TS)]
#[ts(export, rename = "DeleteManyBody")]
struct DeleteManyBodyDTO {
    ids: Vec<Uuid>,
}

async fn delete_many(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Json(query): Json<DeleteManyBodyDTO>,
) -> ApiResult<()> {
    let actor_id = user_info.map(|u| u.user_id);
    state
        .member_service
        .delete_many(query.ids, actor_id)
        .await?;

    Ok(())
}

#[derive(Deserialize, TS)]
#[ts(export, rename = "RoleMemberBody")]
struct RoleMemberBodyDTO {
    role_name: String,
    valid_from: chrono::NaiveDate,
    valid_until: Option<chrono::NaiveDate>,
}

#[debug_handler]
async fn add_role(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
    Json(query): Json<RoleMemberBodyDTO>,
) -> ApiResult<()> {
    let actor_id = user_info.map(|u| u.user_id);
    state
        .role_service
        .add_role_member(
            user_id,
            &query.role_name,
            query.valid_from,
            query.valid_until,
            actor_id,
        )
        .await?;

    Ok(())
}

#[derive(Deserialize, TS)]
#[ts(export, rename = "AddManyRolesBody")]
struct AddManyRolesBodyDTO {
    user_ids: Vec<Uuid>,
    role_names: Vec<String>,
    valid_from: chrono::NaiveDate,
    valid_until: Option<chrono::NaiveDate>,
}

#[debug_handler]
async fn add_many_roles(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Json(query): Json<AddManyRolesBodyDTO>,
) -> ApiResult<()> {
    let actor_id = user_info.map(|u| u.user_id);
    state
        .role_service
        .add_many_role_members(
            query.user_ids,
            query.role_names,
            query.valid_from,
            query.valid_until,
            actor_id,
        )
        .await?;

    Ok(())
}

#[debug_handler]
async fn get_keycloak_sync_status(
    State(state): State<AppState>,
) -> ApiResult<Json<KeycloakSyncStatusMapDTO>> {
    let status_map = state.role_service.get_keycloak_sync_status().await?;

    let statuses = status_map
        .into_iter()
        .map(|(user_id, status)| {
            (
                user_id.to_string(),
                MemberKeycloakSyncStatusDTO::from(status),
            )
        })
        .collect();

    Ok(Json(KeycloakSyncStatusMapDTO { statuses }))
}

#[derive(serde::Deserialize, Default)]
struct KeycloakSyncRequestDTO {
    #[serde(default)]
    remove_expired: bool,
}

#[derive(serde::Serialize)]
struct KeycloakSyncResponseDTO {
    added: u32,
    failed: u32,
    removed: u32,
    remove_failed: u32,
    users_processed: u32,
}

#[debug_handler]
async fn post_keycloak_sync(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    body: Option<Json<KeycloakSyncRequestDTO>>,
) -> ApiResult<Json<KeycloakSyncResponseDTO>> {
    let actor_id = user_info.map(|u| u.user_id);
    let remove_expired = body.map(|b| b.remove_expired).unwrap_or(false);
    let summary = state
        .role_service
        .sync_missing_roles_to_keycloak(actor_id, remove_expired)
        .await?;

    Ok(Json(KeycloakSyncResponseDTO {
        added: summary.added,
        failed: summary.failed,
        removed: summary.removed,
        remove_failed: summary.remove_failed,
        users_processed: summary.users_processed,
    }))
}

#[derive(Deserialize)]
struct RemoveRoleQuery {
    role_name: String,
    valid_from: chrono::NaiveDate,
}

#[debug_handler]
async fn remove_role(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
    Query(query): Query<RemoveRoleQuery>,
) -> ApiResult<()> {
    let actor_id = user_info.map(|u| u.user_id);
    state
        .role_service
        .delete_role_membership(user_id, &query.role_name, query.valid_from, actor_id)
        .await?;
    Ok(())
}

async fn get_member_role_groups(
    Path(user_id): Path<Uuid>,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<RoleGroupMembershipDTO>>> {
    let groups = state
        .role_group_service
        .get_member_groups(&user_id)
        .await?
        .into_iter()
        .map(RoleGroupMembershipDTO::from)
        .collect();
    Ok(Json(groups))
}
