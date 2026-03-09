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
