/*! `FinishSession`: records a session's end time. */

use gymos_domain::{SessionId, Timestamp};

use crate::command::{CommandId, CommandOutcome};
use crate::error::EngineError;
use crate::gymos::GymOs;
use crate::load;
use crate::ports::{Clock, Ports, WriteTx};

/**
 * Finishes a session. On a session that is already finished, this corrects
 * its end time. Only finished sessions can be a "last time" baseline (D-1).
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinishSession {
    /** The session to finish. */
    pub session: SessionId,
    /** When it ended. `None` means now, from the clock port. */
    pub ended_at: Option<Timestamp>,
}

pub(super) fn run<P: Ports>(
    os: &GymOs<P>,
    id: CommandId,
    command: FinishSession,
) -> Result<CommandOutcome, EngineError> {
    super::write(os, id, |tx| {
        let mut session = load::session(tx, command.session)?;
        let ended_at = command.ended_at.unwrap_or_else(|| os.clock.now());
        session.finish(ended_at)?;
        tx.save_session(&session)?;
        Ok(CommandOutcome::SessionFinished)
    })
}
