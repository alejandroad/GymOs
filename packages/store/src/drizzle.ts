import { drizzle } from "drizzle-orm/sqlite-proxy";

import * as schema from "./schema";
import type { SqlConnection, SqlValue } from "./sql";

/**
 * Drizzle over the store's own SQLite connection, through the sqlite-proxy
 * driver: Drizzle builds SQL and maps rows, and the callback runs it in
 * process. Drizzle 0.45 has no `node:sqlite` driver; the proxy driver is its
 * documented route for custom SQLite connections.
 */
export function createDrizzle(conn: SqlConnection) {
  return drizzle(
    async (sql, params, method) => {
      const rows = conn.query(sql, params as SqlValue[]);
      // "get" expects the single row itself (or undefined), not a list.
      return { rows: method === "get" ? (rows[0] as unknown as unknown[]) : rows };
    },
    { schema },
  );
}

export type StoreDb = ReturnType<typeof createDrizzle>;
