/*! `SetDeload`: marks or unmarks a session as a deload. */

use gymos_domain::SessionId;

use crate::command::{CommandId, CommandOutcome};
use crate::error::EngineError;
use crate::gymos::GymOs;
use crate::load;
use crate::ports::{Ports, WriteTx};

/**
 * Marks or unmarks a session as a deload. A deload's own verdicts are
 * `Deload`, and it is never a "last time" baseline (DD-13 gap 2). Verdicts
 * are computed on read, so the change shows up at once.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetDeload {
    /** The session to mark. */
    pub session: SessionId,
    /** `true` to mark it as a deload, `false` to clear the mark. */
    pub deload: bool,
}

pub(super) fn run<P: Ports>(
    os: &GymOs<P>,
    id: CommandId,
    command: SetDeload,
) -> Result<CommandOutcome, EngineError> {
    super::write(os, id, |tx| {
        let mut session = load::session(tx, command.session)?;
        session.set_deload(command.deload);
        tx.save_session(&session)?;
        Ok(CommandOutcome::DeloadSet)
    })
}
