"""Realm-level settings (theme, login, SMTP)."""

from __future__ import annotations

import os
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from admin import KeycloakAdmin


def _allow_unmanaged_user_attributes(kc: KeycloakAdmin) -> None:
    """Allow the registry to push arbitrary user attributes via the admin API.

    KC 26 ships with Declarative User Profile and `unmanagedAttributePolicy`
    defaulting to DISABLED, which means PUT /users/{id} silently drops
    attribute keys that aren't declared in the user-profile schema. We set it
    to ADMIN_EDIT so admins can see and edit registry-managed attributes in
    Keycloak's user page; end users don't see them in the account console.
    """
    profile: dict[str, Any] = kc.get("/users/profile")
    if profile.get("unmanagedAttributePolicy") == "ADMIN_EDIT":
        return
    profile["unmanagedAttributePolicy"] = "ADMIN_EDIT"
    kc.put("/users/profile", profile)
    print("  Set unmanagedAttributePolicy=ADMIN_EDIT.")


def configure_realm(kc: KeycloakAdmin) -> None:
    print("Configuring realm settings...")

    smtp_from = os.environ.get("SMTP_FROM", "noreply@example.com")
    smtp_from_name = os.environ.get("SMTP_FROM_NAME", "Membership Registry")
    sendgrid_api_key = os.environ.get("SENDGRID_API_KEY", "")

    realm_payload: dict[str, object] = {
        "loginTheme": "membership",
        "accountTheme": "membership",
        "registrationAllowed": True,
        "verifyEmail": True,
        "resetPasswordAllowed": True,
        "loginWithEmailAllowed": True,
        "registrationEmailAsUsername": True,
        "duplicateEmailsAllowed": False,
        "editUsernameAllowed": False,
        "bruteForceProtected": os.environ.get("DISABLE_BRUTE_FORCE_PROTECTION") != "true",
        "accessTokenLifespan": 1800,
        "ssoSessionIdleTimeout": 1800,
        "ssoSessionMaxLifespan": 36000,
        "internationalizationEnabled": True,
        "supportedLocales": ["fi", "en"],
        "defaultLocale": "fi",
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

    kc.put("", realm_payload)

    _allow_unmanaged_user_attributes(kc)
