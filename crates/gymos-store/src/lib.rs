/*!
 * # gymos-store
 *
 * The SQLite implementation of the engine's ports, plus schema migrations
 * (DD-04).
 *
 * ## Rules for this crate
 *
 * - **The only crate that knows SQLite exists.** `cargo-deny` enforces this,
 *   and no `rusqlite` type appears in a public signature.
 * - **Errors stop here.** Driver errors are converted into the engine's
 *   repository error type and never leak upward (DD-07).
 * - **Migrations are forward-only**, applied when the database is opened,
 *   after a backup of a database that holds data.
 * - **No paths of its own.** The database location and the backup label
 *   are resolved by the shell and passed in through [`StoreConfig`].
 * - **No business rules.** The store saves and loads whole aggregates and
 *   lists candidates; choosing among them is the engine's job.
 *
 * ## Shape
 *
 * ```text
 * SqliteStore { Mutex<Connection> }       one connection owner (S-1)
 *   open(&StoreConfig) / in_memory()      open → refuse unknown schema → back up → migrate
 *   read(|SqliteRead| …)                  deferred transaction, always rolled back
 *   write(|SqliteWrite| …)                BEGIN IMMEDIATE; commit on Ok, roll back otherwise
 * ```
 */

mod backup;
mod error;
mod migrations;
mod open;
mod rows;
mod sqlite_store;
mod tx;

pub use error::OpenError;
pub use open::StoreConfig;
pub use sqlite_store::SqliteStore;
pub use tx::{SqliteRead, SqliteWrite};
