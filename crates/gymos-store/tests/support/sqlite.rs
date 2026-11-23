/*!
 * Shared helpers for the store's integration tests, included by each test
 * file with `#[path]`.
 */

#![expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test setup helpers: an unexpected error is a failed test"
)]
#![allow(
    dead_code,
    reason = "each test file uses a different subset of these helpers, so #[expect] would be unfulfilled in some"
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use gymos_engine::{EngineError, Store};
use gymos_store::{SqliteRead, SqliteStore, SqliteWrite, StoreConfig};
use gymos_testkit::TestGym;
use tempfile::TempDir;

/**
 * A cloneable handle on one [`SqliteStore`], so a [`TestGym`] can keep a
 * second handle for seeding and inspection (S-9). It only delegates.
 */
#[derive(Debug, Clone)]
pub(crate) struct Shared(pub(crate) Arc<SqliteStore>);

impl Store for Shared {
    type Read<'t> = SqliteRead<'t>;
    type Write<'t> = SqliteWrite<'t>;

    fn read<R>(
        &self,
        f: impl FnOnce(&Self::Read<'_>) -> Result<R, EngineError>,
    ) -> Result<R, EngineError> {
        self.0.read(f)
    }

    fn write<R>(
        &self,
        f: impl FnOnce(&mut Self::Write<'_>) -> Result<R, EngineError>,
    ) -> Result<R, EngineError> {
        self.0.write(f)
    }
}

/** A fresh in-memory store. */
pub(crate) fn memory() -> SqliteStore {
    SqliteStore::in_memory().unwrap()
}

/** A temporary directory that holds one database file. */
pub(crate) struct TempDb {
    /** Deleted when dropped. */
    pub(crate) dir: TempDir,
    /** The database file inside `dir`. */
    pub(crate) path: PathBuf,
}

impl TempDb {
    /** An empty directory; the database file doesn't exist yet. */
    pub(crate) fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gymos.db");
        Self { dir, path }
    }

    /** The config for this database, with `label` for any backup. */
    pub(crate) fn config(&self, label: &str) -> StoreConfig {
        StoreConfig::new(&self.path, label)
    }

    /** Opens the database with a fixed backup label. */
    pub(crate) fn open(&self) -> SqliteStore {
        SqliteStore::open(&self.config("test")).unwrap()
    }

    /** A raw connection, for seeding or inspecting with plain SQL. */
    pub(crate) fn raw(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(&self.path).unwrap()
    }
}

/** A gym over a fresh in-memory SQLite store. */
pub(crate) fn memory_gym() -> TestGym<Shared> {
    TestGym::with_store(Shared(Arc::new(memory())))
}

/** A gym over a fresh SQLite file in `db`. */
pub(crate) fn file_gym(db: &TempDb) -> TestGym<Shared> {
    TestGym::with_store(Shared(Arc::new(db.open())))
}

/** The schema version recorded in a database file. */
pub(crate) fn user_version(path: &Path) -> i64 {
    rusqlite::Connection::open(path)
        .unwrap()
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap()
}

/** The number of rows in `table` (a literal from the test). */
pub(crate) fn count(conn: &rusqlite::Connection, table: &str) -> i64 {
    let sql = match table {
        "exercises" => "SELECT count(*) FROM exercises",
        "sessions" => "SELECT count(*) FROM sessions",
        "session_entries" => "SELECT count(*) FROM session_entries",
        "sets" => "SELECT count(*) FROM sets",
        "applied_commands" => "SELECT count(*) FROM applied_commands",
        other => panic!("unknown table {other}"),
    };
    conn.query_row(sql, [], |row| row.get(0)).unwrap()
}

/** Every table the schema defines. */
pub(crate) const TABLES: [&str; 5] = [
    "exercises",
    "sessions",
    "session_entries",
    "sets",
    "applied_commands",
];
