/*! `StartSession`: starts an empty, in-progress session. */

use gymos_domain::{Name, Session, SessionId, Source, TemplateId, Timestamp};

use crate::command::{CommandId, CommandOutcome};
use crate::error::EngineError;
use crate::gymos::GymOs;
use crate::ports::{Clock, IdGen, Ports, WriteTx};

/**
 * Starts a session. The engine generates its ID and returns it in
 * [`CommandOutcome::SessionStarted`].
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartSession {
    /**
     * When it started. `None` means now, from the clock port; an importer
     * passes the original time.
     */
    pub started_at: Option<Timestamp>,
    /** The template it was started from, which decides "last time" (D-1). */
    pub template: Option<TemplateId>,
    /** An optional title, such as "Heavy legs". */
    pub title: Option<Name>,
    /** Typed in, or imported. */
    pub source: Source,
}

pub(super) fn run<P: Ports>(
    os: &GymOs<P>,
    id: CommandId,
    command: StartSession,
) -> Result<CommandOutcome, EngineError> {
    super::write(os, id, |tx| {
        let started_at = command.started_at.unwrap_or_else(|| os.clock.now());
        let mut session = Session::start(
            SessionId::from_uuid(os.ids.next_id()),
            started_at,
            command.source,
        );
        session.set_template(command.template);
        session.set_title(command.title);
        tx.save_session(&session)?;
        Ok(CommandOutcome::SessionStarted(session.id()))
    })
}
