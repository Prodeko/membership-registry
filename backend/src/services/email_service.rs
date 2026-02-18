use reqwest::Client;

const DEFAULT_BASE_URL: &str = "https://api.sendgrid.com";
const DEFAULT_FROM_EMAIL: &str = "noreply@prodeko.org";

#[derive(Clone)]
pub struct EmailService {
    client: Client,
    api_key: Option<String>,
    from_email: String,
    base_url: String,
}

impl EmailService {
    pub fn new(
        api_key: Option<String>,
        api_url: Option<String>,
        from_email: Option<String>,
    ) -> Self {
        let api_key = api_key.filter(|k| !k.is_empty());
        Self {
            client: Client::new(),
            api_key,
            from_email: from_email
                .filter(|e| !e.is_empty())
                .unwrap_or_else(|| DEFAULT_FROM_EMAIL.to_string()),
            base_url: api_url
                .filter(|u| !u.is_empty())
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
        }
    }

    #[cfg(test)]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    pub async fn send_email(self, to: String, subject: String, html_body: String) {
        let api_key = match self.api_key {
            None => {
                tracing::info!(
                    "[MOCK EMAIL] To: {}, Subject: {}, Body: {}",
                    to,
                    subject,
                    html_body
                );
                return;
            }
            Some(ref key) => key.clone(),
        };

        let url = format!("{}/v3/mail/send", self.base_url);

        let payload = serde_json::json!({
            "personalizations": [{ "to": [{ "email": to }] }],
            "from": { "email": self.from_email },
            "subject": subject,
            "content": [{ "type": "text/html", "value": html_body }]
        });

        match self
            .client
            .post(&url)
            .bearer_auth(&api_key)
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                tracing::info!("Email sent to {}", to);
            }
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                tracing::error!("SendGrid error {}: {}", status, body);
            }
            Err(e) => {
                tracing::error!("Failed to send email to {}: {}", to, e);
            }
        }
    }
}
