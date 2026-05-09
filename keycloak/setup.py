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

from dotenv import load_dotenv

from admin import KeycloakAdmin
from config.clients import configure_auth_client, configure_m2m_client
from config.flows import configure_browser_flow, configure_passkeys, configure_registration_flow
from config.realm import configure_realm
from config.roles import configure_realm_roles
from config.scopes import configure_registry_attributes_scope
from config.users import configure_test_users

load_dotenv(os.path.join(os.path.dirname(__file__), ".env"))

BASE_URL = os.environ.get("KEYCLOAK_URL", "http://localhost:8180")
REALM = "membership-registry"


def main() -> None:
    kc = KeycloakAdmin(BASE_URL, REALM)
    kc.wait_until_ready()
    kc.authenticate()

    configure_realm(kc)
    configure_realm_roles(kc)
    configure_auth_client(kc)
    configure_m2m_client(kc)
    configure_registry_attributes_scope(kc)
    if os.environ.get("KC_SKIP_TEST_USERS", "").lower() != "true":
        configure_test_users(kc)
    else:
        print("Skipping test users (KC_SKIP_TEST_USERS=true).")

    print("Configuring authentication flows...")
    configure_passkeys(kc)
    configure_browser_flow(kc)
    configure_registration_flow(kc)

    print("\nSetup complete.")


if __name__ == "__main__":
    main()
