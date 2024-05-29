use axum::{
    debug_handler,
    extract::State,
    routing::{get, post},
    Json, Router,
};

use crate::{
    api_types::ApiResult,
    repositories::member::{Member, NewMember},
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/members", get(get_members))
        .route("/members", post(create_member))
        .with_state(state)
}

#[debug_handler]
async fn get_members(State(state): State<AppState>) -> ApiResult<Json<Vec<Member>>> {
    let members = state.db.member.fetch_all().await;

    if let Err(e) = &members {
        println!("Error fetching members: {:?}", e);
    }

    members.map(Json).map_err(|e| e.into())
}

#[debug_handler]
async fn create_member(State(state): State<AppState>) -> ApiResult<Json<Member>> {
    let member = NewMember {
        user_id: "5229864b-1e8f-46ef-9640-4c7c0aabd574".to_string(),
        first_name: "John".to_string(),
        last_name: "Doe".to_string(),
        home_municipality: "Oslo".to_string(),
        has_accepted_policies: true,
    };
    let member = state.db.member.create(member).await;

    if let Err(e) = &member {
        println!("Error creating member: {:?}", e);
    }

    member.map(Json).map_err(|e| e.into())
}
