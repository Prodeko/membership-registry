"""Realm role configuration."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from admin import KeycloakAdmin


def configure_realm_roles(kc: KeycloakAdmin) -> None:
    print("Configuring realm roles...")
    existing: set[str] = {r["name"] for r in kc.get("/roles")}
    roles: list[dict[str, str]] = [
        {"name": "admin", "description": "Administrator role"},
        {"name": "membership", "description": "Membership role"},
        {"name": "prodeko-external-member", "description": "External member role"},
    ]
    for role in roles:
        if role["name"] in existing:
            print(f"  Role '{role['name']}' already exists, skipping.")
        else:
            kc.post("/roles", role)
            print(f"  Created role '{role['name']}'.")
