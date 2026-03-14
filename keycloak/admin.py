"""KeycloakAdmin — thin HTTP client for the Keycloak Admin REST API."""

from __future__ import annotations

import sys
import time
from typing import Any
from urllib.parse import quote

import requests


class KeycloakAdmin:
    def __init__(self, base_url: str, realm: str) -> None:
        self.base_url = base_url
        self.realm = realm
        self.session = requests.Session()

    def wait_until_ready(self, timeout: int = 60) -> None:
        print("Waiting for Keycloak...")
        deadline = time.time() + timeout
        while time.time() < deadline:
            try:
                r = self.session.get(f"{self.base_url}/realms/master", timeout=2)
                if r.ok:
                    print("Keycloak is ready.")
                    return
            except requests.ConnectionError:
                pass
            time.sleep(2)
        print("ERROR: Keycloak not ready", file=sys.stderr)
        sys.exit(1)

    def authenticate(self, username: str = "admin", password: str = "admin") -> None:
        r = self.session.post(
            f"{self.base_url}/realms/master/protocol/openid-connect/token",
            data={
                "client_id": "admin-cli",
                "username": username,
                "password": password,
                "grant_type": "password",
            },
        )
        r.raise_for_status()
        token = r.json()["access_token"]
        self.session.headers["Authorization"] = f"Bearer {token}"
        print("Authenticated with Keycloak admin API.")

    # -- HTTP helpers --

    def _url(self, path: str) -> str:
        return f"{self.base_url}/admin/realms/{self.realm}{path}"

    def get(self, path: str) -> Any:
        r = self.session.get(self._url(path))
        r.raise_for_status()
        return r.json()

    def get_optional(self, path: str) -> Any | None:
        """GET that returns None on 404 instead of raising."""
        r = self.session.get(self._url(path))
        if r.status_code == 404:
            return None
        r.raise_for_status()
        return r.json()

    def put(self, path: str, data: dict[str, Any]) -> None:
        r = self.session.put(self._url(path), json=data)
        r.raise_for_status()

    def post(
        self,
        path: str,
        data: list[dict[str, Any]] | dict[str, Any] | None = None,
    ) -> requests.Response:
        r = self.session.post(self._url(path), json=data)
        r.raise_for_status()
        return r

    def delete(self, path: str) -> None:
        r = self.session.delete(self._url(path))
        r.raise_for_status()

    # -- flow helpers --

    def get_flows(self) -> list[dict[str, Any]]:
        result: list[dict[str, Any]] = self.get("/authentication/flows")
        return result

    def flow_exists(self, alias: str) -> bool:
        return any(f["alias"] == alias for f in self.get_flows())

    def get_flow_id(self, alias: str) -> str | None:
        for f in self.get_flows():
            if f["alias"] == alias:
                flow_id: str = f["id"]
                return flow_id
        return None

    def copy_flow(self, source_alias: str, new_name: str) -> None:
        self.post(
            f"/authentication/flows/{quote(source_alias, safe='')}/copy",
            {"newName": new_name},
        )

    def delete_flow(self, flow_id: str) -> None:
        self.delete(f"/authentication/flows/{flow_id}")

    def get_executions(self, flow_alias: str) -> list[dict[str, Any]]:
        result: list[dict[str, Any]] = self.get(
            f"/authentication/flows/{quote(flow_alias, safe='')}/executions"
        )
        return result

    def update_execution(self, flow_alias: str, execution: dict[str, Any]) -> None:
        self.put(
            f"/authentication/flows/{quote(flow_alias, safe='')}/executions",
            execution,
        )

    def add_execution(self, flow_alias: str, provider: str) -> None:
        self.post(
            f"/authentication/flows/{quote(flow_alias, safe='')}/executions/execution",
            {"provider": provider},
        )

    def add_subflow(self, parent_alias: str, alias: str, flow_type: str) -> None:
        self.post(
            f"/authentication/flows/{quote(parent_alias, safe='')}/executions/flow",
            {
                "alias": alias,
                "type": flow_type,
                "provider": "registration-page-form",
            },
        )

    def set_execution_requirement(
        self, flow_alias: str, display_name: str, requirement: str
    ) -> None:
        for exe in self.get_executions(flow_alias):
            if exe.get("displayName") == display_name:
                exe["requirement"] = requirement
                self.update_execution(flow_alias, exe)
                return
        raise ValueError(
            f"Execution '{display_name}' not found in flow '{flow_alias}'"
        )

    def bind_flow(self, **kwargs: str) -> None:
        """Bind custom flows to the realm. E.g. browserFlow='my-browser'."""
        self.put("", kwargs)
