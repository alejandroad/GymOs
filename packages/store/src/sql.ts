/** A value SQLite binds or returns. */
export type SqlValue = null | number | bigint | string | Uint8Array;

/**
 * The one synchronous SQL primitive the store builds on. The migration
 * runner and the Drizzle driver both use it; `node/connection.ts`
 * implements it over `node:sqlite`.
 */
export interface SqlConnection {
  /** Runs one statement and returns its rows as positional arrays. */
  query(sql: string, params?: readonly SqlValue[]): SqlValue[][];
  /** Runs one or more statements, discarding any rows. */
  exec(sql: string): void;
}
