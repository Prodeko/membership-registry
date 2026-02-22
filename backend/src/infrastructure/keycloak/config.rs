#[derive(Clone, Debug)]
pub struct KeycloakConfig {
    pub base_url: String,
    pub realm: String,

    // For refresh_token flow (public client or confidential client)
    pub client_id: String,
    pub client_secret: Option<String>,

    // For admin API (client_credentials)
    pub admin_client_id: String,
    pub admin_client_secret: String,

    // Realm role name that maps to "admin" in this application
    pub admin_role_name: String,
}
