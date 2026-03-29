#[derive(Debug)]
pub enum EmailError {
    SendFailed(String),
}

#[async_trait::async_trait]
pub trait EmailPort: Send + Sync {
    async fn send_email(&self, to: &str, subject: &str, html_body: &str) -> Result<(), EmailError>;
}
