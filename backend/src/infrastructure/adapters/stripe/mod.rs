use std::str::FromStr;

use stripe::{CheckoutSession, EventObject};
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
        let event = stripe::Webhook::construct_event(payload, signature, &self.endpoint_secret)
            .map_err(|_| PaymentWebhookError::InvalidSignature)?;

        match event.type_ {
            stripe::EventType::CheckoutSessionCompleted => {
                let checkout: CheckoutSession = match event.data.object {
                    EventObject::CheckoutSession(session) => session,
                    _ => return Err(PaymentWebhookError::UnhandledEvent),
                };

                let application_id = checkout
                    .client_reference_id
                    .and_then(|s| Uuid::from_str(s.as_str()).ok())
                    .ok_or(PaymentWebhookError::MissingApplicationId)?;

                let payment_intent = checkout
                    .payment_intent
                    .ok_or(PaymentWebhookError::MissingPaymentIntent)?;

                Ok(Some(PaymentEvent {
                    application_id,
                    payment_intent_id: payment_intent.id().to_string(),
                }))
            }
            _ => Ok(None),
        }
    }
}
