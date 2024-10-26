use std::str::FromStr;

use crate::http::errors::HttpResult;

use super::AppState;
use axum::{
    debug_handler, extract::{Json, State}, http::StatusCode, response::IntoResponse, routing::post, Router
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
) -> HttpResult<(StatusCode, Json<Value>)> {
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
            println!("Invalid signature, {}", err);
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid signature."})),
            ));
        }
    };

    match event.type_ {
        stripe::EventType::CheckoutSessionCompleted => {
            let checkout: CheckoutSession = match event.data.object {
                EventObject::CheckoutSession(session) => session,
                _ => {
                    println!("Unhandled event object");
                    return Ok((
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({"error": "Unhandled event object."})),
                    ));
                }
            };

            let application_id = match &checkout
                .client_reference_id
                .map(|s| Uuid::from_str(s.as_str()).ok())
                .flatten()
            {
                Some(id) => *id,
                None => {
                    return Ok((
                        StatusCode::BAD_REQUEST,
                        Json(
                            serde_json::json!({"error": "Failed to parse application ID. It must be a valid UUID."}),
                        ),
                    ));
                }
            };

            state
                .application_service
                .update_payment_id(
                    application_id,
                    checkout.payment_intent.unwrap().id().to_string(),
                )
                .await.map_err(|e| e.into_response());

            Ok((
                StatusCode::OK,
                Json(serde_json::json!({"success": true, "message": "Payment ID updated."})),
            ))
        }
        _ => Ok((
            StatusCode::OK,
            Json(serde_json::json!({"success": true, "message": "Unhandled event type."})),
        )),
    }
}
