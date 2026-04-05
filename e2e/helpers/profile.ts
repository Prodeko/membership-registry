import { expect } from "@playwright/test";
import type { DatabaseHelper } from "../fixtures/db.fixture";
import type { AdminApiHelper } from "../fixtures/api.fixture";

/**
 * Fetches the member created on first login and marks their profile as
 * complete (municipality set, policies accepted) via the admin API, so
 * subsequent flows skip the signup fields. Returns the member's user_id.
 */
export async function completeMemberProfile(
  db: DatabaseHelper,
  adminApi: AdminApiHelper,
  email: string,
): Promise<string> {
  const member = await db.getMemberByEmail(email);
  expect(member, `Expected member with email ${email} to exist`).not.toBeNull();
  const userId = member!.user_id as string;
  await adminApi.updateMember(userId, {
    first_name: member!.first_name as string,
    last_name: member!.last_name as string,
    home_municipality: "Helsinki",
    has_accepted_policies: true,
    email_notifications: false,
    language: "en",
  });
  return userId;
}
