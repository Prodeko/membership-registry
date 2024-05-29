use axum::{extract::State, routing::get, Router};
use ory_client::apis::identity_api::list_identities;

use crate::api_types::ApiResult;

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new().route("/members", get(get_members)).with_state(state)
}

async fn get_members(State(state): State<AppState>) -> ApiResult<String> {
    let members = list_identities(
        &state.ory_config,
        Some(10),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None
    )
    .await;
    let members = members.unwrap().into_iter().map(|e| e.id);
    Ok(members.collect::<Vec<String>>().join(", ").into())
}
