use axum::{
    body::Body,
    debug_handler,
    extract::{Path, Query, State},
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
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
        .route("/roles/export", post(export_members_with_roles))
        .route("/keycloak-sync-status", get(get_keycloak_sync_status))
        .route("/:user_id", delete(delete_member))
        .route("/:user_id/roles", post(add_role))
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
        .get_member_roles(user_id)
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
        has_accepted_policies: body.has_accepted_policies,
        email_notifications: body.email_notifications,
        language: body.language,
    };
    let result = state
        .member_service
        .update_member(user_id, data, actor_id)
        .await
        .map(MemberDTO::from)
        .map(Json)?;

    Ok(result)
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
        .map(|(user_id, status)| (user_id.to_string(), MemberKeycloakSyncStatusDTO::from(status)))
        .collect();

    Ok(Json(KeycloakSyncStatusMapDTO { statuses }))
}
