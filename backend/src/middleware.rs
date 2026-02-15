use axum::{
    body::Body,
    extract::{Path, Request, State},
    http::{Response, StatusCode},
    middleware::Next,
    response::{IntoResponse, Redirect},
    Extension,
};
use axum_extra::extract::CookieJar;
use uuid::Uuid;

use crate::{http::AppState, services::auth0_service::AuthInfo};

pub async fn check_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response<Body> {
    let jar = CookieJar::from_headers(req.headers());
    if let Some(cookie) = jar.get("access_token") {
        let userinfo = state.auth0_service.userinfo(cookie.value().to_string()).await;

        match userinfo {
            Ok(userinfo) => {
                req.extensions_mut().insert(Some(userinfo));
                next.run(req).await
            }
            Err(e) => {
                tracing::error!("Error fetching userinfo {:?}", e);
                StatusCode::UNAUTHORIZED.into_response()
            },
        }
    } else {
        tracing::warn!("No access token in cookie");
        StatusCode::UNAUTHORIZED.into_response()
    }
}

pub async fn check_permission(
    State(state): State<AppState>,
    Extension(userinfo): Extension<Option<AuthInfo>>,
    req: Request,
    next: Next,
) -> Response<Body> {
    let userinfo = userinfo.unwrap();

    let has_access = state
        .auth0_service
        .is_admin(userinfo.user_id.clone())
        .await
        .unwrap_or(false);
    if has_access {
        next.run(req).await
    } else {
        StatusCode::FORBIDDEN.into_response()
    }
}

/**
 * Middleware to check if the user id in the path matches the user id in the token
 * or if the user is an admin
 */
pub async fn check_member_access(
    Extension(userinfo): Extension<Option<AuthInfo>>,
    Path((user_id,)): Path<(Uuid,)>,
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response<Body> {
    match userinfo {
        Some(userinfo) => {
            let is_admin = state
                .auth0_service
                .is_admin(userinfo.user_id.clone())
                .await
                .unwrap_or(false);

            tracing::debug!("Is admin: {}", is_admin);

            if !(is_admin || userinfo.user_id == user_id) {
                return StatusCode::FORBIDDEN.into_response();
            }

            next.run(req).await
        }
        None => {
            tracing::warn!("No userinfo!");
            StatusCode::UNAUTHORIZED.into_response()
        }
    }
}
