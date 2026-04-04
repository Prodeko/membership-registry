import {
  API_BASE_URL,
  ADMIN_EMAIL,
  ADMIN_PASSWORD,
} from "../helpers/constants";

/**
 * Admin API helper that authenticates via HTTP-level OAuth flow
 * (no browser needed) and makes authenticated requests.
 */
export class AdminApiHelper {
  private cookies: string = "";

  async authenticate(): Promise<void> {
    if (this.cookies) return;

    // Step 1: Hit the login endpoint to get the Keycloak redirect URL
    const loginResp = await fetch(`${API_BASE_URL}/auth/login`, {
      redirect: "manual",
    });
    const keycloakUrl = loginResp.headers.get("location")!;

    // Capture any cookies from the login redirect (oauth_state)
    const loginCookies = loginResp.headers.getSetCookie?.() ?? [];

    // Step 2: GET the Keycloak login page to get the form action URL
    const kcPageResp = await fetch(keycloakUrl);
    const kcPageHtml = await kcPageResp.text();
    const kcCookies = kcPageResp.headers.getSetCookie?.() ?? [];

    // Extract form action URL from Keycloak HTML
    const formActionMatch = kcPageHtml.match(
      /action="([^"]+)"/,
    );
    if (!formActionMatch) throw new Error("Could not find Keycloak form action");
    const formAction = formActionMatch[1].replace(/&amp;/g, "&");

    // Step 3: POST username to Keycloak (first step of two-step login)
    const allKcCookies = [...kcCookies];

    const usernameResp = await fetch(formAction, {
      method: "POST",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
        Cookie: allKcCookies.map((c) => c.split(";")[0]).join("; "),
      },
      body: new URLSearchParams({ username: ADMIN_EMAIL }).toString(),
      redirect: "manual",
    });

    // Keycloak may redirect or return the password page directly
    let passwordPageHtml: string;
    const usernameRespCookies = usernameResp.headers.getSetCookie?.() ?? [];
    allKcCookies.push(...usernameRespCookies);
    const kcCookieHeader = allKcCookies.map((c) => c.split(";")[0]).join("; ");

    if (usernameResp.status >= 300 && usernameResp.status < 400) {
      const redirectTo = usernameResp.headers.get("location")!;
      const pwPageResp = await fetch(redirectTo, {
        headers: { Cookie: kcCookieHeader },
      });
      passwordPageHtml = await pwPageResp.text();
      const pwCookies = pwPageResp.headers.getSetCookie?.() ?? [];
      allKcCookies.push(...pwCookies);
    } else {
      passwordPageHtml = await usernameResp.text();
    }

    // Step 4: Extract password form action and POST password
    const pwFormMatch = passwordPageHtml.match(/action="([^"]+)"/);
    if (!pwFormMatch) throw new Error("Could not find password form action");
    const pwFormAction = pwFormMatch[1].replace(/&amp;/g, "&");

    const pwCookieHeader = allKcCookies.map((c) => c.split(";")[0]).join("; ");
    const kcLoginResp = await fetch(pwFormAction, {
      method: "POST",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
        Cookie: pwCookieHeader,
      },
      body: new URLSearchParams({ password: ADMIN_PASSWORD }).toString(),
      redirect: "manual",
    });

    // Step 5: Follow redirects back to our callback
    // Keycloak redirects to our /auth/callback with code + state
    const redirectUrl = kcLoginResp.headers.get("location")!;
    const callbackResp = await fetch(redirectUrl, {
      redirect: "manual",
      headers: {
        Cookie: loginCookies.map((c) => c.split(";")[0]).join("; "),
      },
    });

    // Extract session cookies from the callback response
    const sessionCookies = callbackResp.headers.getSetCookie?.() ?? [];
    this.cookies = sessionCookies.map((c) => c.split(";")[0]).join("; ");
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
