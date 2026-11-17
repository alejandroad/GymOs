/*! `AddExercise`: adds an exercise entry to a session. */

use gymos_domain::{ExerciseId, SessionExercise, SessionExerciseId, SessionId};

use crate::command::{CommandId, CommandOutcome};
use crate::error::EngineError;
use crate::gymos::GymOs;
use crate::load;
use crate::ports::{IdGen, Ports, WriteTx};

/**
 * Appends an entry for `exercise` to `session`. The same exercise may be
 * added more than once; each entry gets its own ID, returned in
 * [`CommandOutcome::ExerciseAdded`].
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddExercise {
    /** The session to add to. Finished sessions stay editable. */
    pub session: SessionId,
    /** The exercise performed. */
    pub exercise: ExerciseId,
}

pub(super) fn run<P: Ports>(
    os: &GymOs<P>,
    id: CommandId,
    command: AddExercise,
) -> Result<CommandOutcome, EngineError> {
    super::write(os, id, |tx| {
        let mut session = load::session(tx, command.session)?;
        let exercise = load::exercise(tx, command.exercise)?;
        let entry = SessionExercise::new(SessionExerciseId::from_uuid(os.ids.next_id()), &exercise);
        let entry_id = entry.id();
        session.add_exercise(entry)?;
        tx.save_session(&session)?;
        Ok(CommandOutcome::ExerciseAdded(entry_id))
    })
}
