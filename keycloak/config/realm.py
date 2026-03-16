"""Realm-level settings (theme, login, SMTP)."""

from __future__ import annotations

import os
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from admin import KeycloakAdmin


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
        "bruteForceProtected": True,
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
