use std::mem;

use axum::{
    body::Body,
    debug_handler,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, Response, StatusCode},
    routing::{delete, get, post, put},
    Json, Router,
};
use csv::WriterBuilder;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::{
    repositories::{member::{Member, MemberWithRoles}, role::RoleMember},
    services::member_service::MemberWithoutUserId,
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/members", get(get_members))
        .route("/members", post(post_member))
        .route("/members", delete(delete_many))
        .route("/members/roles", get(get_members_with_roles))
        .route("/members/roles", post(add_many_roles))
        .route("/members/roles/export", post(export_members_with_roles))
        .route("/members/:user_id", get(get_member))
        .route("/members/:user_id", put(update_member))
        .route("/members/:user_id", delete(delete_member))
        .route("/members/:user_id/roles", get(get_member_roles))
        .route("/members/:user_id/roles", post(add_role))
        .with_state(state)
}

#[derive(Deserialize, Debug)]
struct MembersQuery {
    user_ids: Option<String>,
}
async fn get_members(
    State(state): State<AppState>,
    Query(query): Query<MembersQuery>,
) -> Result<Json<Vec<Member>>, String> {
    let user_ids = query
        .user_ids
        .clone()
        .map(|s| {
            s.split(',')
                .map(|uuid| Uuid::parse_str(uuid).map_err(|e| e.to_string()))
                .collect::<Result<Vec<Uuid>, _>>()
                .ok()
        })
        .flatten();

    println!("User ids: {:?}", user_ids);
    println!("user ids query: {:?}", query.user_ids.clone());

    let members = state
        .member_service
        .get_members_with_ids(user_ids)
        .await;

    if let Err(e) = &members {
        println!("Error fetching members: {:?}", e);
    }

    members.map(Json).map_err(|e| e.to_string())
}

#[derive(Deserialize, Debug)]
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
) -> Result<Json<Vec<MemberWithRoles>>, String> {
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
        .await;

    if let Err(e) = &members {
        println!("Error fetching members: {:?}", e);
    }

    members.map(Json)
}

#[debug_handler]
async fn export_members_with_roles(
    State(state): State<AppState>,
    Query(query): Query<MembersWithRolesQuery>,
) -> Result<Response<Body>, StatusCode> {
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
        .await;

    if let Err(e) = &members {
        println!("Error fetching members: {:?}", e);
    }

    match members {
        Ok(members) => {
            // Write CSV to a string buffer
            let mut wtr = WriterBuilder::new().from_writer(vec![]);
            for record in members {
                wtr.write_record(&record.to_csv_row())
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
            let csv_data = wtr
                .into_inner()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // Write the CSV data to a temporary file
            let mut file = tokio::fs::File::create("/tmp/data.csv")
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            file.write_all(&csv_data)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // Read the file and respond with its content
            let file = tokio::fs::File::open("/tmp/data.csv")
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let stream = tokio_util::io::ReaderStream::new(file);

            // Create the response
            let response = Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/csv")
                .header("Content-Disposition", "attachment; filename=\"data.csv\"")
                .body(Body::from_stream(stream))
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            Ok(response)
        }
        Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[debug_handler]
async fn post_member(
    State(state): State<AppState>,
    Json(new_member): Json<MemberWithoutUserId>,
) -> Result<Json<Member>, String> {
    let member = state.member_service.create_member(new_member).await;

    if let Err(e) = &member {
        println!("Error creating member: {:?}", e);
    }

    member.map(Json).map_err(|e| e.to_string())
}

#[debug_handler]
async fn get_member(
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
) -> Result<Json<Member>, axum::http::StatusCode> {
    let member = state.member_service.get_member(user_id).await;

    if let Err(e) = &member {
        println!("Error fetching member: {:?}", e);
    }

    // TODO: Return proper status code
    member
        .map(Json)
        .map_err(|_e| axum::http::StatusCode::NOT_FOUND)
}

#[debug_handler]
async fn get_member_roles(
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
) -> Result<Json<Vec<RoleMember>>, axum::http::StatusCode> {
    let roles = state.role_service.get_member_roles(user_id).await;

    if let Err(e) = &roles {
        println!("Error fetching member roles: {:?}", e);
    }

    // TODO return proper status code
    roles
        .map(Json)
        .map_err(|_e| axum::http::StatusCode::NOT_FOUND)
}

#[debug_handler]
async fn update_member(
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
    Json(updated_member): Json<Member>,
) -> Result<Json<Member>, String> {
    let member = state.member_service.update_member(updated_member).await;

    if let Err(e) = &member {
        println!("Error updating member: {:?}", e);
    }

    member.map(Json).map_err(|e| e.to_string())
}

#[debug_handler]
async fn delete_member(
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
) -> Result<(), String> {
    state
        .member_service
        .delete_member(user_id)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Deserialize)]
struct DeleteManyBody {
    ids: Vec<Uuid>,
}

async fn delete_many(
    State(state): State<AppState>,
    Json(query): Json<DeleteManyBody>,
) -> Result<(), String> {
    state
        .member_service
        .delete_many(query.ids)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Deserialize)]
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
) -> Result<(), String> {
    let result = state
        .role_service
        .add_role_member(
            user_id,
            &query.role_name,
            query.valid_from,
            query.valid_until,
        )
        .await;

    if let Err(e) = &result {
        println!("Error adding role: {:?}", e);
    }

    result.map(|_| ()).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
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
) -> Result<(), String> {
    let result = state
        .role_service
        .add_many_role_members(
            query.user_ids,
            query.role_names,
            query.valid_from,
            query.valid_until,
        )
        .await;

    if let Err(e) = &result {
        println!("Error adding roles: {:?}", e);
    }

    result.map(|_| ()).map_err(|e| e.to_string())
}
