#[derive(Clone, Debug)]
pub struct SendGridConfig {
    pub api_key: String,
    pub base_url: String,
    pub from_email: String,
}
