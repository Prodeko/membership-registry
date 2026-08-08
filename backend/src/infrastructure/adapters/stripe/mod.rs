use std::str::FromStr;

use stripe_webhook::{EventObject, Webhook, WebhookError};
use uuid::Uuid;

use crate::application::ports::payment_webhook_port::{
    PaymentEvent, PaymentWebhookError, PaymentWebhookPort,
};

pub struct StripeWebhookAdapter {
    endpoint_secret: String,
}

impl StripeWebhookAdapter {
    pub fn new(endpoint_secret: String) -> Self {
        Self { endpoint_secret }
    }
}

#[async_trait::async_trait]
impl PaymentWebhookPort for StripeWebhookAdapter {
    async fn verify_and_parse(
        &self,
        payload: &str,
        signature: &str,
    ) -> Result<Option<PaymentEvent>, PaymentWebhookError> {
        let event = Webhook::construct_event(payload, signature, &self.endpoint_secret).map_err(
            |e| match e {
                WebhookError::BadParse(msg) => {
                    tracing::warn!(error = %msg, "failed to parse Stripe webhook payload");
                    PaymentWebhookError::InvalidPayload(msg)
                }
                _ => PaymentWebhookError::InvalidSignature,
            },
        )?;

        match event.data.object {
            EventObject::CheckoutSessionCompleted(session) => {
                let reference_id = session
                    .client_reference_id
                    .as_deref()
                    .and_then(|s| Uuid::from_str(s).ok())
                    .ok_or(PaymentWebhookError::MissingApplicationId)?;

                let payment_intent = session
                    .payment_intent
                    .ok_or(PaymentWebhookError::MissingPaymentIntent)?;

                Ok(Some(PaymentEvent {
                    reference_id,
                    payment_intent_id: payment_intent.into_id().to_string(),
                }))
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "whsec_test_secret";
    const REFERENCE_ID: &str = "1f8b4c2e-3d5a-4b6c-8d7e-9f0a1b2c3d4e";

    fn checkout_session_completed_payload() -> String {
        serde_json::json!({
            "id": "evt_1U18rHBXFu7VMnhUpCL1ZhcL",
            "object": "event",
            "api_version": "2026-07-29.dahlia",
            "created": 1754415844,
            "livemode": true,
            "pending_webhooks": 1,
            "data": {
                "object": {
                    "id": "cs_live_a1b2c3",
                    "object": "checkout.session",
                    "automatic_tax": { "enabled": false },
                    "client_reference_id": REFERENCE_ID,
                    "created": 1754415841,
                    "custom_fields": [],
                    "custom_text": {},
                    "expires_at": 1754502241,
                    "livemode": true,
                    "mode": "payment",
                    "payment_intent": "pi_3U18qlBXFu7VMnhU0aYBqApQ",
                    "payment_method_types": ["card"],
                    "payment_status": "paid",
                    "shipping_options": [],
                    "status": "complete",
                    "ui_mode": "hosted_page"
                }
            },
            "type": "checkout.session.completed"
        })
        .to_string()
    }

    fn adapter() -> StripeWebhookAdapter {
        StripeWebhookAdapter::new(SECRET.to_string())
    }

    #[tokio::test]
    async fn parses_checkout_session_completed() {
        let payload = checkout_session_completed_payload();
        let signature = Webhook::generate_test_header(&payload, SECRET, None);

        let event = adapter()
            .verify_and_parse(&payload, &signature)
            .await
            .expect("valid signed payload should parse")
            .expect("checkout.session.completed should produce an event");

        assert_eq!(event.reference_id, Uuid::from_str(REFERENCE_ID).unwrap());
        assert_eq!(event.payment_intent_id, "pi_3U18qlBXFu7VMnhU0aYBqApQ");
    }

    #[tokio::test]
    async fn rejects_wrong_secret_as_invalid_signature() {
        let payload = checkout_session_completed_payload();
        let signature = Webhook::generate_test_header(&payload, "whsec_other_secret", None);

        let err = adapter()
            .verify_and_parse(&payload, &signature)
            .await
            .err()
            .unwrap();
        assert!(matches!(err, PaymentWebhookError::InvalidSignature));
    }

    #[tokio::test]
    async fn maps_parse_failure_to_invalid_payload_not_signature() {
        let payload = r#"{"not": "an event"}"#;
        let signature = Webhook::generate_test_header(payload, SECRET, None);

        let err = adapter()
            .verify_and_parse(payload, &signature)
            .await
            .err()
            .unwrap();
        assert!(matches!(err, PaymentWebhookError::InvalidPayload(_)));
    }

    #[tokio::test]
    async fn ignores_unhandled_event_types() {
        let payload = serde_json::json!({
            "id": "evt_unhandled",
            "object": "event",
            "api_version": "2026-07-29.dahlia",
            "created": 1754415844,
            "livemode": true,
            "pending_webhooks": 1,
            "data": { "object": { "object": "radar.early_fraud_warning", "id": "issfr_x" } },
            "type": "radar.early_fraud_warning.created"
        })
        .to_string();
        let signature = Webhook::generate_test_header(&payload, SECRET, None);

        let event = adapter()
            .verify_and_parse(&payload, &signature)
            .await
            .unwrap();
        assert!(event.is_none());
    }
}
