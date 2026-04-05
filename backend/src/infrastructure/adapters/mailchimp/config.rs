#[derive(Clone, Debug)]
pub struct MailchimpConfig {
    pub api_key: String,
    /// Datacenter prefix extracted from the API key (e.g. "us21").
    pub dc: String,
    pub list_id: String,
}

impl MailchimpConfig {
    /// Build a config from an API key and list id. The Mailchimp datacenter
    /// is embedded as the suffix after the final `-` in the API key (e.g.
    /// `abc123...-us21` → `us21`). Returns `None` if the key has no suffix.
    pub fn new(api_key: String, list_id: String) -> Option<Self> {
        let dc = api_key.rsplit_once('-').map(|(_, dc)| dc.to_string())?;
        if dc.is_empty() {
            return None;
        }
        Some(Self {
            api_key,
            dc,
            list_id,
        })
    }

    pub fn base_url(&self) -> String {
        format!("https://{}.api.mailchimp.com/3.0", self.dc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_datacenter_from_key_suffix() {
        let cfg = MailchimpConfig::new("abc123def456-us21".to_string(), "list1".to_string())
            .expect("valid key");
        assert_eq!(cfg.dc, "us21");
        assert_eq!(cfg.base_url(), "https://us21.api.mailchimp.com/3.0");
    }

    #[test]
    fn rejects_key_without_suffix() {
        assert!(MailchimpConfig::new("nosuffix".to_string(), "list1".to_string()).is_none());
    }

    #[test]
    fn rejects_empty_suffix() {
        assert!(MailchimpConfig::new("trailing-".to_string(), "list1".to_string()).is_none());
    }
}
