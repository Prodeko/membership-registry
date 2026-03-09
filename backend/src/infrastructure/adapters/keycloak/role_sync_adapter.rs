use crate::application::ports::rolesync_port::{IdpSubject, RoleSyncError, RoleSyncPort};
use crate::domain::RoleName;

use super::client::{KeycloakClient, RealmRoleDTO};

#[derive(Clone)]
pub struct KeycloakRoleSyncAdapter {
    client: KeycloakClient,
}

impl KeycloakRoleSyncAdapter {
    pub fn new(client: KeycloakClient) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl RoleSyncPort for KeycloakRoleSyncAdapter {
    async fn create_role(&self, role: &RoleName) -> Result<(), RoleSyncError> {
        self.client
            .create_realm_role(&role.0)
            .await
            .map_err(|_| RoleSyncError::IdpError)
    }

    async fn delete_role(&self, role: &RoleName) -> Result<(), RoleSyncError> {
        self.client
            .delete_realm_role(&role.0)
            .await
            .map_err(|_| RoleSyncError::IdpError)
    }

    async fn assign_role(
        &self,
        subject: &IdpSubject,
        role: &RoleName,
    ) -> Result<(), RoleSyncError> {
        let role_id = self
            .client
            .get_realm_role_id(&role.0)
            .await
            .map_err(|_| RoleSyncError::IdpError)?
            .ok_or(RoleSyncError::IdpError)?;

        self.client
            .add_user_realm_roles(
                &subject.0,
                &[RealmRoleDTO {
                    id: role_id,
                    name: role.0.clone(),
                }],
            )
            .await
            .map_err(|_| RoleSyncError::IdpError)
    }

    async fn remove_role(
        &self,
        subject: &IdpSubject,
        role: &RoleName,
    ) -> Result<(), RoleSyncError> {
        // If the role doesn't exist in Keycloak, there's nothing to remove
        let role_id = match self
            .client
            .get_realm_role_id(&role.0)
            .await
            .map_err(|_| RoleSyncError::IdpError)?
        {
            Some(id) => id,
            None => return Ok(()),
        };

        self.client
            .remove_user_realm_roles(
                &subject.0,
                &[RealmRoleDTO {
                    id: role_id,
                    name: role.0.clone(),
                }],
            )
            .await
            .map_err(|_| RoleSyncError::IdpError)
    }

    async fn has_role(&self, subject: &IdpSubject, role: &RoleName) -> Result<bool, RoleSyncError> {
        let roles = self
            .client
            .list_user_realm_roles(&subject.0)
            .await
            .map_err(|_| RoleSyncError::IdpError)?;

        Ok(roles.iter().any(|r| r == &role.0))
    }
}
