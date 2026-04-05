"""Test user configuration."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from admin import KeycloakAdmin


def _ensure_user(
    kc: KeycloakAdmin,
    email: str,
    password: str,
    roles: list[str] | None = None,
) -> None:
    """Create or update a user, with optional realm roles.

    With registrationEmailAsUsername=True, username is always set to email.
    Looks up existing users by email for idempotency.
    """
    print(f"Configuring user '{email}'...")

    user_payload: dict[str, Any] = {
        "username": email,
        "email": email,
        "firstName": "Test",
        "lastName": "User",
        "enabled": True,
        "emailVerified": True,
    }

    existing: list[dict[str, Any]] = kc.get(f"/users?email={email}&exact=true")
    if existing:
        user_id: str = existing[0]["id"]
        kc.put(f"/users/{user_id}", user_payload)
        print(f"  Updated user '{email}'.")
    else:
        user_payload["credentials"] = [
            {"type": "password", "value": password, "temporary": False}
        ]
        kc.post("/users", user_payload)
        existing = kc.get(f"/users?email={email}&exact=true")
        user_id = existing[0]["id"]
        print(f"  Created user '{email}'.")

    kc.put(f"/users/{user_id}/reset-password", {
        "type": "password",
        "value": password,
        "temporary": False,
    })

    if roles:
        all_roles: list[dict[str, Any]] = kc.get("/roles")
        roles_to_assign = [r for r in all_roles if r["name"] in roles]
        if roles_to_assign:
            kc.post(f"/users/{user_id}/role-mappings/realm", roles_to_assign)
            assigned = [r["name"] for r in roles_to_assign]
            print(f"  Assigned roles: {', '.join(assigned)}")


def _assign_realm_management_roles(
    kc: KeycloakAdmin,
    email: str,
    roles: list[str],
) -> None:
    """Assign realm-management client roles to a user (for Keycloak admin access)."""
    existing: list[dict[str, Any]] = kc.get(f"/users?email={email}&exact=true")
    if not existing:
        print(f"  WARNING: user '{email}' not found, skipping realm-management roles.")
        return
    user_id: str = existing[0]["id"]

    clients: list[dict[str, Any]] = kc.get("/clients?clientId=realm-management")
    rm_client_id: str | None = None
    for c in clients:
        if c["clientId"] == "realm-management":
            rm_client_id = c["id"]
            break
    if not rm_client_id:
        print("  WARNING: realm-management client not found, skipping role assignment.")
        return

    available: list[dict[str, Any]] = kc.get(
        f"/users/{user_id}/role-mappings/clients/{rm_client_id}/available"
    )
    available_by_name = {r["name"]: r for r in available}
    to_assign = [available_by_name[name] for name in roles if name in available_by_name]

    if to_assign:
        kc.post(f"/users/{user_id}/role-mappings/clients/{rm_client_id}", to_assign)
        assigned = [r["name"] for r in to_assign]
        print(f"  Assigned realm-management roles to '{email}': {', '.join(assigned)}")
    else:
        print(f"  Realm-management roles already assigned to '{email}'.")


def configure_test_users(kc: KeycloakAdmin) -> None:
    """Create test users for development and e2e testing."""
    _ensure_user(
        kc,
        email="cto@prodeko.org",
        password="kananugetti",
        roles=["admin"],
    )
    _assign_realm_management_roles(
        kc,
        email="cto@prodeko.org",
        roles=["realm-admin"],
    )
    # E2e test users, one per parallel Playwright worker. Each worker
    # needs its own Keycloak user because tests clean up member rows in
    # beforeEach — sharing would race. Keep this count in sync with
    # E2E_WORKERS in .github/workflows/e2e.yml.
    for i in range(4):
        _ensure_user(
            kc,
            email=f"user-{i}@prodeko.org",
            password="sateenkaari",
        )
