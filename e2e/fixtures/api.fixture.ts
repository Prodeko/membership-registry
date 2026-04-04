import { type BrowserContext, chromium } from "@playwright/test";
import { KeycloakLoginPage } from "../pages/keycloak-login.page";
import {
  API_BASE_URL,
  ADMIN_EMAIL,
  ADMIN_PASSWORD,
} from "../helpers/constants";

/**
 * Admin API helper that authenticates via a headless browser
 * (Keycloak OAuth flow) and then uses the session cookies for API requests.
 */
export class AdminApiHelper {
  private cookies: string = "";
  private context: BrowserContext | null = null;

  async authenticate(): Promise<void> {
    if (this.cookies) return;

    const browser = await chromium.launch();
    this.context = await browser.newContext();
    const page = await this.context.newPage();

    // Login via Keycloak
    await page.goto(`${API_BASE_URL}/auth/login`);
    const keycloak = new KeycloakLoginPage(page);
    await keycloak.login(ADMIN_EMAIL, ADMIN_PASSWORD);
    await page.waitForURL("**/home", { timeout: 30_000 });

    // Extract cookies from the browser context
    const allCookies = await this.context.cookies();
    this.cookies = allCookies
      .map((c) => `${c.name}=${c.value}`)
      .join("; ");

    await page.close();
  }

  async cleanup(): Promise<void> {
    if (this.context) {
      await this.context.close();
      this.context = null;
    }
  }

  private async request(
    method: string,
    path: string,
    body?: unknown,
  ): Promise<Response> {
    await this.authenticate();
    return fetch(`${API_BASE_URL}${path}`, {
      method,
      headers: {
        "Content-Type": "application/json",
        Cookie: this.cookies,
      },
      body: body ? JSON.stringify(body) : undefined,
    });
  }

  async createRole(name: string): Promise<void> {
    const resp = await this.request("POST", "/admin/roles", { name });
    if (!resp.ok && resp.status !== 409) {
      throw new Error(`Create role failed: ${resp.status} ${await resp.text()}`);
    }
  }

  async approveApplication(applicationId: string): Promise<void> {
    const resp = await this.request(
      "PUT",
      `/admin/applications/${applicationId}/status`,
      { action: "approve" },
    );
    if (!resp.ok) throw new Error(`Approve failed: ${resp.status} ${await resp.text()}`);
  }

  async rejectApplication(applicationId: string): Promise<void> {
    const resp = await this.request(
      "PUT",
      `/admin/applications/${applicationId}/status`,
      { action: "reject" },
    );
    if (!resp.ok) throw new Error(`Reject failed: ${resp.status} ${await resp.text()}`);
  }

  async createTargetableRole(
    roleName: string,
    validUntil: string,
    paymentLink?: string,
  ): Promise<void> {
    const resp = await this.request(
      "POST",
      "/admin/applications/targetable-roles",
      { role_name: roleName, valid_until: validUntil, payment_link: paymentLink ?? null },
    );
    if (!resp.ok) throw new Error(`Create targetable role failed: ${resp.status} ${await resp.text()}`);
  }

  async deleteTargetableRole(
    roleName: string,
    validUntil: string,
  ): Promise<void> {
    const resp = await this.request(
      "DELETE",
      `/admin/applications/targetable-roles?role_name=${encodeURIComponent(roleName)}&valid_until=${encodeURIComponent(validUntil)}`,
    );
    if (!resp.ok && resp.status !== 404) {
      throw new Error(`Delete targetable role failed: ${resp.status} ${await resp.text()}`);
    }
  }
}
