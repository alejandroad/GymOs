/*!
 * Loading aggregates that a command or query requires to exist, turning a
 * missing one into [`EngineError::NotFound`].
 */

use gymos_domain::{Exercise, ExerciseId, Session, SessionId};

use crate::error::{EngineError, Missing};
use crate::ports::ReadTx;

/** Loads a session that must exist. */
pub(crate) fn session(tx: &impl ReadTx, id: SessionId) -> Result<Session, EngineError> {
    tx.session(id)?
        .ok_or(EngineError::NotFound(Missing::Session(id)))
}

/** Loads an exercise that must exist. */
pub(crate) fn exercise(tx: &impl ReadTx, id: ExerciseId) -> Result<Exercise, EngineError> {
    tx.exercise(id)?
        .ok_or(EngineError::NotFound(Missing::Exercise(id)))
}
