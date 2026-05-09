use crate::domain::RoleName;

pub use crate::domain::{IdpGroupId, IdpSubject};

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

    async fn list_role_members(&self, role: &RoleName) -> Result<Vec<IdpSubject>, RoleSyncError>;

    async fn create_group(&self, name: &str) -> Result<IdpGroupId, RoleSyncError>;

    async fn delete_group(&self, id: &IdpGroupId) -> Result<(), RoleSyncError>;

    async fn set_group_roles(
        &self,
        id: &IdpGroupId,
        roles: &[RoleName],
    ) -> Result<(), RoleSyncError>;

    async fn add_user_to_group(
        &self,
        subject: &IdpSubject,
        group_id: &IdpGroupId,
    ) -> Result<(), RoleSyncError>;

    async fn remove_user_from_group(
        &self,
        subject: &IdpSubject,
        group_id: &IdpGroupId,
    ) -> Result<(), RoleSyncError>;
}
