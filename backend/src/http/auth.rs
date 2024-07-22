use axum::{
    extract::State, response::Redirect, routing::get, Json, Router
};
use axum_extra::extract::CookieJar;
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
use reqwest::Client;
use serde_json::Value;

use crate::auth::set_session_cookie;

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
    jar: CookieJar,
) -> (CookieJar, Json<String>) {
    if let (Some(code), Some(_state_param)) = (query.get("code"), query.get("state")) {
        let token_result = state
            .oauth2_client
            .exchange_code(AuthorizationCode::new(code.clone()))
            .request_async(oauth2::reqwest::async_http_client)
            .await;
        println!("Token result: {:?}", token_result);

        match token_result {
            Ok(token) => {
                println!("Token: {:?}", token.access_token().secret());
                // Retrieve user info from OIDC provider
                let client = Client::new();
                match client
                    .get("http://127.0.0.1:4444/userinfo")
                    .bearer_auth(token.access_token().secret())
                    .send()
                    .await
                {
                    Ok(response) => match response.json::<Value>().await {
                        Ok(user_info) => {
                            if let Some(_user_id) = user_info["sub"].as_str() {
                                let jar = set_session_cookie(&jar, token.access_token().secret());
                                return (jar, Json(token.access_token().secret().to_string()));
                            } else {
                                return (
                                    jar,
                                    Json("Error: Missing user ID in response".to_string()),
                                );
                            }
                        }
                        Err(err) => {
                            eprintln!("Failed to parse user info: {:?}", err);
                            return (jar, Json("Error: Failed to parse user info".to_string()));
                        }
                    },
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
        (jar, Json("Missing code or state".to_string()))
    }
}
