use serde::Deserialize;

pub const DIRECTORY_API_BASE: &str = "https://admin.googleapis.com/admin/directory/v1";
pub const GROUP_MEMBER_SCOPE: &str = "https://www.googleapis.com/auth/admin.directory.group.member";

/// The fields of a Google service-account JSON key this adapter needs.
#[derive(Clone, Deserialize)]
pub struct ServiceAccountKey {
    pub client_email: String,
    pub private_key: String,
    pub token_uri: String,
}

impl std::fmt::Debug for ServiceAccountKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceAccountKey")
            .field("client_email", &self.client_email)
            .field("token_uri", &self.token_uri)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub struct GoogleGroupsConfig {
    pub key: ServiceAccountKey,
    /// Workspace admin the service account impersonates through
    /// domain-wide delegation. Directory API writes need an admin subject.
    pub delegated_admin: String,
    pub api_base: String,
}

impl GoogleGroupsConfig {
    /// Build a config from the service-account key JSON (the file Google
    /// hands out, passed as the env var's value) and the admin to
    /// impersonate. Returns `None` if the JSON is not a service-account key.
    pub fn new(key_json: &str, delegated_admin: String) -> Option<Self> {
        let key: ServiceAccountKey = serde_json::from_str(key_json).ok()?;
        if key.client_email.is_empty() || key.private_key.is_empty() {
            return None;
        }
        Some(Self {
            key,
            delegated_admin,
            api_base: DIRECTORY_API_BASE.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_service_account_key() {
        let json = r#"{
            "type": "service_account",
            "client_email": "sync@project.iam.gserviceaccount.com",
            "private_key": "-----BEGIN PRIVATE KEY-----\nabc\n-----END PRIVATE KEY-----\n",
            "token_uri": "https://oauth2.googleapis.com/token"
        }"#;
        let cfg =
            GoogleGroupsConfig::new(json, "admin@prodeko.org".to_string()).expect("valid key");
        assert_eq!(cfg.key.client_email, "sync@project.iam.gserviceaccount.com");
        assert_eq!(cfg.api_base, DIRECTORY_API_BASE);
    }

    #[test]
    fn rejects_non_key_json() {
        assert!(GoogleGroupsConfig::new("{}", "admin@prodeko.org".to_string()).is_none());
        assert!(GoogleGroupsConfig::new("not json", "admin@prodeko.org".to_string()).is_none());
    }

    #[test]
    fn debug_hides_private_key() {
        let key = ServiceAccountKey {
            client_email: "a@b".to_string(),
            private_key: "SECRET".to_string(),
            token_uri: "t".to_string(),
        };
        assert!(!format!("{key:?}").contains("SECRET"));
    }
}
