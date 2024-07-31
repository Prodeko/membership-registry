use axum::{
    body::Body,
    extract::{Request, State},
    http::{Response, StatusCode},
    middleware::Next,
};
use axum_extra::extract::CookieJar;
use cookie::{Cookie, SameSite};

use crate::http::AppState;

pub async fn check_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response<Body> {
    let jar = CookieJar::from_headers(req.headers());

    if let Some(cookie) = jar.get("access_token") {
        let userinfo = state
            .ory_service
            .userinfo(cookie.value().to_string())
            .await;

        match userinfo {
            Ok(userinfo) => {
                let has_access = state
                    .ory_service
                    .check_permission(
                        "membership-registry-admin-scope".to_string(),
                        "access".to_string(),
                        userinfo.user_id,
                        userinfo.access_token.clone(),
                    )
                    .await
                    .unwrap_or(false);
                if has_access {
                    req.extensions_mut().insert(Some(userinfo.access_token));
                    next.run(req).await
                } else {
                    Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .body(Body::empty())
                        .unwrap()
                }
            }
            Err(_) => Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(Body::empty())
                .unwrap(),
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
