const KEYCLOAK_URL = process.env.KEYCLOAK_URL ?? "http://127.0.0.1:8180";
const KEYCLOAK_REALM = process.env.KEYCLOAK_REALM ?? "membership-registry";
const CLIENT_ID =
  process.env.KEYCLOAK_ADMIN_CLIENT_ID ?? "membership-registry-m2m";
const CLIENT_SECRET =
  process.env.KEYCLOAK_ADMIN_CLIENT_SECRET ??
  "dev-secret-membership-registry-m2m";

let cachedToken: { token: string; expiresAt: number } | null = null;

async function getServiceToken(): Promise<string> {
  if (cachedToken && Date.now() < cachedToken.expiresAt - 30_000) {
    return cachedToken.token;
  }

  const resp = await fetch(
    `${KEYCLOAK_URL}/realms/${KEYCLOAK_REALM}/protocol/openid-connect/token`,
    {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({
        grant_type: "client_credentials",
        client_id: CLIENT_ID,
        client_secret: CLIENT_SECRET,
      }),
    },
  );

  if (!resp.ok) {
    throw new Error(
      `Keycloak token request failed: ${resp.status} ${await resp.text()}`,
    );
  }

  const data = (await resp.json()) as {
    access_token: string;
    expires_in: number;
  };
  cachedToken = {
    token: data.access_token,
    expiresAt: Date.now() + data.expires_in * 1000,
  };
  return cachedToken.token;
}

export async function getUserRealmRoles(
  keycloakUserId: string,
): Promise<string[]> {
  const token = await getServiceToken();
  const resp = await fetch(
    `${KEYCLOAK_URL}/admin/realms/${KEYCLOAK_REALM}/users/${keycloakUserId}/role-mappings/realm`,
    {
      headers: { Authorization: `Bearer ${token}` },
    },
  );

  if (!resp.ok) {
    throw new Error(
      `Failed to get user roles: ${resp.status} ${await resp.text()}`,
    );
  }

  const roles = (await resp.json()) as { name: string }[];
  return roles.map((r) => r.name);
}

export async function findKeycloakUserByEmail(
  email: string,
): Promise<string | null> {
  const token = await getServiceToken();
  const resp = await fetch(
    `${KEYCLOAK_URL}/admin/realms/${KEYCLOAK_REALM}/users?email=${encodeURIComponent(email)}&exact=true`,
    {
      headers: { Authorization: `Bearer ${token}` },
    },
  );

  if (!resp.ok) {
    throw new Error(
      `Failed to find KC user: ${resp.status} ${await resp.text()}`,
    );
  }

  const users = (await resp.json()) as { id: string }[];
  return users[0]?.id ?? null;
}
