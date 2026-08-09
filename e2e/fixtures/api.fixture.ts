import fs from "fs";
import { API_BASE_URL } from "../helpers/constants";

const ADMIN_AUTH_FILE = ".auth/admin.json";

/**
 * Admin API helper that loads auth cookies from the storageState file
 * saved during the auth setup phase.
 */
export class AdminApiHelper {
  private cookies: string = "";

  private loadCookies(): void {
    if (this.cookies) return;
    const state = JSON.parse(fs.readFileSync(ADMIN_AUTH_FILE, "utf-8"));
    this.cookies = state.cookies
      .map((c: { name: string; value: string }) => `${c.name}=${c.value}`)
      .join("; ");
  }

  private async request(
    method: string,
    path: string,
    body?: unknown,
  ): Promise<Response> {
    this.loadCookies();
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
      throw new Error(
        `Create role failed: ${resp.status} ${await resp.text()}`,
      );
    }
  }

  async updateRole(
    roleName: string,
    data: {
      renewable: boolean;
      renewal_payment_link?: string | null;
      renewal_period_months?: number | null;
      renewal_notification_days?: number[];
      renewal_window_days?: number;
      grace_period_days?: number;
      renewal_prompts?: {
        locale: string;
        title: string;
        body: string;
        button_label: string;
      }[];
    },
  ): Promise<void> {
    const resp = await this.request(
      "PUT",
      `/admin/roles/${encodeURIComponent(roleName)}`,
      {
        renewable: data.renewable,
        renewal_payment_link: data.renewal_payment_link ?? null,
        renewal_period_months: data.renewal_period_months ?? null,
        renewal_notification_days: data.renewal_notification_days ?? [30, 7, 1],
        renewal_window_days: data.renewal_window_days ?? 30,
        grace_period_days: data.grace_period_days ?? 0,
        renewal_prompts: data.renewal_prompts ?? [],
      },
    );
    if (!resp.ok)
      throw new Error(
        `Update role failed: ${resp.status} ${await resp.text()}`,
      );
  }

  async updateMember(
    userId: string,
    data: {
      first_name: string;
      last_name: string;
      home_municipality?: string | null;
      email_notifications: boolean;
      language: string;
    },
  ): Promise<void> {
    const resp = await this.request("PUT", `/admin/members/${userId}`, data);
    if (!resp.ok)
      throw new Error(
        `Update member failed: ${resp.status} ${await resp.text()}`,
      );
  }

  async assignRole(
    userId: string,
    roleName: string,
    validFrom: string,
    validUntil?: string,
  ): Promise<void> {
    const resp = await this.request("POST", `/admin/members/${userId}/roles`, {
      role_name: roleName,
      valid_from: validFrom,
      valid_until: validUntil ?? null,
    });
    if (!resp.ok)
      throw new Error(
        `Assign role failed: ${resp.status} ${await resp.text()}`,
      );
  }

  async approveApplication(applicationId: string): Promise<void> {
    const resp = await this.request(
      "PUT",
      `/admin/applications/${applicationId}/status`,
      { action: "approve" },
    );
    if (!resp.ok)
      throw new Error(`Approve failed: ${resp.status} ${await resp.text()}`);
  }

  async rejectApplication(applicationId: string): Promise<void> {
    const resp = await this.request(
      "PUT",
      `/admin/applications/${applicationId}/status`,
      { action: "reject" },
    );
    if (!resp.ok)
      throw new Error(`Reject failed: ${resp.status} ${await resp.text()}`);
  }

  async createTargetableRole(
    roleName: string,
    validUntil: string,
    paymentLink?: string,
    opts?: {
      approved_email_template?: string;
      rejected_email_template?: string;
      form_attributes?: string[];
    },
  ): Promise<void> {
    const resp = await this.request(
      "POST",
      "/admin/applications/targetable-roles",
      {
        role_name: roleName,
        valid_until: validUntil,
        payment_link: paymentLink ?? null,
        approved_email_template: opts?.approved_email_template ?? null,
        rejected_email_template: opts?.rejected_email_template ?? null,
        form_attributes: opts?.form_attributes ?? [],
      },
    );
    if (!resp.ok)
      throw new Error(
        `Create targetable role failed: ${resp.status} ${await resp.text()}`,
      );
  }

  async createEmailTemplate(name: string): Promise<void> {
    const resp = await this.request("POST", "/admin/email-templates", { name });
    if (!resp.ok && resp.status !== 409) {
      throw new Error(
        `Create email template failed: ${resp.status} ${await resp.text()}`,
      );
    }
  }

  async upsertEmailTranslation(
    templateName: string,
    locale: string,
    subject: string,
    bodyHtml: string,
  ): Promise<void> {
    const resp = await this.request(
      "PUT",
      `/admin/email-templates/${encodeURIComponent(templateName)}/translations/${locale}`,
      { subject, body_html: bodyHtml },
    );
    if (!resp.ok)
      throw new Error(
        `Upsert translation failed: ${resp.status} ${await resp.text()}`,
      );
  }

  async deleteEmailTemplate(name: string): Promise<void> {
    const resp = await this.request(
      "DELETE",
      `/admin/email-templates/${encodeURIComponent(name)}`,
    );
    if (!resp.ok && resp.status !== 404) {
      throw new Error(
        `Delete email template failed: ${resp.status} ${await resp.text()}`,
      );
    }
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
      throw new Error(
        `Delete targetable role failed: ${resp.status} ${await resp.text()}`,
      );
    }
  }

  async createAttributeDefinition(input: {
    name: string;
    description?: string | null;
    allowed_values?: string[] | null;
    default_value?: string | null;
    sync_to_keycloak?: boolean;
    editable_by?: "admin" | "user" | "both";
  }): Promise<void> {
    const resp = await this.request("POST", "/admin/attributes", {
      name: input.name,
      description: input.description ?? null,
      allowed_values: input.allowed_values ?? null,
      default_value: input.default_value ?? null,
      // Default to internal (no SSO) so tests don't need a Keycloak mapper
      // round trip.
      sync_to_keycloak: input.sync_to_keycloak ?? false,
      editable_by: input.editable_by ?? "admin",
    });
    if (!resp.ok && resp.status !== 409) {
      throw new Error(
        `Create attribute failed: ${resp.status} ${await resp.text()}`,
      );
    }
  }

  async deleteAttributeDefinition(name: string): Promise<void> {
    const resp = await this.request(
      "DELETE",
      `/admin/attributes/${encodeURIComponent(name)}`,
    );
    if (!resp.ok && resp.status !== 404) {
      throw new Error(
        `Delete attribute failed: ${resp.status} ${await resp.text()}`,
      );
    }
  }
}
