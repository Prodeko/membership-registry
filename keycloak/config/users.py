"""Test user configuration."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from admin import KeycloakAdmin


def _ensure_user(
    kc: KeycloakAdmin,
    username: str,
    email: str,
    password: str,
    roles: list[str] | None = None,
) -> None:
    """Create or update a user, with optional realm roles."""
    print(f"Configuring user '{username}' ({email})...")

    user_payload: dict[str, Any] = {
        "username": username,
        "email": email,
        "firstName": "Test",
        "lastName": "User",
        "enabled": True,
        "emailVerified": True,
    }

    existing: list[dict[str, Any]] = kc.get(f"/users?username={username}&exact=true")
    if existing:
        user_id: str = existing[0]["id"]
        kc.put(f"/users/{user_id}", user_payload)
        print(f"  Updated user '{username}'.")
    else:
        user_payload["credentials"] = [
            {"type": "password", "value": password, "temporary": False}
        ]
        kc.post("/users", user_payload)
        existing = kc.get(f"/users?username={username}&exact=true")
        user_id = existing[0]["id"]
        print(f"  Created user '{username}'.")

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


def configure_test_users(kc: KeycloakAdmin) -> None:
    """Create test users for development and e2e testing."""
    _ensure_user(
        kc,
        username="testadmin",
        email="test-admin@example.com",
        password="testpassword",
        roles=["admin"],
    )
    _ensure_user(
        kc,
        username="testuser",
        email="test-user@example.com",
        password="testpassword",
    )
