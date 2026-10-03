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
    pub test_database_url: Option<String>,

    #[envconfig(from = "FRONTEND_URL")]
    #[validate(length(min = 1, max = 1024))]
    pub frontend_url: String,

    /// 6-field cron expression (seconds first) for the daily admin digest of
    /// stale applications, evaluated in Europe/Helsinki.
    #[envconfig(from = "APPLICATION_DIGEST_CRON", default = "0 0 7 * * *")]
    pub application_digest_cron: String,

    /// Days an application must have been waiting before the daily digest
    /// lists it. New applications are alerted individually as they arrive;
    /// the digest only re-surfaces the ones nobody has acted on.
    #[envconfig(from = "APPLICATION_DIGEST_STALE_DAYS", default = "3")]
    pub application_digest_stale_days: u32,

    #[envconfig(from = "KEYCLOAK_URL")]
    #[validate(length(min = 1, max = 1024))]
    pub keycloak_url: String,

    #[envconfig(from = "KEYCLOAK_REALM")]
    #[validate(length(min = 1, max = 1024))]
    pub keycloak_realm: String,

    #[envconfig(from = "KEYCLOAK_CLIENT_ID")]
    #[validate(length(min = 1, max = 1024))]
    pub keycloak_client_id: String,

    #[envconfig(from = "KEYCLOAK_CLIENT_SECRET")]
    #[validate(length(min = 1, max = 1024))]
    pub keycloak_client_secret: String,

    #[envconfig(from = "OAUTH_REDIRECT_URL")]
    #[validate(length(min = 1, max = 1024))]
    pub oauth_redirect_url: String,

    #[envconfig(from = "KEYCLOAK_ADMIN_CLIENT_ID")]
    #[validate(length(min = 1, max = 1024))]
    pub keycloak_admin_client_id: String,

    #[envconfig(from = "KEYCLOAK_ADMIN_CLIENT_SECRET")]
    #[validate(length(min = 1, max = 2048))]
    pub keycloak_admin_client_secret: String,

    #[envconfig(from = "STRIPE_ENDPOINT_SECRET")]
    #[validate(length(min = 1, max = 1024))]
    pub stripe_endpoint_secret: String,

    #[envconfig(from = "SENDGRID_API_KEY")]
    pub sendgrid_api_key: Option<String>,

    #[envconfig(from = "SENDGRID_API_URL")]
    pub sendgrid_api_url: Option<String>,

    #[envconfig(from = "SENDGRID_FROM_EMAIL")]
    pub sendgrid_from_email: Option<String>,

    #[envconfig(from = "SMTP_HOST")]
    pub smtp_host: Option<String>,

    #[envconfig(from = "SMTP_PORT")]
    pub smtp_port: Option<u16>,

    #[envconfig(from = "SMTP_FROM_EMAIL")]
    pub smtp_from_email: Option<String>,

    #[envconfig(from = "MAILCHIMP_API_KEY")]
    pub mailchimp_api_key: Option<String>,

    #[envconfig(from = "MAILCHIMP_LIST_ID")]
    pub mailchimp_list_id: Option<String>,

    /// Service-account key JSON (the file's contents, not a path).
    #[envconfig(from = "GOOGLE_SERVICE_ACCOUNT_KEY")]
    pub google_service_account_key: Option<String>,

    #[envconfig(from = "GOOGLE_DELEGATED_ADMIN")]
    pub google_delegated_admin: Option<String>,

    /// JSON array of group rules — see `GroupRule::parse_all`.
    #[envconfig(from = "GOOGLE_GROUP_RULES")]
    pub google_group_rules: Option<String>,
}
