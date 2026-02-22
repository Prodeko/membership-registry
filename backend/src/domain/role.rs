#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleName(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    pub name: RoleName,
    pub color: String,
    pub description: Option<String>,
}
