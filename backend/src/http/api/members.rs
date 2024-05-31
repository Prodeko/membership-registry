use axum::{
    debug_handler,
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use uuid::Uuid;

use crate::services::member_service::{MemberWithUser, MemberWithoutId};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/members", get(get_members))
        .route("/members", post(post_member))
        .route("/members/:user_id", get(get_member))
        .route("/members/:user_id", put(update_member))
        .route("/members/:user_id", delete(delete_member))
        .with_state(state)
}

#[debug_handler]
async fn get_members(State(state): State<AppState>) -> Result<Json<Vec<MemberWithUser>>, String> {
    let members = state.member_service.get_all_members().await;

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
    state.member_service.delete_member(user_id).await.map_err(|e| e.to_string())
}
