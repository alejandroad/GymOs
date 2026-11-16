/*!
 * # gymos-store
 *
 * The SQLite implementation of the engine's ports, plus schema migrations
 * (DD-04).
 *
 * ## Rules for this crate
 *
 * - **The only crate that knows SQLite exists.** `cargo-deny` enforces this.
 * - **Errors stop here.** Driver errors are converted into the engine's
 *   repository error type and never leak upward (DD-07).
 * - **Migrations are forward-only**, applied at startup after a backup copy
 *   of the database file is taken.
 * - **No paths of its own.** The database location is resolved by the shell
 *   and passed in.
 */
