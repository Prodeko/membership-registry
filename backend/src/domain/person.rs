#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonId(pub uuid::Uuid);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email(String);

impl Email {
    #[allow(clippy::expect_used)]
    pub fn new(email: String) -> Result<Self, &'static str> {
        let re = regex::Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").expect("valid regex literal");
        if !re.is_match(&email) {
            return Err("invalid email format");
        }
        Ok(Self(email))
    }

    /// Construct from a trusted source (e.g. database) without validation.
    pub fn new_unchecked(email: String) -> Self {
        Self(email)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    pub id: PersonId,
    pub email: Email,
    pub first_name: String,
    pub last_name: String,
    pub full_name: Option<String>,
    pub home_municipality: String,
    pub has_accepted_policies: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPerson {
    pub id: PersonId,
    pub email: Email,
    pub first_name: String,
    pub last_name: String,
    pub home_municipality: String,
    pub has_accepted_policies: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_valid_simple() {
        assert!(Email::new("user@example.com".to_string()).is_ok());
    }

    #[test]
    fn email_valid_with_plus_and_subdomain() {
        assert!(Email::new("user+tag@sub.domain.co.uk".to_string()).is_ok());
    }

    #[test]
    fn email_invalid_empty() {
        assert!(Email::new(String::new()).is_err());
    }

    #[test]
    fn email_invalid_no_at() {
        assert!(Email::new("no-at-sign".to_string()).is_err());
    }

    #[test]
    fn email_invalid_no_local() {
        assert!(Email::new("@no-local.com".to_string()).is_err());
    }

    #[test]
    fn email_invalid_trailing_at() {
        assert!(Email::new("trailing@".to_string()).is_err());
    }

    #[test]
    fn email_invalid_spaces() {
        assert!(Email::new("spaces in@email.com".to_string()).is_err());
    }

    #[test]
    fn email_unchecked_always_succeeds() {
        let email = Email::new_unchecked("anything".to_string());
        assert_eq!(email.as_str(), "anything");
    }
}
