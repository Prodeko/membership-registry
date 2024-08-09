use axum::{
    body::Body,
    extract::{Path, Request, State},
    http::{Response, StatusCode},
    middleware::Next, Extension,
};
use axum_extra::extract::CookieJar;
use uuid::Uuid;

use crate::{http::AppState, services::ory_service::AuthInfo};

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
                req.extensions_mut().insert(userinfo);
                next.run(req).await
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

pub async fn check_permission(
    State(state): State<AppState>,
    Extension(userinfo): Extension<Option<AuthInfo>>,
    req: Request,
    next: Next,
) -> Response<Body> {
    let userinfo = userinfo.unwrap();

    let has_access = state
        .ory_service
        .is_admin(
            userinfo.user_id.clone(),
        )
        .await
        .unwrap_or(false);
    if has_access {
        next.run(req).await
    } else {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .unwrap()
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
                .ory_service
                .is_admin(userinfo.user_id.clone())
                .await
                .unwrap_or(false);

            if !is_admin || userinfo.user_id != user_id {
                return Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Body::empty())
                    .unwrap();
            }

            next.run(req).await
        }
        None => Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .unwrap(),
    }
}

