use axum::{
    debug_handler,
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    repositories::role::RoleMember,
    services::member_service::{MemberWithRoles, MemberWithUser, MemberWithoutId},
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/members", get(get_members))
        .route("/members", post(post_member))
        .route("/members", delete(delete_many))
        .route("/members/roles", get(get_members_with_roles))
        .route("/members/roles", post(add_many_roles))
        .route("/members/:user_id", get(get_member))
        .route("/members/:user_id", put(update_member))
        .route("/members/:user_id", delete(delete_member))
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
) -> Result<Json<Vec<MemberWithUser>>, String> {
    let user_ids = query
        .user_ids
        .clone()
        .map(|s| {
            s.split(',')
                .map(|uuid| Uuid::parse_str(uuid).map_err(|e| e.to_string()))
                .collect::<Result<Vec<Uuid>, _>>().ok()
        }).flatten();

    println!("User ids: {:?}", user_ids);
    println!("user ids query: {:?}", query.user_ids.clone());

    let members = state
        .member_service
        .get_all_members_with_user(user_ids)
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

    println!("Query: {:?}", query);

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
async fn post_member(
    State(state): State<AppState>,
    Json(new_member): Json<MemberWithoutId>,
) -> Result<Json<MemberWithUser>, String> {
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
) -> Result<Json<MemberWithUser>, String> {
    let member = state.member_service.get_member(user_id).await;

    if let Err(e) = &member {
        println!("Error fetching member: {:?}", e);
    }

    member.map(Json).map_err(|e| e.to_string())
}

#[debug_handler]
async fn update_member(
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
    Json(updated_member): Json<MemberWithUser>,
) -> Result<Json<MemberWithUser>, String> {
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
