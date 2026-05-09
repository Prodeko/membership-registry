"""Client scope configuration for the registry-attributes feature.

Creates a single client scope `registry-attributes` and attaches it as a
default scope to the `membership-registry` auth client. The backend manages
the protocol mappers inside this scope at runtime as admins create attribute
definitions.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from admin import KeycloakAdmin


REGISTRY_SCOPE_NAME = "registry-attributes"
AUTH_CLIENT_ID = "membership-registry"


def _find_scope_id(kc: KeycloakAdmin, scope_name: str) -> str | None:
    scopes: list[dict[str, Any]] = kc.get("/client-scopes")
    for s in scopes:
        if s.get("name") == scope_name:
            scope_id: str = s["id"]
            return scope_id
    return None


def _find_client_id(kc: KeycloakAdmin, client_id: str) -> str | None:
    clients: list[dict[str, Any]] = kc.get(f"/clients?clientId={client_id}")
    for c in clients:
        if c["clientId"] == client_id:
            uuid: str = c["id"]
            return uuid
    return None


def configure_registry_attributes_scope(kc: KeycloakAdmin) -> None:
    """Idempotent: create the scope and attach it to the auth client."""
    print(f"Configuring client scope '{REGISTRY_SCOPE_NAME}'...")

    scope_payload: dict[str, Any] = {
        "name": REGISTRY_SCOPE_NAME,
        "description": (
            "User attributes managed by membership-registry, exposed to "
            "clients via JWT claims"
        ),
        "protocol": "openid-connect",
        "attributes": {
            "include.in.token.scope": "true",
            "display.on.consent.screen": "false",
        },
    }

    existing = _find_scope_id(kc, REGISTRY_SCOPE_NAME)
    if existing:
        kc.put(f"/client-scopes/{existing}", scope_payload)
        print(f"  Updated client scope '{REGISTRY_SCOPE_NAME}'.")
    else:
        kc.post("/client-scopes", scope_payload)
        existing = _find_scope_id(kc, REGISTRY_SCOPE_NAME)
        print(f"  Created client scope '{REGISTRY_SCOPE_NAME}'.")

    if not existing:
        print(f"  WARNING: failed to resolve scope id for '{REGISTRY_SCOPE_NAME}'.")
        return

    auth_client_uuid = _find_client_id(kc, AUTH_CLIENT_ID)
    if not auth_client_uuid:
        print(
            f"  WARNING: client '{AUTH_CLIENT_ID}' not found, "
            "skipping default-scope assignment."
        )
        return

    # PUT is idempotent — re-attaching an already-default scope is a no-op.
    kc.put(
        f"/clients/{auth_client_uuid}/default-client-scopes/{existing}",
        {},
    )
    print(
        f"  Attached '{REGISTRY_SCOPE_NAME}' as a default scope on "
        f"'{AUTH_CLIENT_ID}'."
    )
