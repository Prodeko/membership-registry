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
    /// The UUID from client_reference_id — could be an application or a renewal.
    pub reference_id: Uuid,
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
