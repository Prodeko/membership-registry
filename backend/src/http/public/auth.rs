use axum::{extract::State, response::Redirect, routing::get, Json, Router};
use axum_extra::extract::CookieJar;
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
use reqwest::Client;
use serde_json::Value;

use crate::helpers::set_session_cookie;

use super::AppState;

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
) -> (CookieJar, Json<String>) {
    println!("Running callback");
    if let (Some(code), Some(_state_param)) = (query.get("code"), query.get("state")) {
        println!("Code: {}", code);
        let token_result = state
            .oauth2_client
            .exchange_code(AuthorizationCode::new(code.clone()))
            .request_async(oauth2::reqwest::async_http_client)
            .await;
        println!("Token result: {:?}", token_result);

        match token_result {
            Ok(token) => {
                let userinfo = state
                    .auth0_service
                    .userinfo(token.access_token().secret().clone())
                    .await;
                match userinfo {
                    Ok(response) => {
                        let jar = set_session_cookie(&jar, token.access_token().secret());
                        return (jar, Json(token.access_token().secret().to_string()));
                    }
                    Err(err) => {
                        eprintln!("Failed to retrieve user info: {:?}", err);
                        return (jar, Json("Error: Failed to retrieve user info".to_string()));
                    }
                }
            }
            Err(err) => {
                eprintln!("Failed to exchange code: {:?}", err);
                (jar, Json(format!("Error: {:?}", err)))
            }
        }
    } else {
        println!("Missing code or state");
        (jar, Json("Missing code or state".to_string()))
    }
}
