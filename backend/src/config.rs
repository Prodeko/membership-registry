use envconfig::Envconfig;
use validator::Validate;

#[derive(Envconfig, Validate, Clone)]
pub struct Config {
    #[envconfig(from = "PORT")]
    pub port: u16,

    #[envconfig(from = "DATABASE_URL")]
    #[validate(length(min = 1, max = 1024))]
    pub database_url: String,

    #[envconfig(from = "TEST_DATABASE_URL")]
    #[validate(length(min = 1, max = 1024))]
    pub test_database_url: String,

    #[envconfig(from = "FRONTEND_URL")]
    #[validate(length(min = 1, max = 1024))]
    pub frontend_url: String,

    #[envconfig(from = "AUTH0_DOMAIN")]
    #[validate(length(min = 1, max = 1024))]
    pub auth0_domain: String,

    #[envconfig(from = "AUTH0_CLIENT_ID")]
    #[validate(length(min = 1, max = 1024))]
    pub auth0_client_id: String,

    #[envconfig(from = "AUTH0_CLIENT_SECRET")]
    #[validate(length(min = 1, max = 1024))]
    pub auth0_client_secret: String,

    #[envconfig(from = "OAUTH_REDIRECT_URL")]
    #[validate(length(min = 1, max = 1024))]
    pub oauth_redirect_url: String,

    #[envconfig(from = "AUTH0_AUDIENCE")]
    pub auth0_audience: Option<String>,

    #[envconfig(from = "AUTH0_MANAGEMENT_API_TOKEN")]
    #[validate(length(min = 1, max = 1024))]
    pub auth0_management_api_token: String,

    #[envconfig(from = "STRIPE_ENDPOINT_SECRET")]
    #[validate(length(min = 1, max = 1024))]
    pub stripe_endpoint_secret: String,
}
