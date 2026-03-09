use axum::{
    body::Body,
    debug_handler,
    extract::{Path, Query, State},
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Extension, Json, Router,
};
use csv::WriterBuilder;
use serde::Deserialize;
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    infrastructure::http::{
        dto::{
            member::{MemberDTO, MemberWithRolesDTO, UpdateMemberDTO},
            role::RoleMembershipDTO,
        },
        errors::{ApiError, ApiResult},
    },
    application::services::authentication_service::AuthenticatedUser,
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(get_members))
        .route("/", delete(delete_many))
        .route("/roles", get(get_members_with_roles))
        .route("/roles", post(add_many_roles))
        .route("/roles/export", post(export_members_with_roles))
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
        .and_then(|s| {
            s.split(',')
                .map(|uuid| {
                    Uuid::parse_str(uuid).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID"))
                })
                .collect::<Result<Vec<Uuid>, _>>()
                .ok()
        });

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

    let mut wtr = WriterBuilder::new().from_writer(Vec::new());
    for record in members {
        wtr.write_record(record.to_csv_row())
            .map_err(|_| ApiError::InternalServerError)?;
    }
    let csv_data = wtr
        .into_inner()
        .map_err(|_| ApiError::InternalServerError)?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/csv")
        .header("Content-Disposition", "attachment; filename=\"data.csv\"")
        .body(Body::from(csv_data))
        .map_err(|_| ApiError::InternalServerError)?;

    Ok(response)
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
    let result = state
        .member_service
        .update_member(user_id, body.first_name, body.last_name, body.home_municipality, body.has_accepted_policies, actor_id)
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
        .add_role_member(user_id, &query.role_name, query.valid_from, query.valid_until, actor_id)
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
        .add_many_role_members(query.user_ids, query.role_names, query.valid_from, query.valid_until, actor_id)
        .await?;

    Ok(())
}
