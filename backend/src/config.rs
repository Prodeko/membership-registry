use envconfig::Envconfig;
use validator::Validate;

#[derive(Envconfig, Validate)]
pub struct Config {
    #[envconfig(from = "PORT")]
    pub port: u16,

    #[envconfig(from = "DATABASE_URL")]
    #[validate(length(min = 1, max = 1024))]
    pub database_url: String,

    #[envconfig(from = "ORY_BASE_URL")]
    #[validate(length(min = 1, max = 1024))]
    pub ory_base_url: String,

    #[envconfig(from = "OAUTH_CLIENT_ID")]
    #[validate(length(min = 1, max = 1024))]
    pub oauth_client_id: String,

    #[envconfig(from = "OAUTH_CLIENT_SECRET")]
    #[validate(length(min = 1, max = 1024))]
    pub oauth_client_secret: String,

    #[envconfig(from = "OAUTH_ISSUER_URL")]
    #[validate(length(min = 1, max = 1024))]
    pub oauth_issuer_url: String,

    #[envconfig(from = "OAUTH_REDIRECT_URL")]
    #[validate(length(min = 1, max = 1024))]
    pub oauth_redirect_url: String,

    #[envconfig(from = "STRIPE_ENDPOINT_SECRET")]
    #[validate(length(min = 1, max = 1024))]
    pub stripe_endpoint_secret: String,
}
