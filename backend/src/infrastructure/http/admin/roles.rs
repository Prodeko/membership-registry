use axum::{
    body::Body,
    debug_handler,
    extract::{Path, Query, State},
    http::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
use ts_rs::TS;

use crate::{
    application::services::authentication_service::AuthenticatedUser,
    domain::{Role, RoleName},
    infrastructure::http::{
        dto::{
            member::MemberDTO,
            role::{RoleDTO, RoleStatsDTO},
        },
        errors::ApiResult,
        types::RolePath,
    },
};

use super::{members::build_export_response, AppState};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(get_roles))
        .route("/", post(post_role))
        .route("/stats", get(get_roles_stats))
        .route("/export", post(export_roles))
        .route("/cleanup-expired", post(cleanup_expired_roles))
        .route("/:id", get(get_role))
        .route("/:id/members", get(get_role_members))
        .with_state(state)
}

async fn get_roles(State(state): State<AppState>) -> ApiResult<Json<Vec<RoleDTO>>> {
    let roles: Vec<RoleDTO> = state
        .role_service
        .get_all_roles()
        .await?
        .into_iter()
        .map(RoleDTO::from)
        .collect();

    Ok(Json(roles))
}

async fn get_role(
    Path(path): Path<RolePath>,
    State(state): State<AppState>,
) -> ApiResult<Json<RoleDTO>> {
    let role = state
        .role_service
        .get_role(&path.id)
        .await
        .map(RoleDTO::from)
        .map(Json)?;
    Ok(role)
}

async fn get_role_members(
    Path(path): Path<RolePath>,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<MemberDTO>>> {
    let members: Vec<MemberDTO> = state
        .role_service
        .get_role_members(&path.id)
        .await?
        .into_iter()
        .map(MemberDTO::from)
        .collect();
    Ok(Json(members))
}

#[derive(Deserialize, Debug, TS)]
#[ts(export, rename = "RolesWithStatsQuery")]
struct RolesWithStatsQueryDTO {
    page_size: Option<u64>,
    offset: Option<u64>,
    search: Option<String>,
    sorting: Option<String>,
    sort_desc: Option<bool>,
}

async fn get_roles_stats(
    State(state): State<AppState>,
    Query(query): Query<RolesWithStatsQueryDTO>,
) -> ApiResult<Json<Vec<RoleStatsDTO>>> {
    let result: Vec<RoleStatsDTO> = state
        .role_service
        .get_role_stats(
            query.page_size,
            query.offset,
            query.search,
            query.sorting,
            query.sort_desc,
        )
        .await?
        .into_iter()
        .map(RoleStatsDTO::from)
        .collect();

    Ok(Json(result))
}

#[derive(Deserialize, Debug, TS)]
#[ts(export, rename = "CreateRoleRequest")]
struct CreateRoleRequestDTO {
    name: String,
    color: Option<String>,
}

#[debug_handler]
async fn post_role(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Json(body): Json<CreateRoleRequestDTO>,
) -> ApiResult<Json<RoleDTO>> {
    let actor_id = user_info.map(|u| u.user_id);
    let new_role = Role {
        name: RoleName(body.name),
        color: body.color,
        description: None,
    };
    let role = state
        .role_service
        .create_role(&new_role, actor_id)
        .await
        .map(RoleDTO::from)
        .map(Json)?;

    Ok(role)
}

#[derive(serde::Serialize)]
struct CleanupExpiredResponseDTO {
    synced: u32,
}

#[debug_handler]
async fn cleanup_expired_roles(
    State(state): State<AppState>,
) -> ApiResult<Json<CleanupExpiredResponseDTO>> {
    let synced = state.role_service.cleanup_expired_roles().await?;
    Ok(Json(CleanupExpiredResponseDTO { synced }))
}

#[debug_handler]
async fn export_roles(
    State(state): State<AppState>,
    Query(query): Query<RolesWithStatsQueryDTO>,
) -> ApiResult<Response<Body>> {
    let roles = state
        .role_service
        .get_role_stats(
            query.page_size,
            query.offset,
            query.search,
            query.sorting,
            query.sort_desc,
        )
        .await?;

    let exported = state.export_service.export(&roles)?;
    build_export_response(exported)
}
