/*!
 * The backup taken before a migration (DD-04, S-6).
 *
 * `VACUUM INTO` writes a consistent copy through SQLite itself, so pages
 * still in the `-wal` file are included, which a plain file copy could
 * miss. Backups are never overwritten and never pruned: deleting user data
 * is not the store's call.
 */

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::OpenError;

/**
 * Checks that `label` is safe to put in a file name: non-empty, ASCII
 * letters, digits, `-`, `_` and `.`, and not starting with `.`. That rules
 * out path separators and `..`.
 */
pub(crate) fn check_label(label: &str) -> Result<(), OpenError> {
    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.');
    if label.is_empty() || label.starts_with('.') || !label.chars().all(allowed) {
        return Err(OpenError::InvalidBackupLabel);
    }
    Ok(())
}

/** `<db>.<label>.bak`, next to the database. */
pub(crate) fn path_for(database: &Path, label: &str) -> PathBuf {
    let mut name = OsString::from(database.as_os_str());
    name.push(".");
    name.push(label);
    name.push(".bak");
    PathBuf::from(name)
}

/**
 * Writes a backup of the open database to `target`.
 *
 * # Errors
 *
 * [`OpenError::BackupExists`] if `target` exists, [`OpenError::NonUtf8Path`]
 * if it isn't UTF-8, or [`OpenError::Backup`] if SQLite fails.
 */
pub(crate) fn write(conn: &Connection, target: &Path) -> Result<(), OpenError> {
    if target.exists() {
        return Err(OpenError::BackupExists(target.to_owned()));
    }
    let utf8 = target
        .to_str()
        .ok_or_else(|| OpenError::NonUtf8Path(target.to_owned()))?;
    conn.execute("VACUUM INTO ?1", [utf8])
        .map_err(|e| OpenError::Backup(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_timestamp_like_labels() {
        assert_eq!(check_label("2026-11-19T07-30-00Z"), Ok(()));
        assert_eq!(check_label("v1_to_v2.1"), Ok(()));
    }

    #[test]
    fn rejects_labels_that_could_escape_the_directory() {
        for label in ["", ".", "..", "../x", "a/b", "a\\b", ".hidden", "a b", "ü"] {
            assert_eq!(
                check_label(label),
                Err(OpenError::InvalidBackupLabel),
                "{label:?}"
            );
        }
    }

    #[test]
    fn the_backup_sits_next_to_the_database() {
        assert_eq!(
            path_for(Path::new("/data/gymos.db"), "2026-11-19"),
            PathBuf::from("/data/gymos.db.2026-11-19.bak")
        );
    }
}
