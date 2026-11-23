/*!
 * Store errors (DD-07).
 *
 * This is the only module that names `rusqlite::Error`. Every driver error
 * becomes a [`RepoError`] here, or an [`OpenError`] while opening, so no
 * SQLite type reaches the engine or the shell.
 */

use std::path::PathBuf;

use gymos_engine::RepoError;
use thiserror::Error;

/** Why a database could not be opened. */
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OpenError {
    /**
     * The database was written by a newer GymOs. It is left untouched
     * (DD-04): the user needs a newer build, not a downgrade.
     */
    #[error("the database uses schema version {found}, but this build supports up to {supported}")]
    NewerSchema {
        /** The schema version stored in the file. */
        found: i64,
        /** The newest schema version this build knows. */
        supported: i64,
    },
    /** The schema version stored in the file is not one GymOs ever writes. */
    #[error("the database has an unknown schema version {0}")]
    UnknownSchema(i64),
    /**
     * The backup label would not make a plain file name. Labels may use
     * ASCII letters, digits, `-`, `_` and `.` (not leading).
     */
    #[error("the backup label is not a valid file name part")]
    InvalidBackupLabel,
    /** The database path is not valid UTF-8, which SQLite needs for a backup. */
    #[error("the database path is not valid UTF-8: {0}")]
    NonUtf8Path(PathBuf),
    /**
     * A backup with this label already exists. Backups are never
     * overwritten (S-6), so the shell must pass a new label.
     */
    #[error("a backup already exists at {0}")]
    BackupExists(PathBuf),
    /** The backup could not be written; nothing was migrated. */
    #[error("the backup before migrating failed: {0}")]
    Backup(String),
    /** A migration failed and was rolled back. */
    #[error("migrating the database failed: {0}")]
    Migration(String),
    /** SQLite refused to enable foreign-key enforcement. */
    #[error("SQLite did not enable foreign keys")]
    ForeignKeysOff,
    /** SQLite refused to switch the file to write-ahead logging. */
    #[error("SQLite did not enable WAL mode (it reported {0:?})")]
    WalOff(String),
    /** The database could not be opened or configured. */
    #[error("opening the database failed: {0}")]
    Database(String),
}

/*
 * Conversions into `OpenError` are crate-private functions, not `From`
 * impls: a public `impl From<rusqlite::Error>` would put the driver's type
 * into this crate's public API (PROMPT-3 §6.1).
 */

/** A driver error while opening or configuring the database. */
pub(crate) fn database(error: &rusqlite::Error) -> OpenError {
    OpenError::Database(error.to_string())
}

/** A migration that failed and was rolled back. */
pub(crate) fn migration(error: &rusqlite_migration::Error) -> OpenError {
    OpenError::Migration(error.to_string())
}

/** `?`-friendly conversion of driver results while opening. */
pub(crate) trait IntoOpen<T> {
    /** Converts the error with [`database`]. */
    fn open_err(self) -> Result<T, OpenError>;
}

impl<T> IntoOpen<T> for rusqlite::Result<T> {
    fn open_err(self) -> Result<T, OpenError> {
        self.map_err(|error| database(&error))
    }
}

/**
 * Converts a driver error. A value SQLite could not convert to the Rust
 * type asked for (wrong type, out of range, text that isn't UTF-8) means
 * the stored data is wrong, so it is `Corrupt`; anything else is a
 * `Backend` failure.
 */
pub(crate) fn repo(error: &rusqlite::Error) -> RepoError {
    let corrupt = matches!(
        *error,
        rusqlite::Error::InvalidColumnType(..)
            | rusqlite::Error::FromSqlConversionFailure(..)
            | rusqlite::Error::IntegralValueOutOfRange(..)
            | rusqlite::Error::Utf8Error(..)
    );
    if corrupt {
        RepoError::Corrupt(error.to_string())
    } else {
        RepoError::Backend(error.to_string())
    }
}

/** `?`-friendly conversion of driver results. */
pub(crate) trait IntoRepo<T> {
    /** Converts the error with [`repo`]. */
    fn repo(self) -> Result<T, RepoError>;
}

impl<T> IntoRepo<T> for rusqlite::Result<T> {
    fn repo(self) -> Result<T, RepoError> {
        self.map_err(|error| repo(&error))
    }
}

/**
 * A stored value the domain rejected (S-5). The detail names the table, the
 * row and the field, never the value, so it can be logged without leaking
 * user data.
 */
pub(crate) fn corrupt(table: &str, id: &dyn core::fmt::Display, field: &str) -> RepoError {
    RepoError::Corrupt(format!("{table} {id}: invalid {field}"))
}
