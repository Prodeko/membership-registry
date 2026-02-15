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
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::{
    http::errors::{ApiError, ApiResult},
    repositories::{
        member::{Member, MemberWithRoles},
        role::RoleMember,
    },
    services::auth0_service::AuthInfo,
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
#[ts(export)]
struct MembersQuery {
    user_ids: Option<String>,
}
async fn get_members(
    State(state): State<AppState>,
    Query(query): Query<MembersQuery>,
) -> ApiResult<Json<Vec<Member>>> {
    let user_ids = query
        .user_ids
        .clone()
        .map(|s| {
            s.split(',')
                .map(|uuid| {
                    Uuid::parse_str(uuid).map_err(|e| (StatusCode::BAD_REQUEST, "Invalid UUID"))
                })
                .collect::<Result<Vec<Uuid>, _>>()
                .ok()
        })
        .flatten();

    println!("User ids: {:?}", user_ids);
    println!("user ids query: {:?}", query.user_ids.clone());

    let members = state
        .member_service
        .get_members_with_ids(user_ids)
        .await
        .map(Json)?;

    Ok(members)
}

#[derive(Deserialize, Debug, TS)]
#[ts(export)]
struct MembersWithRolesQuery {
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
    Query(query): Query<MembersWithRolesQuery>,
) -> ApiResult<Json<Vec<MemberWithRoles>>> {
    let roles = query.roles.clone().map(|roles| {
        roles
            .split(',')
            .map(|role| role.to_string())
            .collect::<Vec<String>>()
    });

    let members = state
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
        .await
        .map(Json)?;

    Ok(members)
}

#[debug_handler]
async fn export_members_with_roles(
    State(state): State<AppState>,
    Query(query): Query<MembersWithRolesQuery>,
) -> ApiResult<Response<Body>> {
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

    // TODO make this more general, CSV is needed elsewhere also
    // Write CSV to a string buffer
    let mut wtr = WriterBuilder::new().from_writer(vec![]);
    for record in members {
        wtr.write_record(&record.to_csv_row())
            .map_err(|_| ApiError::InternalServerError)?; 
    }
    let csv_data = wtr
        .into_inner()
        .map_err(|_| ApiError::InternalServerError)?; 

    // Write the CSV data to a temporary file
    let mut file = tokio::fs::File::create("/tmp/data.csv")
        .await
        .map_err(|_| ApiError::InternalServerError)?; 

    file.write_all(&csv_data)
        .await
        .map_err(|_| ApiError::InternalServerError)?; 

    // Read the file and respond with its content
    let file = tokio::fs::File::open("/tmp/data.csv")
        .await
        .map_err(|_| ApiError::InternalServerError)?; 

    let stream = tokio_util::io::ReaderStream::new(file);

    // Create the response
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/csv")
        .header("Content-Disposition", "attachment; filename=\"data.csv\"")
        .body(Body::from_stream(stream))
        .map_err(|_| ApiError::InternalServerError)?;

    Ok(response)
}

#[debug_handler]
async fn get_member(
    Extension(user_info): Extension<Option<AuthInfo>>,
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
) -> ApiResult<Json<Member>> {
    let member = state
        .member_service
        .get_member_with_user(user_id, user_info.unwrap().access_token)
        .await
        .map(Json)?;

    Ok(member)
}

#[debug_handler]
async fn get_member_roles(
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
) -> ApiResult<Json<Vec<RoleMember>>> {
    let roles = state
        .role_service
        .get_member_roles(user_id)
        .await
        .map(Json)?;

    Ok(roles)
}

#[debug_handler]
async fn update_member(
    Extension(user_info): Extension<Option<AuthInfo>>,
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
    Json(updated_member): Json<Member>,
) -> ApiResult<Json<Member>> {
    let result = state
        .member_service
        .update_member(updated_member, user_info.unwrap().access_token)
        .await
        .map(Json)?;

    Ok(result)
}

#[debug_handler]
async fn delete_member(
    Extension(user_info): Extension<Option<AuthInfo>>,
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
) -> ApiResult<()> {
    let result = state
        .member_service
        .delete_member(user_id, user_info.unwrap().access_token)
        .await?;

    Ok(result)
}

#[derive(Deserialize, TS)]
#[ts(export)]
struct DeleteManyBody {
    ids: Vec<Uuid>,
}

async fn delete_many(
    Extension(user_info): Extension<Option<AuthInfo>>,
    State(state): State<AppState>,
    Json(query): Json<DeleteManyBody>,
) -> ApiResult<()> {
    let result = state
        .member_service
        .delete_many(query.ids, user_info.unwrap().access_token)
        .await?;

    Ok(result)
}

#[derive(Deserialize, TS)]
#[ts(export)]
struct RoleMemberBody {
    role_name: String,
    valid_from: chrono::NaiveDate,
    valid_until: Option<chrono::NaiveDate>,
}

#[debug_handler]
async fn add_role(
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
    Json(query): Json<RoleMemberBody>,
) -> ApiResult<()> {
    let result = state
        .role_service
        .add_role_member(user_id, &query.role_name, query.valid_from, query.valid_until)
        .await?;

    Ok(result)
}

#[derive(Deserialize, TS)]
#[ts(export)]
struct AddManyRolesBody {
    user_ids: Vec<Uuid>,
    role_names: Vec<String>,
    valid_from: chrono::NaiveDate,
    valid_until: Option<chrono::NaiveDate>,
}

#[debug_handler]
async fn add_many_roles(
    State(state): State<AppState>,
    Json(query): Json<AddManyRolesBody>,
) -> ApiResult<()> {
    let result = state
        .role_service
        .add_many_role_members(query.user_ids, query.role_names, query.valid_from, query.valid_until)
        .await?;

    Ok(result)
}
