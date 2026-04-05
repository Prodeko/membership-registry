import { test as base } from "@playwright/test";
import { DatabaseHelper } from "./db.fixture";
import { AdminApiHelper } from "./api.fixture";
import { closePool } from "../helpers/db-client";
import { getWorkerTestUser, getWorkerTestRole } from "../helpers/constants";

type TestFixtures = {
  db: DatabaseHelper;
  adminApi: AdminApiHelper;
};

type WorkerFixtures = {
  testUser: { email: string; password: string };
  testRole: { name: string; displayName: string };
};

export const test = base.extend<TestFixtures, WorkerFixtures>({
  db: async ({}, use) => {
    const db = new DatabaseHelper();
    await use(db);
  },

  adminApi: async ({}, use) => {
    const api = new AdminApiHelper();
    await use(api);
  },

  testUser: [
    async ({}, use, workerInfo) => {
      await use(getWorkerTestUser(workerInfo.parallelIndex));
    },
    { scope: "worker" },
  ],

  testRole: [
    async ({}, use, workerInfo) => {
      await use(getWorkerTestRole(workerInfo.parallelIndex));
    },
    { scope: "worker" },
  ],
});

test.afterAll(async () => {
  await closePool();
});

export { expect } from "@playwright/test";
