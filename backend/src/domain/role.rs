#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleName(pub String);

impl From<RoleName> for String {
    fn from(name: RoleName) -> Self {
        name.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    pub name: RoleName,
    pub color: Option<String>,
    pub description: Option<String>,
}
