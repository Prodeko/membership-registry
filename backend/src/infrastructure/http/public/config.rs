use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use ts_rs::TS;

use super::super::AppState;

#[derive(Serialize, TS)]
#[ts(export, rename = "PublicConfig")]
struct PublicConfigDTO {
    keycloak_account_url: String,
    keycloak_admin_url: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(get_public_config))
}

async fn get_public_config(State(state): State<AppState>) -> Json<PublicConfigDTO> {
    let account_url = format!(
        "{}/realms/{}/account/",
        state.config.keycloak_url, state.config.keycloak_realm
    );
    let admin_url = format!(
        "{}/admin/{}/console/",
        state.config.keycloak_url, state.config.keycloak_realm
    );
    Json(PublicConfigDTO {
        keycloak_account_url: account_url,
        keycloak_admin_url: admin_url,
    })
}
