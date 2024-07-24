use axum::{
    body::Body,
    extract::{Request, State},
    http::{Response, StatusCode},
    middleware::Next,
};
use axum_extra::extract::CookieJar;
use cookie::{Cookie, SameSite};
use reqwest::{Client, Proxy};
use serde_json::Value;

use crate::http::AppState;

pub async fn check_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response<Body> {
    let jar = CookieJar::from_headers(req.headers());
    if let Some(cookie) = jar.get("access_token") {
        // Optionally validate token with userinfo endpoint
        let proxy = Proxy::http("http://localhost:8080").unwrap();
        let client = Client::builder().proxy(proxy).build().unwrap();
        let res = client
            .get("http://127.0.0.1:4444/userinfo")
            .bearer_auth(cookie.value())
            .send()
            .await
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Unauthorized"));

        if res.is_ok() && res.as_ref().unwrap().status().is_success() {
            // Add  access token to request extensions so it can be accessed by other services down the stack
            let access_token = cookie.value().to_string();
            let res_body = res.unwrap().json::<Value>().await.unwrap();
            let user_id = res_body["sub"].as_str().unwrap_or("unknown").to_string();
            if state
                .user_service
                .check_permission(
                    "membership-registry-admin-scope".to_string(),
                    "access".to_string(),
                    user_id,
                    access_token.clone(),
                )
                .await
                .unwrap_or(false)
            {
                req.extensions_mut().insert(Some(access_token));
                next.run(req).await
            } else {
                Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(Body::empty())
                    .unwrap()
            }
        } else {
            Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(Body::empty())
                .unwrap()
        }
    } else {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .unwrap()
    }
}

pub fn set_session_cookie(jar: &CookieJar, token: &str) -> CookieJar {
    println!("Setting session cookie");
    let base_cookie = Cookie::new("access_token", token.to_string());
    let cookie = Cookie::build(base_cookie)
        .path("/")
        .secure(true) // Set to true if using HTTPS
        .http_only(true)
        .same_site(SameSite::None); // Allows the cookie to be sent with requests from other sites

    jar.clone().add(cookie)
}
