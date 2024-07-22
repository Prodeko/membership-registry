use axum::{
    body::Body,
    extract::{Request, State},
    http::Response,
    middleware::Next
};
use axum_extra::extract::CookieJar;
use cookie::{Cookie, SameSite};
use reqwest::{Client, StatusCode};

use crate::http::AppState;

pub async fn check_auth(State(_state): State<AppState>, req: Request, next: Next) -> Response<Body> {
  let jar = CookieJar::from_headers(req.headers());
    if let Some(cookie) = jar.get("access_token") {
        // Optionally validate token with userinfo endpoint
        let client = Client::new();
        let res = client
            .get("http://127.0.0.1:4444/userinfo")
            .bearer_auth(cookie.value())
            .send()
            .await
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Unauthorized"));

        if res.is_ok() {
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
}

pub fn set_session_cookie(jar: &CookieJar, token: &str) -> CookieJar {
    println!("Setting session cookie");
    let base_cookie = Cookie::new("access_token", token.to_string());
    let cookie = Cookie::build(base_cookie)
        .domain("127.0.0.1") // Replace with your actual domain
        .path("/")
        .secure(false) // Set to true if using HTTPS
        .http_only(true)
        .same_site(SameSite::Lax); // Allows the cookie to be sent with requests from other sites

    jar.clone().add(cookie)
}
