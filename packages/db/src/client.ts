import { neon } from "@neondatabase/serverless";
import { sql } from "drizzle-orm";
import { drizzle } from "drizzle-orm/neon-http";

import * as schema from "./schema";

/**
 * Creates a Drizzle client over Neon's HTTP driver. Each query is one
 * stateless HTTPS request, which suits serverless and edge runtimes.
 */
export function createDb(databaseUrl: string) {
  return drizzle({ client: neon(databaseUrl), schema });
}

export type Db = ReturnType<typeof createDb>;

/** Round-trips a trivial query; throws if the database is unreachable. */
export async function ping(db: Db): Promise<void> {
  await db.execute(sql`select 1`);
}
