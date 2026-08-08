use crate::application::ports::payment_webhook_port::PaymentWebhookError;
use crate::infrastructure::http::errors::{ApiError, ApiResult};

use super::AppState;
use axum::{
    debug_handler,
    extract::{Json, State},
    routing::post,
    Router,
};
use serde_json::{self, Value};

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
        .ok_or(ApiError::Unauthorized)?;

    let event = state
        .payment_webhook
        .verify_and_parse(&body, sig_header)
        .await
        .map_err(|e| match e {
            PaymentWebhookError::InvalidSignature => ApiError::Unauthorized,
            PaymentWebhookError::MissingApplicationId
            | PaymentWebhookError::MissingPaymentIntent
            | PaymentWebhookError::InvalidPayload(_) => ApiError::BadRequest,
        })?;

    let Some(event) = event else {
        return Ok(Json(
            serde_json::json!({"success": true, "message": "Unhandled event type."}),
        ));
    };

    // Try renewal first, then application
    let handled_as_renewal = state
        .renewal_service
        .process_renewal_payment(event.reference_id, event.payment_intent_id.clone())
        .await
        .map_err(ApiError::ServiceError)?;

    if !handled_as_renewal {
        state
            .application_service
            .update_payment_id(event.reference_id, event.payment_intent_id)
            .await
            .map_err(ApiError::ServiceError)?;
    }

    Ok(Json(
        serde_json::json!({"success": true, "message": "Payment processed."}),
    ))
}
