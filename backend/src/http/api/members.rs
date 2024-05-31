use axum::{
    debug_handler,
    extract::State,
    routing::{get, post},
    Json, Router,
};

use crate::services::member_service::{MemberWithUser, MemberWithoutId};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/members", get(get_members))
        .route("/members", post(post_member))
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
