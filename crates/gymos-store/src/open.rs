/*!
 * Opening a database: open → refuse an unknown schema → configure →
 * back up if a migration is pending → migrate (DD-04).
 *
 * The schema version of an existing file is first read on a read-only
 * connection that never checkpoints, so a database from a newer GymOs is
 * refused with its main file untouched, even if another process left
 * frames in its WAL.
 */

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::config::DbConfig;
use rusqlite::{Connection, OpenFlags};

use crate::backup;
use crate::error::{IntoOpen, OpenError, migration};
use crate::migrations;

/**
 * Where the database lives and how to label the backup taken before a
 * migration. The shell resolves both: the store reads no clock, environment
 * or path of its own (DD-03).
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreConfig {
    path: PathBuf,
    backup_label: String,
}

impl StoreConfig {
    /**
     * A database at `path`. `backup_label` names the backup taken if this
     * open migrates the schema; the shell passes a timestamp such as
     * `2026-11-19T07-30-00Z`. It is checked when the database is opened.
     */
    pub fn new(path: impl Into<PathBuf>, backup_label: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            backup_label: backup_label.into(),
        }
    }

    /** The database file. */
    pub fn path(&self) -> &Path {
        &self.path
    }

    /** The label for a pre-migration backup. */
    pub fn backup_label(&self) -> &str {
        &self.backup_label
    }

    /** Where a pre-migration backup is written: `<db>.<label>.bak`. */
    pub fn backup_path(&self) -> PathBuf {
        backup::path_for(&self.path, &self.backup_label)
    }
}

/** How long a write waits for a lock held elsewhere, such as a DB browser. */
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/** Opens, backs up if needed, and migrates a database file. */
pub(crate) fn file(config: &StoreConfig) -> Result<Connection, OpenError> {
    backup::check_label(&config.backup_label)?;
    if config.path.exists() {
        check_known(probe(&config.path)?)?;
    }
    let mut conn = Connection::open(&config.path).open_err()?;
    conn.busy_timeout(BUSY_TIMEOUT).open_err()?;
    /* Read again: the probe and this connection are not one transaction. */
    let found = schema_version(&conn)?;
    check_known(found)?;
    configure(&conn)?;
    enable_wal(&conn)?;
    if found < migrations::LATEST && !is_empty(&conn)? {
        backup::write(&conn, &config.backup_path())?;
    }
    migrations::all()
        .to_latest(&mut conn)
        .map_err(|e| migration(&e))?;
    Ok(conn)
}

/** Opens and migrates a private in-memory database. */
pub(crate) fn memory() -> Result<Connection, OpenError> {
    let mut conn = Connection::open_in_memory().open_err()?;
    configure(&conn)?;
    migrations::all()
        .to_latest(&mut conn)
        .map_err(|e| migration(&e))?;
    Ok(conn)
}

/**
 * Reads the schema version of an existing file without changing it.
 *
 * A read-write connection would checkpoint a WAL left by another process
 * into the main file when it closes, so a database from a newer GymOs
 * would be modified even though it is refused. The probe is read-only and
 * also asks SQLite not to checkpoint on close. (A hot rollback journal
 * can't be recovered read-only; that fails as `OpenError::Database`.)
 */
fn probe(path: &Path) -> Result<i64, OpenError> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .open_err()?;
    conn.busy_timeout(BUSY_TIMEOUT).open_err()?;
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, true)
        .open_err()?;
    schema_version(&conn)
}

/** The schema version, from `user_version`. Reading it writes nothing. */
fn schema_version(conn: &Connection) -> Result<i64, OpenError> {
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
        .open_err()
}

/** Refuses a schema this build doesn't know (DD-04). */
fn check_known(found: i64) -> Result<(), OpenError> {
    if found < 0 {
        return Err(OpenError::UnknownSchema(found));
    }
    if found > migrations::LATEST {
        return Err(OpenError::NewerSchema {
            found,
            supported: migrations::LATEST,
        });
    }
    Ok(())
}

/**
 * Connection settings (S-8). Foreign keys are checked after setting,
 * because SQLite ignores the pragma silently inside a transaction.
 */
fn configure(conn: &Connection) -> Result<(), OpenError> {
    conn.pragma_update(None, "foreign_keys", true).open_err()?;
    let enforced: bool = conn
        .pragma_query_value(None, "foreign_keys", |row| row.get(0))
        .open_err()?;
    if !enforced {
        return Err(OpenError::ForeignKeysOff);
    }
    conn.pragma_update(None, "synchronous", "FULL").open_err()?;
    Ok(())
}

/** Switches a file database to write-ahead logging and checks it took. */
fn enable_wal(conn: &Connection) -> Result<(), OpenError> {
    let mode: String = conn
        .pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))
        .open_err()?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(OpenError::WalOff(mode));
    }
    Ok(())
}

/** Whether the file holds no schema at all: a new database. */
fn is_empty(conn: &Connection) -> Result<bool, OpenError> {
    let objects: i64 = conn
        .query_row("SELECT count(*) FROM sqlite_schema", [], |row| row.get(0))
        .open_err()?;
    Ok(objects == 0)
}
