#!/usr/bin/env python3
"""
Configures Keycloak realm via Admin REST API.

Run after `docker compose up` once Keycloak is ready.
Idempotent — safe to run multiple times.

Usage:
    python keycloak/setup.py
    KEYCLOAK_URL=http://localhost:8180 python keycloak/setup.py
"""

import os
import sys
import time
from urllib.parse import quote

from dotenv import load_dotenv
import requests

load_dotenv(os.path.join(os.path.dirname(__file__), ".env"))

BASE_URL = os.environ.get("KEYCLOAK_URL", "http://localhost:8180")
REALM = "membership-registry"


class KeycloakAdmin:
    def __init__(self, base_url: str, realm: str):
        self.base_url = base_url
        self.realm = realm
        self.session = requests.Session()

    def wait_until_ready(self, timeout: int = 60):
        print("Waiting for Keycloak...")
        deadline = time.time() + timeout
        while time.time() < deadline:
            try:
                r = self.session.get(f"{self.base_url}/realms/master", timeout=2)
                if r.ok:
                    print("Keycloak is ready.")
                    return
            except requests.ConnectionError:
                pass
            time.sleep(2)
        print("ERROR: Keycloak not ready", file=sys.stderr)
        sys.exit(1)

    def authenticate(self, username: str = "admin", password: str = "admin"):
        r = self.session.post(
            f"{self.base_url}/realms/master/protocol/openid-connect/token",
            data={
                "client_id": "admin-cli",
                "username": username,
                "password": password,
                "grant_type": "password",
            },
        )
        r.raise_for_status()
        token = r.json()["access_token"]
        self.session.headers["Authorization"] = f"Bearer {token}"
        print("Authenticated with Keycloak admin API.")

    def _url(self, path: str) -> str:
        return f"{self.base_url}/admin/realms/{self.realm}{path}"

    def get(self, path: str):
        r = self.session.get(self._url(path))
        r.raise_for_status()
        return r.json()

    def get_optional(self, path: str) -> dict | list | None:
        """GET that returns None on 404 instead of raising."""
        r = self.session.get(self._url(path))
        if r.status_code == 404:
            return None
        r.raise_for_status()
        return r.json()

    def put(self, path: str, data: dict):
        r = self.session.put(self._url(path), json=data)
        r.raise_for_status()

    def post(self, path: str, data: dict | None = None) -> requests.Response:
        r = self.session.post(self._url(path), json=data)
        r.raise_for_status()
        return r

    def delete(self, path: str):
        r = self.session.delete(self._url(path))
        r.raise_for_status()

    # -- realm settings --

    def configure_realm(self):
        print("Configuring realm settings...")

        smtp_from = os.environ.get("SMTP_FROM", "noreply@example.com")
        smtp_from_name = os.environ.get("SMTP_FROM_NAME", "Membership Registry")
        sendgrid_api_key = os.environ.get("SENDGRID_API_KEY", "")

        realm_payload = {
            "loginTheme": "membership",
            "registrationAllowed": True,
            "verifyEmail": True,
            "resetPasswordAllowed": True,
            "loginWithEmailAllowed": True,
            "duplicateEmailsAllowed": False,
            "editUsernameAllowed": False,
            "bruteForceProtected": True,
            "accessTokenLifespan": 1800,
            "ssoSessionIdleTimeout": 1800,
            "ssoSessionMaxLifespan": 36000,
        }

        if sendgrid_api_key:
            realm_payload["smtpServer"] = {
                "host": "smtp.sendgrid.net",
                "port": "587",
                "from": smtp_from,
                "fromDisplayName": smtp_from_name,
                "auth": "true",
                "starttls": "true",
                "user": "apikey",
                "password": sendgrid_api_key,
            }
            print("  SMTP configured (SendGrid).")
        else:
            print("  SMTP not configured (set SENDGRID_API_KEY to enable).")

        self.put("", realm_payload)

    # -- realm roles --

    def configure_realm_roles(self):
        print("Configuring realm roles...")
        existing = {r["name"] for r in self.get("/roles")}
        roles = [
            {"name": "admin", "description": "Administrator role"},
            {"name": "membership", "description": "Membership role"},
            {"name": "prodeko-external-member", "description": "External member role"},
        ]
        for role in roles:
            if role["name"] in existing:
                print(f"  Role '{role['name']}' already exists, skipping.")
            else:
                self.post("/roles", role)
                print(f"  Created role '{role['name']}'.")

    # -- clients --

    def _find_client_id(self, client_id: str) -> str | None:
        """Find a client's internal UUID by clientId."""
        clients = self.get(f"/clients?clientId={client_id}")
        for c in clients:
            if c["clientId"] == client_id:
                return c["id"]
        return None

    def configure_auth_client(self):
        """Configure the auth consumer client (membership-registry)."""
        client_id_str = "membership-registry"
        print(f"Configuring client '{client_id_str}'...")

        client_payload = {
            "clientId": client_id_str,
            "enabled": True,
            "publicClient": False,
            "secret": "dev-secret-membership-registry",
            "redirectUris": [
                "http://127.0.0.1:5173/*",
                "http://localhost:5173/*",
                "http://127.0.0.1:8080/*",
                "http://localhost:8080/*",
            ],
            "webOrigins": [
                "http://127.0.0.1:5173",
                "http://localhost:5173",
            ],
            "standardFlowEnabled": True,
            "directAccessGrantsEnabled": True,
            "protocol": "openid-connect",
            "fullScopeAllowed": True,
            "protocolMappers": [
                {
                    "name": "email",
                    "protocol": "openid-connect",
                    "protocolMapper": "oidc-usermodel-property-mapper",
                    "config": {
                        "claim.name": "email",
                        "user.attribute": "email",
                        "id.token.claim": "true",
                        "access.token.claim": "true",
                        "userinfo.token.claim": "true",
                        "jsonType.label": "String",
                    },
                },
                {
                    "name": "given_name",
                    "protocol": "openid-connect",
                    "protocolMapper": "oidc-usermodel-property-mapper",
                    "config": {
                        "claim.name": "given_name",
                        "user.attribute": "firstName",
                        "id.token.claim": "true",
                        "access.token.claim": "true",
                        "userinfo.token.claim": "true",
                        "jsonType.label": "String",
                    },
                },
                {
                    "name": "family_name",
                    "protocol": "openid-connect",
                    "protocolMapper": "oidc-usermodel-property-mapper",
                    "config": {
                        "claim.name": "family_name",
                        "user.attribute": "lastName",
                        "id.token.claim": "true",
                        "access.token.claim": "true",
                        "userinfo.token.claim": "true",
                        "jsonType.label": "String",
                    },
                },
            ],
        }

        existing_id = self._find_client_id(client_id_str)
        if existing_id:
            self.put(f"/clients/{existing_id}", client_payload)
            print(f"  Updated client '{client_id_str}'.")
        else:
            self.post("/clients", client_payload)
            print(f"  Created client '{client_id_str}'.")

    def configure_m2m_client(self):
        """Configure the M2M client (membership-registry-m2m)."""
        client_id_str = "membership-registry-m2m"
        print(f"Configuring client '{client_id_str}'...")

        client_payload = {
            "clientId": client_id_str,
            "enabled": True,
            "publicClient": False,
            "secret": "dev-secret-membership-registry-m2m",
            "serviceAccountsEnabled": True,
            "standardFlowEnabled": False,
            "directAccessGrantsEnabled": False,
            "protocol": "openid-connect",
            "fullScopeAllowed": True,
        }

        existing_id = self._find_client_id(client_id_str)
        if existing_id:
            self.put(f"/clients/{existing_id}", client_payload)
            print(f"  Updated client '{client_id_str}'.")
        else:
            self.post("/clients", client_payload)
            existing_id = self._find_client_id(client_id_str)
            print(f"  Created client '{client_id_str}'.")

        self._assign_service_account_roles(client_id_str, existing_id)

    def _assign_service_account_roles(self, client_id_str: str, client_uuid: str):
        """Assign realm-management roles to the service account."""
        # Get the service account user
        sa_user = self.get(f"/clients/{client_uuid}/service-account-user")
        sa_user_id = sa_user["id"]

        # Find the realm-management client UUID
        rm_client_id = self._find_client_id("realm-management")
        if not rm_client_id:
            print("  WARNING: realm-management client not found, skipping role assignment.")
            return

        # Get available roles for this client
        available_roles = self.get(
            f"/users/{sa_user_id}/role-mappings/clients/{rm_client_id}/available"
        )
        available_by_name = {r["name"]: r for r in available_roles}

        desired_roles = ["manage-users", "view-users", "manage-realm", "view-realm"]
        roles_to_assign = []
        for role_name in desired_roles:
            if role_name in available_by_name:
                roles_to_assign.append(available_by_name[role_name])

        if roles_to_assign:
            self.post(
                f"/users/{sa_user_id}/role-mappings/clients/{rm_client_id}",
                roles_to_assign,
            )
            assigned_names = [r["name"] for r in roles_to_assign]
            print(f"  Assigned realm-management roles: {', '.join(assigned_names)}")
        else:
            print("  Service account roles already assigned.")

    # -- test users --

    def _ensure_user(
        self,
        username: str,
        email: str,
        password: str,
        roles: list[str] | None = None,
    ):
        """Create or update a user, with optional realm roles."""
        print(f"Configuring user '{username}' ({email})...")

        user_payload = {
            "username": username,
            "email": email,
            "firstName": "Test",
            "lastName": "User",
            "enabled": True,
            "emailVerified": True,
        }

        existing = self.get(f"/users?username={username}&exact=true")
        if existing:
            user_id = existing[0]["id"]
            self.put(f"/users/{user_id}", user_payload)
            print(f"  Updated user '{username}'.")
        else:
            user_payload["credentials"] = [
                {"type": "password", "value": password, "temporary": False}
            ]
            self.post("/users", user_payload)
            existing = self.get(f"/users?username={username}&exact=true")
            user_id = existing[0]["id"]
            print(f"  Created user '{username}'.")

        # Reset password (idempotent)
        self.put(f"/users/{user_id}/reset-password", {
            "type": "password",
            "value": password,
            "temporary": False,
        })

        if roles:
            all_roles = self.get("/roles")
            roles_to_assign = [r for r in all_roles if r["name"] in roles]
            if roles_to_assign:
                self.post(f"/users/{user_id}/role-mappings/realm", roles_to_assign)
                assigned = [r["name"] for r in roles_to_assign]
                print(f"  Assigned roles: {', '.join(assigned)}")

    def configure_test_users(self):
        """Create test users for development and e2e testing."""
        self._ensure_user(
            username="testadmin",
            email="test-admin@example.com",
            password="testpassword",
            roles=["admin"],
        )
        self._ensure_user(
            username="testuser",
            email="test-user@example.com",
            password="testpassword",
        )

    # -- authentication flows --

    def get_flows(self) -> list[dict]:
        return self.get("/authentication/flows")

    def flow_exists(self, alias: str) -> bool:
        return any(f["alias"] == alias for f in self.get_flows())

    def get_flow_id(self, alias: str) -> str | None:
        for f in self.get_flows():
            if f["alias"] == alias:
                return f["id"]
        return None

    def copy_flow(self, source_alias: str, new_name: str):
        self.post(
            f"/authentication/flows/{quote(source_alias, safe='')}/copy",
            {"newName": new_name},
        )

    def delete_flow(self, flow_id: str):
        self.delete(f"/authentication/flows/{flow_id}")

    def get_executions(self, flow_alias: str) -> list[dict]:
        return self.get(f"/authentication/flows/{quote(flow_alias, safe='')}/executions")

    def update_execution(self, flow_alias: str, execution: dict):
        self.put(
            f"/authentication/flows/{quote(flow_alias, safe='')}/executions",
            execution,
        )

    def add_execution(self, flow_alias: str, provider: str):
        self.post(
            f"/authentication/flows/{quote(flow_alias, safe='')}/executions/execution",
            {"provider": provider},
        )

    def add_subflow(self, parent_alias: str, alias: str, flow_type: str):
        self.post(
            f"/authentication/flows/{quote(parent_alias, safe='')}/executions/flow",
            {
                "alias": alias,
                "type": flow_type,
                "provider": "registration-page-form",
            },
        )

    def set_execution_requirement(
        self, flow_alias: str, display_name: str, requirement: str
    ):
        for exe in self.get_executions(flow_alias):
            if exe.get("displayName") == display_name:
                exe["requirement"] = requirement
                self.update_execution(flow_alias, exe)
                return
        raise ValueError(
            f"Execution '{display_name}' not found in flow '{flow_alias}'"
        )

    def bind_flow(self, **kwargs):
        """Bind custom flows to the realm. E.g. browserFlow='my-browser'."""
        self.put("", kwargs)

    # -- flow configuration --

    def configure_browser_flow(self):
        """Configure the browser (login) flow.

        Customize this method to modify the login experience.
        Example: copy the default browser flow and add/remove steps.

        To add OTP:
            if not self.flow_exists("membership-browser"):
                self.copy_flow("browser", "membership-browser")
                self.set_execution_requirement("membership-browser", "OTP Form", "REQUIRED")
                self.bind_flow(browserFlow="membership-browser")
        """
        print("  Browser flow: default (edit configure_browser_flow to customize)")

    def configure_registration_flow(self):
        """Configure the registration flow.

        Customize this method to modify the registration experience.
        Example: copy the default registration flow and modify fields.

        To customize:
            if not self.flow_exists("membership-registration"):
                self.copy_flow("registration", "membership-registration")
                # ... modify executions ...
                self.bind_flow(registrationFlow="membership-registration")
        """
        print("  Registration flow: default (edit configure_registration_flow to customize)")


def main():
    kc = KeycloakAdmin(BASE_URL, REALM)
    kc.wait_until_ready()
    kc.authenticate()

    kc.configure_realm()
    kc.configure_realm_roles()
    kc.configure_auth_client()
    kc.configure_m2m_client()
    kc.configure_test_users()

    print("Configuring authentication flows...")
    kc.configure_browser_flow()
    kc.configure_registration_flow()

    print("\nSetup complete.")


if __name__ == "__main__":
    main()
