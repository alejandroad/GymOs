import type { SqlConnection } from "./sql";

/** One forward-only migration: drizzle-kit SQL with statement breakpoints. */
export interface Migration {
  tag: string;
  sql: string;
}

export class MigrationError extends Error {
  override name = "MigrationError";
}

const BREAKPOINT = "--> statement-breakpoint";

/**
 * Brings the database up to date with `migrations`, at startup, before the
 * UI reads anything.
 *
 * - Migrations apply in list order, each in its own transaction together
 *   with its row in `__migrations`, so a failure leaves no partial schema.
 * - Forward-only: there are no down migrations. Applied history must be a
 *   prefix of `migrations`; a database from a newer app version, or one
 *   whose history diverges, is refused rather than guessed at.
 * - Idempotent: running it again applies nothing.
 *
 * Returns the tags applied by this run.
 */
export function runMigrations(conn: SqlConnection, migrations: readonly Migration[]): string[] {
  conn.exec(`CREATE TABLE IF NOT EXISTS __migrations (
    idx INTEGER PRIMARY KEY,
    tag TEXT NOT NULL UNIQUE,
    applied_at INTEGER NOT NULL
  )`);

  const applied = conn.query("SELECT idx, tag FROM __migrations ORDER BY idx");
  if (applied.length > migrations.length) {
    throw new MigrationError(
      `The database has ${applied.length} migrations but this app knows ${migrations.length}. ` +
        "It was written by a newer version of GymOs.",
    );
  }
  applied.forEach(([idx, tag], i) => {
    if (idx !== i || tag !== migrations[i]?.tag) {
      throw new MigrationError(
        `Migration history diverges at position ${i}: database has "${String(tag)}", app expects "${migrations[i]?.tag}".`,
      );
    }
  });

  const pending = migrations.slice(applied.length);
  for (const [offset, migration] of pending.entries()) {
    const idx = applied.length + offset;
    conn.exec("BEGIN IMMEDIATE");
    try {
      for (const statement of migration.sql.split(BREAKPOINT)) {
        if (statement.trim()) conn.exec(statement);
      }
      conn.query("INSERT INTO __migrations (idx, tag, applied_at) VALUES (?, ?, ?)", [
        idx,
        migration.tag,
        Date.now(),
      ]);
      conn.exec("COMMIT");
    } catch (error) {
      conn.exec("ROLLBACK");
      throw new MigrationError(`Migration "${migration.tag}" failed and was rolled back`, { cause: error });
    }
  }
  return pending.map((migration) => migration.tag);
}
