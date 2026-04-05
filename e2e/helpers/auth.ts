import type { Page } from "@playwright/test";
import { KeycloakLoginPage } from "../pages/keycloak-login.page";
import { API_BASE_URL } from "./constants";

export async function loginViaKeycloak(
  page: Page,
  email: string,
  password: string,
): Promise<void> {
  await page.goto(`${API_BASE_URL}/auth/login`);
  const keycloak = new KeycloakLoginPage(page);
  await keycloak.login(email, password);
  await page.waitForURL("**/home", { timeout: 30_000 });
}
