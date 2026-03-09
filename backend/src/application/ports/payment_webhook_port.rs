use uuid::Uuid;

#[derive(Debug)]
pub enum PaymentWebhookError {
    InvalidSignature,
    InvalidPayload(String),
    MissingApplicationId,
    MissingPaymentIntent,
    UnhandledEvent,
}

pub struct PaymentEvent {
    pub application_id: Uuid,
    pub payment_intent_id: String,
}

#[async_trait::async_trait]
pub trait PaymentWebhookPort: Send + Sync {
    async fn verify_and_parse(
        &self,
        payload: &str,
        signature: &str,
    ) -> Result<Option<PaymentEvent>, PaymentWebhookError>;
}
