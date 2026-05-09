"""Client configuration (auth + M2M)."""

from __future__ import annotations

import json
import os
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from admin import KeycloakAdmin


def _env_json_list(key: str, default: list[str]) -> list[str]:
    """Read a JSON array from env, falling back to default."""
    raw = os.environ.get(key)
    if raw:
        result: list[str] = json.loads(raw)
        return result
    return default


def _find_client_id(kc: KeycloakAdmin, client_id: str) -> str | None:
    """Find a client's internal UUID by clientId."""
    clients: list[dict[str, Any]] = kc.get(f"/clients?clientId={client_id}")
    for c in clients:
        if c["clientId"] == client_id:
            uuid: str = c["id"]
            return uuid
    return None


def configure_auth_client(kc: KeycloakAdmin) -> None:
    """Configure the auth consumer client (membership-registry)."""
    client_id_str = "membership-registry"
    print(f"Configuring client '{client_id_str}'...")

    client_payload: dict[str, Any] = {
        "clientId": client_id_str,
        "enabled": True,
        "publicClient": False,
        "secret": os.environ.get("KC_AUTH_CLIENT_SECRET", "dev-secret-membership-registry"),
        "redirectUris": _env_json_list("KC_AUTH_REDIRECT_URIS", [
            "http://127.0.0.1:5173/*",
            "http://localhost:5173/*",
            "http://127.0.0.1:8080/*",
            "http://localhost:8080/*",
        ]),
        "webOrigins": _env_json_list("KC_AUTH_WEB_ORIGINS", [
            "http://127.0.0.1:5173",
            "http://localhost:5173",
        ]),
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
            {
                "name": "locale",
                "protocol": "openid-connect",
                "protocolMapper": "oidc-usermodel-attribute-mapper",
                "config": {
                    "claim.name": "locale",
                    "user.attribute": "locale",
                    "id.token.claim": "true",
                    "access.token.claim": "true",
                    "userinfo.token.claim": "true",
                    "jsonType.label": "String",
                },
            },
        ],
    }

    existing_id = _find_client_id(kc, client_id_str)
    if existing_id:
        kc.put(f"/clients/{existing_id}", client_payload)
        print(f"  Updated client '{client_id_str}'.")
    else:
        kc.post("/clients", client_payload)
        print(f"  Created client '{client_id_str}'.")


def _assign_service_account_roles(
    kc: KeycloakAdmin, client_id_str: str, client_uuid: str | None
) -> None:
    """Assign realm-management roles to the service account."""
    sa_user: dict[str, Any] = kc.get(f"/clients/{client_uuid}/service-account-user")
    sa_user_id: str = sa_user["id"]

    rm_client_id = _find_client_id(kc, "realm-management")
    if not rm_client_id:
        print("  WARNING: realm-management client not found, skipping role assignment.")
        return

    available_roles: list[dict[str, Any]] = kc.get(
        f"/users/{sa_user_id}/role-mappings/clients/{rm_client_id}/available"
    )
    available_by_name = {r["name"]: r for r in available_roles}

    desired_roles = [
        "manage-users",
        "view-users",
        "manage-realm",
        "view-realm",
        "manage-clients",
        "view-clients",
    ]
    roles_to_assign = [
        available_by_name[name] for name in desired_roles if name in available_by_name
    ]

    if roles_to_assign:
        kc.post(
            f"/users/{sa_user_id}/role-mappings/clients/{rm_client_id}",
            roles_to_assign,
        )
        assigned_names = [r["name"] for r in roles_to_assign]
        print(f"  Assigned realm-management roles: {', '.join(assigned_names)}")
    else:
        print("  Service account roles already assigned.")


def configure_m2m_client(kc: KeycloakAdmin) -> None:
    """Configure the M2M client (membership-registry-m2m)."""
    client_id_str = "membership-registry-m2m"
    print(f"Configuring client '{client_id_str}'...")

    client_payload: dict[str, Any] = {
        "clientId": client_id_str,
        "enabled": True,
        "publicClient": False,
        "secret": os.environ.get("KC_M2M_CLIENT_SECRET", "dev-secret-membership-registry-m2m"),
        "serviceAccountsEnabled": True,
        "standardFlowEnabled": False,
        "directAccessGrantsEnabled": False,
        "protocol": "openid-connect",
        "fullScopeAllowed": True,
    }

    existing_id = _find_client_id(kc, client_id_str)
    if existing_id:
        kc.put(f"/clients/{existing_id}", client_payload)
        print(f"  Updated client '{client_id_str}'.")
    else:
        kc.post("/clients", client_payload)
        existing_id = _find_client_id(kc, client_id_str)
        print(f"  Created client '{client_id_str}'.")

    _assign_service_account_roles(kc, client_id_str, existing_id)
