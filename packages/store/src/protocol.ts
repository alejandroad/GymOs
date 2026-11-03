import type { SpikeNote } from "./schema";

export type { SpikeNote };

/** Facts about the on-disk database, reported by the local server. */
export interface StoreInfo {
  sqliteVersion: string;
  /** Absolute path of the database file. */
  dbPath: string;
  /** Number of migrations applied to the database, i.e. its schema version. */
  schemaVersion: number;
  /** Milliseconds from opening the file until it was locked and migrated. */
  initMs: number;
}

/**
 * The store's RPC surface, served by the local server at POST /api/rpc. The
 * UI never touches SQLite. A type alias so it satisfies rpc's Api constraint.
 */
export type StoreApi = {
  info(): Promise<StoreInfo>;
  /** Idempotent: the caller generates the note's UUIDv7. */
  addNote(note: SpikeNote): Promise<SpikeNote>;
  listNotes(limit: number): Promise<SpikeNote[]>;
  countNotes(): Promise<number>;
  /** Result of PRAGMA integrity_check: "ok" when the database is sound. */
  integrityCheck(): Promise<string>;
};

export interface SerializedError {
  name: string;
  message: string;
}

export function serializeError(error: unknown): SerializedError {
  if (error instanceof Error) return { name: error.name, message: error.message };
  return { name: "Error", message: String(error) };
}
