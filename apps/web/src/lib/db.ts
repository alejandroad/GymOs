import "server-only";
import { createDb, type Db } from "@gymos/db";

import { env } from "@/env";

let db: Db | undefined;

/**
 * The app's database client, created on first use so that importing a
 * module never needs a connection string (CI builds have none).
 */
export function getDb(): Db {
  db ??= createDb(env.DATABASE_URL);
  return db;
}
