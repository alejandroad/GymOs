/*!
 * # gymos-engine
 *
 * The GymOs engine: the `GymOs` struct, the commands that change state, the
 * queries that read it, and the ports (traits) it needs from the outside
 * world (store, clock, ID generator).
 *
 * ## Rules for this crate
 *
 * - **Owns the ports.** The engine defines the traits; adapters such as
 *   `gymos-store` implement them (dependency inversion, DD-03).
 * - **Orchestrates, doesn't decide.** Business rules live in `gymos-domain`.
 *   The engine sequences load, decide and persist inside one transaction.
 * - **Synchronous.** No async runtime here; the shell decides how to keep
 *   the UI responsive.
 * - **Dependencies:** `gymos-domain` and `thiserror` only. Never SQLite,
 *   never Tauri.
 *
 * ## Shape
 *
 * ```text
 * GymOs<P: Ports>
 *   execute(Command)            -> CommandOutcome   one store.write each
 *   last_time(session, entry)   -> Option<Baseline> one store.read
 *   session_verdicts(session)   -> SessionVerdicts  one store.read
 * ```
 */

mod command;
mod commands;
mod error;
mod gymos;
mod load;
mod ports;
mod queries;

pub use command::{Command, CommandId, CommandKind, CommandOutcome};
pub use commands::{AddExercise, CreateExercise, FinishSession, LogSet, SetDeload, StartSession};
pub use error::{DomainError, EngineError, Missing, RepoError};
pub use gymos::GymOs;
pub use ports::{Clock, IdGen, Ports, ReadTx, SessionHeader, Store, WriteTx};
pub use queries::{Baseline, BaselineRule, EntryOutcome, EntryVerdict, SessionVerdicts};
