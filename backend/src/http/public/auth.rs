use axum::{extract::State, http::StatusCode, response::{IntoResponse, Redirect, Response}, routing::get, Json, Router};
use axum_extra::extract::CookieJar;
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
use serde::Serialize;

use crate::helpers::set_session_cookie;

use super::AppState;

#[derive(Serialize)]
struct CallbackResponse {
    redirect_to: String,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/login", get(login))
        .route("/callback", get(callback))
        .route("/logout", get(logout))
        .with_state(state)
}

async fn login(State(state): State<AppState>) -> Redirect {
    let (auth_url, _csrf_token) = state
        .oauth2_client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".into()))
        .add_scope(Scope::new("email".into()))
        .add_scope(Scope::new("profile".into()))
        .url();

    Redirect::temporary(auth_url.to_string().as_str())
}

async fn logout(State(state): State<AppState>, jar: CookieJar) -> (CookieJar, Json<String>) {
    let jar = set_session_cookie(&jar, "");
    (jar, Json("Logged out".to_string()))
}

async fn callback(
    State(state): State<AppState>,
    query: axum::extract::Query<std::collections::HashMap<String, String>>,
    jar: CookieJar,
) -> Result<(CookieJar, Json<CallbackResponse>), Response> {
    let (code, _state_param) = match (query.get("code"), query.get("state")) {
        (Some(code), Some(state)) => (code.clone(), state.clone()),
        _ => {
            tracing::error!("Missing code or state");
            return Err((StatusCode::BAD_REQUEST, "Missing code or state").into_response());
        }
    };

    let token = state
        .oauth2_client
        .exchange_code(AuthorizationCode::new(code))
        .request_async(oauth2::reqwest::async_http_client)
        .await
        .map_err(|err| {
            tracing::error!("Failed to exchange code: {:?}", err);
            (StatusCode::BAD_GATEWAY, "Failed to exchange code").into_response()
        })?;

    let user_info = state
        .auth0_service
        .userinfo(token.access_token().secret().clone())
        .await
        .map_err(|err| {
            tracing::error!("Failed to retrieve user info: {:?}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to retrieve user info").into_response()
        })?;

    let jar = set_session_cookie(&jar, token.access_token().secret());

    let redirect_to = match state.member_service.get_member(user_info.user_id).await {
        Ok(_) => "/".to_string(),
        Err(_) => "/signup".to_string(),
    };

    Ok((jar, Json(CallbackResponse { redirect_to })))
}
