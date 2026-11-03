import { mkdirSync } from "node:fs";
import { dirname } from "node:path";
import { DatabaseSync } from "node:sqlite";

import { createDrizzle } from "../drizzle";
import { runMigrations } from "../migrate";
import { migrations } from "../migrations.generated";
import { addNote, countNotes, listNotes } from "../notes";
import type { StoreApi, StoreInfo } from "../protocol";
import { connectionFor } from "./connection";

/** Another process (usually a second GymOs server) has the database open. */
export class StoreLockedError extends Error {
  override name = "StoreLockedError";
}

export interface OpenStoreOptions {
  /** Absolute path of the database file. Its folder is created if missing. */
  dbPath: string;
  /** How long to wait for another process's lock before giving up. */
  busyTimeoutMs?: number;
}

export interface LocalStore {
  api: StoreApi;
  info: StoreInfo;
  /** Closes the connection, which checkpoints the WAL and releases the lock. */
  close(): void;
}

const SQLITE_BUSY = 5;
const SQLITE_LOCKED = 6;

/**
 * Opens the GymOs database file, takes it for this process and migrates it.
 *
 * - `locking_mode=EXCLUSIVE` keeps an OS file lock until the connection
 *   closes, so exactly one process (one writer) uses the file. The OS drops
 *   the lock when the process dies, even on a hard kill, so a lock can't go
 *   stale. A second opener gets a StoreLockedError.
 * - `journal_mode=WAL` with `synchronous=FULL`: a committed write survives a
 *   crash or power loss. Durability over speed.
 * - Migrations run before this returns, so nothing reads an old schema. A
 *   database from a newer app version or with a diverged history throws a
 *   MigrationError, and its contents are left as they were.
 */
export function openStore({ dbPath, busyTimeoutMs = 2000 }: OpenStoreOptions): LocalStore {
  const startedAt = performance.now();
  mkdirSync(dirname(dbPath), { recursive: true });
  const raw = new DatabaseSync(dbPath, { timeout: busyTimeoutMs });
  try {
    // Order matters: exclusive mode must be set before entering WAL, so the
    // WAL index lives in process memory instead of a shared-memory file.
    raw.exec("PRAGMA locking_mode = EXCLUSIVE");
    raw.prepare("PRAGMA journal_mode = WAL").get();
    raw.exec("PRAGMA synchronous = FULL");
    raw.exec("PRAGMA foreign_keys = ON");
    // Take the exclusive lock now rather than on the first write.
    raw.exec("BEGIN EXCLUSIVE; COMMIT");
  } catch (error) {
    raw.close();
    if (isBusy(error)) {
      throw new StoreLockedError(
        `Another process is using ${dbPath}. Is GymOs already running? Stop it and try again.`,
        { cause: error },
      );
    }
    throw error;
  }

  const conn = connectionFor(raw);
  try {
    runMigrations(conn, migrations);
  } catch (error) {
    raw.close();
    throw error;
  }

  const db = createDrizzle(conn);
  const info: StoreInfo = {
    sqliteVersion: String(conn.query("SELECT sqlite_version()")[0]?.[0]),
    dbPath,
    schemaVersion: migrations.length,
    initMs: performance.now() - startedAt,
  };

  const api: StoreApi = {
    info: async () => info,
    addNote: (note) => addNote(db, note),
    listNotes: (limit) => listNotes(db, limit),
    countNotes: () => countNotes(db),
    integrityCheck: async () =>
      conn
        .query("PRAGMA integrity_check")
        .map(([line]) => String(line))
        .join("; "),
  };

  return { api, info, close: () => raw.close() };
}

function isBusy(error: unknown): boolean {
  const code = (error as { errcode?: unknown } | null)?.errcode;
  return code === SQLITE_BUSY || code === SQLITE_LOCKED;
}
