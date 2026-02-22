#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonId(pub uuid::Uuid);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    pub id: PersonId,
    pub email: Email,
    pub first_name: String,
    pub last_name: String,
    pub home_municipality: String,
    pub has_accepted_policies: bool,
}
