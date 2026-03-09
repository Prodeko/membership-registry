use axum::{
    body::Body,
    extract::{Path, Request, State},
    http::{Response, StatusCode},
    middleware::Next,
    response::IntoResponse,
    Extension,
};
use axum_extra::extract::CookieJar;
use uuid::Uuid;

use crate::{
    application::services::authentication_service::{AuthServiceError, AuthenticatedUser},
    helpers::{set_refresh_token_cookie, set_session_cookie},
};

use super::AppState;

pub async fn check_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response<Body> {
    let jar = CookieJar::from_headers(req.headers());
    let Some(access_cookie) = jar.get("access_token") else {
        tracing::warn!("No access token in cookie");
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let access_token = access_cookie.value().to_string();
    let result = state
        .authentication_service
        .validate_token(access_token)
        .await;

    match result {
        Ok(userinfo) => {
            req.extensions_mut().insert(Some(userinfo));
            next.run(req).await
        }
        Err(AuthServiceError::TokenExpired) => {
            let Some(refresh_cookie) = jar.get("refresh_token") else {
                tracing::debug!("Token expired and no refresh token available");
                return StatusCode::UNAUTHORIZED.into_response();
            };

            let refreshed = state
                .authentication_service
                .refresh_access_token(refresh_cookie.value())
                .await;

            match refreshed {
                Ok(tokens) => {
                    let userinfo = state
                        .authentication_service
                        .validate_token(tokens.access_token.clone())
                        .await;

                    match userinfo {
                        Ok(userinfo) => {
                            req.extensions_mut().insert(Some(userinfo));
                            let mut response = next.run(req).await;

                            let new_jar = CookieJar::new();
                            let new_jar = set_session_cookie(&new_jar, &tokens.access_token);
                            let new_jar = match tokens.refresh_token {
                                Some(ref rt) => set_refresh_token_cookie(&new_jar, rt),
                                None => new_jar,
                            };
                            for cookie in new_jar.iter() {
                                if let Ok(val) = cookie.to_string().parse() {
                                    response
                                        .headers_mut()
                                        .append(axum::http::header::SET_COOKIE, val);
                                }
                            }

                            response
                        }
                        Err(e) => {
                            tracing::error!("Error validating refreshed token {e:?}");
                            StatusCode::UNAUTHORIZED.into_response()
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!("Token refresh failed: {e:?}");
                    StatusCode::UNAUTHORIZED.into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("Error validating token {e:?}");
            StatusCode::UNAUTHORIZED.into_response()
        }
    }
}

pub async fn check_permission(
    State(state): State<AppState>,
    Extension(userinfo): Extension<Option<AuthenticatedUser>>,
    req: Request,
    next: Next,
) -> Response<Body> {
    let Some(userinfo) = userinfo else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    match state
        .authentication_service
        .is_admin(userinfo.user_id)
        .await
    {
        Ok(true) => next.run(req).await,
        Ok(false) => StatusCode::FORBIDDEN.into_response(),
        Err(e) => {
            tracing::error!("Error checking admin status: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn check_member_access(
    Extension(userinfo): Extension<Option<AuthenticatedUser>>,
    Path((user_id,)): Path<(Uuid,)>,
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response<Body> {
    match userinfo {
        Some(userinfo) => {
            let is_admin = match state
                .authentication_service
                .is_admin(userinfo.user_id)
                .await
            {
                Ok(val) => val,
                Err(e) => {
                    tracing::error!("Error checking admin status: {e:?}");
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            };

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
