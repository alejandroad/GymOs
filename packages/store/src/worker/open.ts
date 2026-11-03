import sqlite3InitModule, {
  type Database,
  type SAHPoolUtil,
  type Sqlite3Static,
} from "@sqlite.org/sqlite-wasm";

/** Name of the OPFS SyncAccessHandle pool VFS (needs no COOP/COEP headers). */
export const VFS_NAME = "opfs-sahpool";
/** Database path inside the pool. Pool paths must be absolute. */
export const DB_FILE = "/gymos.sqlite3";

type PoolOptions = Parameters<Sqlite3Static["installOpfsSAHPoolVfs"]>[0] & {
  // Supported at runtime since SQLite 3.47 but missing from the bundled
  // typings (3.54.1-build1).
  forceReinitIfPreviouslyFailed?: boolean;
};

const INSTALL_ATTEMPTS = 5;
const INSTALL_RETRY_MS = 200;

export interface OpenedDatabase {
  db: Database;
  vfs: string;
  sqliteVersion: string;
}

/**
 * Loads SQLite-WASM and opens the GymOs database on the opfs-sahpool VFS.
 *
 * Must run in a dedicated worker, and only in the tab that holds the writer
 * lock: the pool takes exclusive SyncAccessHandles on its files.
 */
export async function openDatabase(): Promise<OpenedDatabase> {
  const sqlite3 = await sqlite3InitModule();
  const pool = await installPool(sqlite3);
  const db = new pool.OpfsSAHPoolDb(DB_FILE);
  db.exec("PRAGMA foreign_keys = ON");
  return { db, vfs: VFS_NAME, sqliteVersion: sqlite3.version.libVersion };
}

/**
 * Installs the pool, retrying briefly. A tab that just closed can hold its
 * access handles for a moment after the writer lock passes to us.
 *
 * preserveOnInitFailure keeps the stored databases when an attempt fails.
 * Without it, SQLite wipes the pool's storage on an init failure.
 */
async function installPool(sqlite3: Sqlite3Static): Promise<SAHPoolUtil> {
  let lastError: unknown;
  for (let attempt = 1; attempt <= INSTALL_ATTEMPTS; attempt++) {
    try {
      const options: PoolOptions = {
        name: VFS_NAME,
        preserveOnInitFailure: true,
        // Retrying a cached failure needs this flag. It is safe here only
        // because preserveOnInitFailure keeps the pool's data intact.
        forceReinitIfPreviouslyFailed: attempt > 1,
      };
      return await sqlite3.installOpfsSAHPoolVfs(options);
    } catch (error) {
      lastError = error;
      await new Promise((resolve) => setTimeout(resolve, INSTALL_RETRY_MS * attempt));
    }
  }
  throw lastError;
}
