"""Authentication flow configuration."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from admin import KeycloakAdmin

BROWSER_FLOW_ALIAS = "membership-browser"


def configure_passkeys(kc: KeycloakAdmin) -> None:
    """Enable optional passkey (WebAuthn passwordless) authentication.

    Configures realm-level WebAuthn policy and registers the required action
    so users can add passkeys from the account console.
    """
    print("Configuring passkeys...")

    kc.put("", {
        "webAuthnPolicyPasswordlessUserVerificationRequirement": "required",
        "webAuthnPolicyPasswordlessRequireResidentKey": "Yes",
    })

    kc.post_ignore_conflict("/authentication/register-required-action", {
        "providerId": "webauthn-register-passwordless",
        "name": "Webauthn Register Passwordless",
    })

    kc.put("/authentication/required-actions/webauthn-register-passwordless", {
        "alias": "webauthn-register-passwordless",
        "name": "Webauthn Register Passwordless",
        "providerId": "webauthn-register-passwordless",
        "enabled": True,
        "defaultAction": False,
        "priority": 70,
    })
    print("  Passkeys enabled (users self-register via account console).")


def configure_browser_flow(kc: KeycloakAdmin) -> None:
    """Build a custom browser flow: email → password OR passkey.

    Target structure inside the "forms" subflow:
        Username Form                          [REQUIRED]
        Password or Passkey (subflow)          [REQUIRED]
            Password Form                      [ALTERNATIVE]
            WebAuthn Passwordless Authenticator [ALTERNATIVE]
        Browser - Conditional 2FA (subflow)    [CONDITIONAL]  (kept from default)
    """
    print("Configuring browser flow...")

    if kc.flow_exists(BROWSER_FLOW_ALIAS):
        # Delete and recreate to ensure it matches the desired structure
        flow_id = kc.get_flow_id(BROWSER_FLOW_ALIAS)
        # Unbind first if it's currently bound
        realm = kc.get("")
        if realm.get("browserFlow") == BROWSER_FLOW_ALIAS and flow_id:
            kc.bind_flow(browserFlow="browser")
        if flow_id:
            kc.delete_flow(flow_id)

    kc.copy_flow("browser", BROWSER_FLOW_ALIAS)

    # Find the forms subflow alias (copy renames it to "membership-browser forms")
    forms_alias = _find_forms_alias(kc)

    # Delete the "Username Password Form" execution — we'll replace it with a two-step
    _delete_execution_by_provider(kc, forms_alias, "auth-username-password-form")

    # Step 1: Username (email) form
    kc.add_execution(forms_alias, "auth-username-form")
    kc.set_execution_requirement(BROWSER_FLOW_ALIAS, "Username Form", "REQUIRED")

    # Step 2: Subflow where password and passkey are alternatives
    subflow_alias = "Password or Passkey"
    kc.add_subflow(forms_alias, subflow_alias, "basic-flow")
    kc.set_execution_requirement(BROWSER_FLOW_ALIAS, subflow_alias, "REQUIRED")

    kc.add_execution(subflow_alias, "auth-password-form")
    kc.set_execution_requirement(BROWSER_FLOW_ALIAS, "Password Form", "ALTERNATIVE")
    kc.add_execution(subflow_alias, "webauthn-authenticator-passwordless")
    kc.set_execution_requirement(
        BROWSER_FLOW_ALIAS, "WebAuthn Passwordless Authenticator", "ALTERNATIVE"
    )

    # Fix ordering: move Username Form and Password or Passkey above Conditional 2FA
    username_exe = kc.find_execution(BROWSER_FLOW_ALIAS, "Username Form")
    kc.raise_execution_priority(username_exe["id"])
    passkey_exe = kc.find_execution(BROWSER_FLOW_ALIAS, subflow_alias)
    kc.raise_execution_priority(passkey_exe["id"])

    # Bind the custom flow to the realm
    kc.bind_flow(browserFlow=BROWSER_FLOW_ALIAS)
    print(f"  Bound '{BROWSER_FLOW_ALIAS}' (email → password or passkey).")


def _find_forms_alias(kc: KeycloakAdmin) -> str:
    """Find the alias of the 'forms' subflow in the custom browser flow."""
    for exe in kc.get_executions(BROWSER_FLOW_ALIAS):
        if exe.get("authenticationFlow") and "forms" in exe.get("displayName", "").lower():
            alias: str = exe["displayName"]
            return alias
    raise ValueError(f"Could not find forms subflow in '{BROWSER_FLOW_ALIAS}'")


def _delete_execution_by_provider(
    kc: KeycloakAdmin, flow_alias: str, provider_id: str
) -> None:
    """Delete an execution from a flow by its provider ID."""
    for exe in kc.get_executions(flow_alias):
        if exe.get("providerId") == provider_id:
            kc.delete_execution(exe["id"])
            return
    raise ValueError(f"Execution with provider '{provider_id}' not found in '{flow_alias}'")


def configure_registration_flow(kc: KeycloakAdmin) -> None:
    """Configure the registration flow.

    With registrationEmailAsUsername=True (set in realm config), the default
    registration flow only shows email, password, first name, and last name.
    No custom flow needed.
    """
    print("  Registration flow: default (username hidden via registrationEmailAsUsername)")
