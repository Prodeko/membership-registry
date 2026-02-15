use std::str::FromStr;

use crate::http::errors::{ApiError, ApiResult};

use super::AppState;
use axum::{
    debug_handler,
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use serde_json::{self, Value};
use stripe::{CheckoutSession, EventObject};
use uuid::Uuid;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/webhook", post(stripe_webhook))
        .with_state(state)
}

#[debug_handler]
async fn stripe_webhook(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body: String,
) -> ApiResult<Json<Value>> {
    let sig_header = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let event = match stripe::Webhook::construct_event(
        &body,
        sig_header,
        &state.config.stripe_endpoint_secret,
    ) {
        Ok(event) => event,
        Err(err) => {
            tracing::warn!("Invalid signature, {}", err);
            return Err(ApiError::Unauthorized);
        }
    };

    match event.type_ {
        stripe::EventType::CheckoutSessionCompleted => {
            let checkout: CheckoutSession = match event.data.object {
                EventObject::CheckoutSession(session) => session,
                _ => {
                    tracing::warn!("Unhandled event object");
                    return Ok(Json(
                        serde_json::json!({"success": true, "message": "Unhandled event object."}),
                    ));
                }
            };

            let application_id = match &checkout
                .client_reference_id
                .and_then(|s| Uuid::from_str(s.as_str()).ok())
            {
                Some(id) => *id,
                None => {
                    return Err(ApiError::BadRequest);
                }
            };

            state
                .application_service
                .update_payment_id(
                    application_id,
                    checkout.payment_intent.unwrap().id().to_string(),
                )
                .await
                .map_err(|e| e.into_response());

            Ok(Json(
                serde_json::json!({"success": true, "message": "Payment ID updated."}),
            ))
        }
        _ => Ok(Json(
            serde_json::json!({"success": true, "message": "Unhandled event type."}),
        )),
    }
}
