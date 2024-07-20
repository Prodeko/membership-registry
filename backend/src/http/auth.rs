use axum::{
    extract::State,
    response::{Html, Redirect},
    routing::get,
    Router,
};
use oauth2::{reqwest::async_http_client, AuthorizationCode, CsrfToken, Scope, TokenResponse};

use super::AppState;

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/login", get(login))
        .route("/callback", get(callback))
        .with_state(state)
}

async fn login(State(state): State<AppState>) -> Redirect {
    let (auth_url, _csrf_token) = state
        .oauth2_client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".into()))
        .url();

    Redirect::temporary(auth_url.to_string().as_str())
}

async fn callback(
    State(state): State<AppState>,
    query: axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Html<String> {
    if let (Some(code), Some(_state_param)) = (query.get("code"), query.get("state")) {
        let token_result = state
            .oauth2_client
            .exchange_code(AuthorizationCode::new(code.clone()))
            .request_async(async_http_client)
            .await;

        match token_result {
            Ok(token) => Html(format!("Access Token: {:?}", token.access_token().secret())),
            Err(err) => Html(format!("Error: {:?}", err)),
        }
    } else {
        Html("Missing code or state".to_string())
    }
}
