import { getPool } from "../helpers/db-client";

export class DatabaseHelper {
  private pool = getPool();

  async query<T = Record<string, unknown>>(
    sql: string,
    params: unknown[] = [],
  ): Promise<T[]> {
    const result = await this.pool.query(sql, params);
    return result.rows as T[];
  }

  async cleanupTestUser(email: string): Promise<void> {
    // Delete in FK-safe order
    await this.query(
      `DELETE FROM audit_log WHERE actor_user_id IN (SELECT user_id FROM member WHERE email = $1)`,
      [email],
    );
    await this.query(
      `DELETE FROM rolerenewal WHERE user_id IN (SELECT user_id FROM member WHERE email = $1)`,
      [email],
    );
    await this.query(
      `DELETE FROM application WHERE user_id IN (SELECT user_id FROM member WHERE email = $1)`,
      [email],
    );
    await this.query(
      `DELETE FROM rolemember WHERE user_id IN (SELECT user_id FROM member WHERE email = $1)`,
      [email],
    );
    await this.query(`DELETE FROM member WHERE email = $1`, [email]);
  }

  async cleanupTestRole(roleName: string): Promise<void> {
    await this.query(
      `DELETE FROM applicationtargetablerole WHERE role_name = $1`,
      [roleName],
    );
    await this.query(
      `DELETE FROM rolerenewal WHERE role_name = $1`,
      [roleName],
    );
    await this.query(`DELETE FROM application WHERE role_name = $1`, [
      roleName,
    ]);
    await this.query(`DELETE FROM rolemember WHERE role_name = $1`, [roleName]);
    await this.query(`DELETE FROM role WHERE name = $1`, [roleName]);
  }

  async getApplicationByUserEmail(
    email: string,
  ): Promise<Record<string, unknown> | null> {
    const rows = await this.query(
      `SELECT a.* FROM application a JOIN member m ON a.user_id = m.user_id WHERE m.email = $1 ORDER BY a.timestamp DESC LIMIT 1`,
      [email],
    );
    return rows[0] ?? null;
  }

  async getMemberByEmail(
    email: string,
  ): Promise<Record<string, unknown> | null> {
    const rows = await this.query(
      `SELECT * FROM member WHERE email = $1`,
      [email],
    );
    return rows[0] ?? null;
  }
}
