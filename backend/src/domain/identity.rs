/// Subject identifier issued by an external identity provider (e.g.
/// Keycloak's user UUID). Treated as opaque from the domain's perspective.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdpSubject(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdpGroupId(pub String);

impl IdpSubject {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl IdpGroupId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
