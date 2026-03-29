use crate::application::ports::email_port::{EmailError, EmailPort};

use super::config::SendGridConfig;

pub struct SendGridEmailAdapter {
    config: SendGridConfig,
    http: reqwest::Client,
}

impl SendGridEmailAdapter {
    pub fn new(config: SendGridConfig) -> Self {
        Self {
            config,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait::async_trait]
impl EmailPort for SendGridEmailAdapter {
    async fn send_email(&self, to: &str, subject: &str, html_body: &str) -> Result<(), EmailError> {
        let url = format!("{}/v3/mail/send", self.config.base_url);

        let payload = serde_json::json!({
            "personalizations": [{ "to": [{ "email": to }] }],
            "from": { "email": self.config.from_email },
            "subject": subject,
            "content": [{ "type": "text/html", "value": html_body }]
        });

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| EmailError::SendFailed(e.to_string()))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(EmailError::SendFailed(format!("SendGrid {status}: {body}")))
        }
    }
}
