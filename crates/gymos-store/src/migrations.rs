/*!
 * The embedded schema migrations (DD-04).
 *
 * Forward-only: a released migration is never edited, and there are no
 * down migrations. `rusqlite_migration` tracks the schema version in
 * SQLite's `user_version`, so version *n* means the first *n* migrations
 * have run.
 */

use rusqlite_migration::{M, Migrations};

/** Every migration, in order. Append only. */
const MIGRATIONS: &[M<'static>] = &[M::up(include_str!("../migrations/0001_initial.sql"))];

/** The migration list `rusqlite_migration` runs. */
pub(crate) const fn all() -> Migrations<'static> {
    Migrations::from_slice(MIGRATIONS)
}

/**
 * The newest schema version this build knows: the number of migrations.
 * It is also written into every row's `schema_version` column.
 */
pub(crate) const LATEST: i64 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_migration_list_is_valid() {
        all().validate().unwrap();
    }

    #[test]
    fn latest_counts_every_migration() {
        assert_eq!(usize::try_from(LATEST).unwrap(), MIGRATIONS.len());
    }
}
