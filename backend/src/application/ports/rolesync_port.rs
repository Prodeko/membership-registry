use crate::domain::RoleName;

pub struct IdpSubject(pub String);
#[derive(Debug)]
pub enum RoleSyncError {
    IdpError,
}

#[async_trait::async_trait]
pub trait RoleSyncPort: Send + Sync {
    async fn create_role(&self, role: &RoleName) -> Result<(), RoleSyncError>;

    async fn delete_role(&self, role: &RoleName) -> Result<(), RoleSyncError>;

    async fn assign_role(&self, subject: &IdpSubject, role: &RoleName)
        -> Result<(), RoleSyncError>;

    async fn remove_role(&self, user_id: &IdpSubject, role: &RoleName)
        -> Result<(), RoleSyncError>;

    async fn has_role(&self, user_id: &IdpSubject, role: &RoleName) -> Result<bool, RoleSyncError>;
}
