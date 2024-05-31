use axum::{
    debug_handler,
    extract::State,
    routing::{get, post},
    Json, Router,
};
use uuid::uuid;

use crate::{
    api_types::ApiResult,
    repositories::member::{Member, NewMember},
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/members", get(get_members))
        .with_state(state)
}

#[debug_handler]
async fn get_members(State(state): State<AppState>) -> Result<Json<Vec<Member>>, String> {
    let members = state.member_service.get_all_members().await;

    if let Err(e) = &members {
        println!("Error fetching members: {:?}", e);
    }

    members.map(Json)
}