/*!
 * The store: one SQLite connection behind a mutex (S-1, DD-04).
 *
 * A single owner keeps every transaction serial, which is all one user
 * needs, and keeps the kernel synchronous. Reads queue behind writes.
 */

use core::fmt;
use std::sync::{Mutex, MutexGuard};

use gymos_engine::RepoError;
use rusqlite::Connection;

use crate::error::OpenError;
use crate::open::{self, StoreConfig};

/**
 * The SQLite implementation of the engine's
 * [`Store`](gymos_engine::Store).
 *
 * Open it once at startup and hand it to `GymOs`. Opening migrates the
 * schema, so the UI can't issue a command against an old one.
 */
pub struct SqliteStore {
    conn: Mutex<Connection>,
}

impl SqliteStore {
    /**
     * Opens (or creates) the database file named in `config`, then backs it
     * up and migrates it if this build has newer migrations.
     *
     * A database that already holds data is copied to
     * [`StoreConfig::backup_path`] before any pending migration runs. A new,
     * empty file has nothing to protect and gets no backup.
     *
     * # Errors
     *
     * - [`OpenError::NewerSchema`] or [`OpenError::UnknownSchema`] if the
     *   file's schema is not one this build knows. The file is not written.
     * - [`OpenError::InvalidBackupLabel`], [`OpenError::BackupExists`],
     *   [`OpenError::NonUtf8Path`] or [`OpenError::Backup`] if a needed
     *   backup can't be written. Nothing is migrated then.
     * - [`OpenError::Migration`] if a migration fails; it is rolled back.
     * - [`OpenError::ForeignKeysOff`], [`OpenError::WalOff`] or
     *   [`OpenError::Database`] if SQLite can't be opened or configured.
     */
    pub fn open(config: &StoreConfig) -> Result<Self, OpenError> {
        open::file(config).map(Self::from_connection)
    }

    /**
     * A private, empty, fully migrated database in memory, for tests and
     * previews. It disappears when the store is dropped.
     *
     * # Errors
     *
     * [`OpenError`] if SQLite can't create or migrate it.
     */
    pub fn in_memory() -> Result<Self, OpenError> {
        open::memory().map(Self::from_connection)
    }

    const fn from_connection(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    /**
     * Locks the connection and checks that no transaction is still open.
     *
     * A panic inside a transaction poisons the lock. The transaction guard
     * has rolled back by then, but the store does not take that on trust:
     * it recovers only if SQLite confirms no transaction is open
     * (autocommit mode), and otherwise reports [`RepoError::Backend`]
     * (S-1).
     *
     * The same check runs on every lock, because a `ROLLBACK` that failed
     * inside the guard's drop is ignored and would leave the connection
     * mid-transaction. One more `ROLLBACK` is tried before giving up.
     */
    pub(crate) fn lock(&self) -> Result<MutexGuard<'_, Connection>, RepoError> {
        let (guard, poisoned) = match self.conn.lock() {
            Ok(guard) => (guard, false),
            Err(poisoned) => (poisoned.into_inner(), true),
        };
        if !guard.is_autocommit() {
            /* Ignored: the autocommit check below decides. */
            let _rolled_back = guard.execute_batch("ROLLBACK");
            if !guard.is_autocommit() {
                return Err(RepoError::Backend(
                    "a transaction was left open and could not be rolled back".to_owned(),
                ));
            }
        }
        if poisoned {
            self.conn.clear_poison();
        }
        Ok(guard)
    }
}

impl fmt::Debug for SqliteStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SqliteStore").finish_non_exhaustive()
    }
}
