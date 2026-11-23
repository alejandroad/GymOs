/*!
 * Command handlers, one file per command.
 *
 * Each handler makes exactly one `store.write` call and does, in order:
 * check whether the command was applied before, load, construct domain
 * values, decide (pure domain rules), persist and record the command with
 * its outcome. Any error rolls the whole write back, including the record
 * of the command, so a failed command can be retried with the same
 * [`CommandId`].
 */

mod add_exercise;
mod create_exercise;
mod finish_session;
mod log_set;
mod set_deload;
mod start_session;

pub use add_exercise::AddExercise;
pub use create_exercise::CreateExercise;
pub use finish_session::FinishSession;
pub use log_set::LogSet;
pub use set_deload::SetDeload;
pub use start_session::StartSession;

use crate::command::{Command, CommandId, CommandKind, CommandOutcome};
use crate::error::EngineError;
use crate::gymos::GymOs;
use crate::ports::{Ports, Store, WriteTx};

/** Applies one command. */
pub(crate) fn execute<P: Ports>(
    os: &GymOs<P>,
    command: Command,
) -> Result<CommandOutcome, EngineError> {
    match command.kind {
        CommandKind::CreateExercise(c) => create_exercise::run(os, command.id, c),
        CommandKind::StartSession(c) => start_session::run(os, command.id, c),
        CommandKind::AddExercise(c) => add_exercise::run(os, command.id, c),
        CommandKind::LogSet(c) => log_set::run(os, command.id, c),
        CommandKind::FinishSession(c) => finish_session::run(os, command.id, c),
        CommandKind::SetDeload(c) => set_deload::run(os, command.id, c),
    }
}

/**
 * The one write every command makes: runs `apply` and records `id` with
 * its outcome, in the same transaction. If `id` was applied before,
 * `apply` never runs and the stored outcome is returned (S-7).
 */
fn write<P: Ports>(
    os: &GymOs<P>,
    id: CommandId,
    apply: impl FnOnce(&mut <P::Store as Store>::Write<'_>) -> Result<CommandOutcome, EngineError>,
) -> Result<CommandOutcome, EngineError> {
    os.store.write(|tx| {
        if let Some(outcome) = tx.begin_command(id)? {
            return Ok(outcome);
        }
        let outcome = apply(tx)?;
        tx.finish_command(id, outcome)?;
        Ok(outcome)
    })
}
