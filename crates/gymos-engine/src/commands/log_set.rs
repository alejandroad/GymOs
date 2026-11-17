/*! `LogSet`: logs one set into a session entry. */

use gymos_domain::{
    Effort, LoggedSet, Note, SessionExerciseId, SessionId, SetId, SetMetrics, SetType,
};

use crate::command::{CommandId, CommandOutcome};
use crate::error::{EngineError, Missing};
use crate::gymos::GymOs;
use crate::load;
use crate::ports::{IdGen, Ports, WriteTx};

/**
 * Appends a set to one entry of a session. Finished sessions accept sets
 * too, because past sessions stay editable (product §3.5). The set's ID is
 * returned in [`CommandOutcome::SetLogged`].
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogSet {
    /** The session the entry belongs to. */
    pub session: SessionId,
    /** The entry to log into. */
    pub entry: SessionExerciseId,
    /** The role the set played. */
    pub set_type: SetType,
    /** What was measured. Must match the entry's tracking mode. */
    pub metrics: SetMetrics,
    /** RPE or RIR, if recorded. */
    pub effort: Option<Effort>,
    /** A note on the set. */
    pub note: Option<Note>,
}

pub(super) fn run<P: Ports>(
    os: &GymOs<P>,
    id: CommandId,
    command: LogSet,
) -> Result<CommandOutcome, EngineError> {
    super::write(os, id, |tx| {
        let mut session = load::session(tx, command.session)?;
        let set = LoggedSet {
            id: SetId::from_uuid(os.ids.next_id()),
            set_type: command.set_type,
            metrics: command.metrics,
            effort: command.effort,
            note: command.note,
        };
        let set_id = set.id;
        session
            .entry_mut(command.entry)
            .ok_or(EngineError::NotFound(Missing::Entry {
                session: command.session,
                entry: command.entry,
            }))?
            .push_set(set)?;
        tx.save_session(&session)?;
        Ok(CommandOutcome::SetLogged(set_id))
    })
}
