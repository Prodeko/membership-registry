"""Authentication flow configuration."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from admin import KeycloakAdmin


def configure_browser_flow(kc: KeycloakAdmin) -> None:
    """Configure the browser (login) flow.

    Customize this function to modify the login experience.
    Example: copy the default browser flow and add/remove steps.

    To add OTP:
        if not kc.flow_exists("membership-browser"):
            kc.copy_flow("browser", "membership-browser")
            kc.set_execution_requirement("membership-browser", "OTP Form", "REQUIRED")
            kc.bind_flow(browserFlow="membership-browser")
    """
    print("  Browser flow: default (edit configure_browser_flow to customize)")


def configure_registration_flow(kc: KeycloakAdmin) -> None:
    """Configure the registration flow.

    Customize this function to modify the registration experience.
    Example: copy the default registration flow and modify fields.

    To customize:
        if not kc.flow_exists("membership-registration"):
            kc.copy_flow("registration", "membership-registration")
            # ... modify executions ...
            kc.bind_flow(registrationFlow="membership-registration")
    """
    print("  Registration flow: default (edit configure_registration_flow to customize)")
