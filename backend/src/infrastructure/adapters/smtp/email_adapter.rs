use lettre::{
    message::header::ContentType, transport::smtp::client::Tls, AsyncSmtpTransport, AsyncTransport,
    Message, Tokio1Executor,
};

use crate::application::ports::email_port::{EmailError, EmailPort};

#[derive(Clone, Debug)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub from_email: String,
}

pub struct SmtpEmailAdapter {
    from_email: String,
    transport: AsyncSmtpTransport<Tokio1Executor>,
}

/// Hosts this adapter is allowed to connect to.
///
/// This adapter sends plaintext SMTP with no TLS and no auth — it exists only
/// for local dev and e2e against Mailpit. To prevent accidental production use,
/// any host not in this list is rejected at construction time. Prod must use
/// `SendGridEmailAdapter` instead.
const ALLOWED_HOSTS: &[&str] = &["localhost", "127.0.0.1", "::1"];

#[derive(Debug)]
pub struct SmtpAdapterError(pub String);

impl std::fmt::Display for SmtpAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for SmtpAdapterError {}

impl SmtpEmailAdapter {
    pub fn new(config: SmtpConfig) -> Result<Self, SmtpAdapterError> {
        if !ALLOWED_HOSTS.contains(&config.host.as_str()) {
            return Err(SmtpAdapterError(format!(
                "SmtpEmailAdapter refuses host '{}': only local hosts {:?} are allowed. \
                 This adapter uses no TLS/auth and is for dev/e2e only — use SendGrid in production.",
                config.host, ALLOWED_HOSTS
            )));
        }

        let transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
            .port(config.port)
            .tls(Tls::None)
            .build();

        Ok(Self {
            from_email: config.from_email,
            transport,
        })
    }
}

#[async_trait::async_trait]
impl EmailPort for SmtpEmailAdapter {
    async fn send_email(&self, to: &str, subject: &str, html_body: &str) -> Result<(), EmailError> {
        let from = self
            .from_email
            .parse()
            .map_err(|e: lettre::address::AddressError| EmailError::SendFailed(e.to_string()))?;
        let to_addr = to
            .parse()
            .map_err(|e: lettre::address::AddressError| EmailError::SendFailed(e.to_string()))?;

        let email = Message::builder()
            .from(from)
            .to(to_addr)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(html_body.to_string())
            .map_err(|e| EmailError::SendFailed(e.to_string()))?;

        self.transport
            .send(email)
            .await
            .map_err(|e| EmailError::SendFailed(e.to_string()))?;

        Ok(())
    }
}
