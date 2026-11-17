/*! The `GymOs` struct: the engine's single entry point. */

use core::fmt;

use gymos_domain::{SessionExerciseId, SessionId};

use crate::command::{Command, CommandOutcome};
use crate::commands;
use crate::error::EngineError;
use crate::ports::{Ports, Store};
use crate::queries::{self, Baseline, SessionVerdicts};

/**
 * The GymOs engine: commands that change state and queries that read it,
 * over one set of [`Ports`].
 *
 * Methods take `&self` and are synchronous, so the shell can share one
 * `GymOs` across threads; locking, if any, lives inside the store.
 */
pub struct GymOs<P: Ports> {
    pub(crate) store: P::Store,
    pub(crate) clock: P::Clock,
    pub(crate) ids: P::Ids,
}

impl<P: Ports> GymOs<P> {
    /** Builds the engine over concrete ports. */
    pub const fn new(store: P::Store, clock: P::Clock, ids: P::Ids) -> Self {
        Self { store, clock, ids }
    }

    /**
     * Applies one command in one transaction. Applying the same
     * [`CommandId`](crate::CommandId) again returns
     * [`CommandOutcome::AlreadyApplied`] and changes nothing.
     *
     * # Errors
     *
     * - [`EngineError::NotFound`] if the command names a missing session,
     *   entry or exercise.
     * - [`EngineError::Domain`] if a domain rule rejects the change, such as
     *   a set in the wrong tracking mode or an end time before the start.
     * - [`EngineError::Repo`] if the store fails.
     *
     * On any error nothing is written, not even the command's ID.
     */
    pub fn execute(&self, command: Command) -> Result<CommandOutcome, EngineError> {
        commands::execute(self, command)
    }

    /**
     * The baseline that `entry` in `session` is compared against, or `None`
     * if no earlier session qualifies (D-1). Never a deload and never an
     * unfinished session.
     *
     * # Errors
     *
     * - [`EngineError::NotFound`] if the session or the entry is missing.
     * - [`EngineError::Repo`] if the store fails or its data is
     *   inconsistent.
     */
    pub fn last_time(
        &self,
        session: SessionId,
        entry: SessionExerciseId,
    ) -> Result<Option<Baseline>, EngineError> {
        self.store.read(|tx| queries::last_time(tx, session, entry))
    }

    /**
     * One verdict per entry of `session`, in order, plus the session's
     * counts. Computed on read with each exercise's current scheme (D-6).
     *
     * # Errors
     *
     * - [`EngineError::NotFound`] if the session, or an exercise one of its
     *   entries names, is missing.
     * - [`EngineError::Domain`] if stored data is inconsistent, such as a
     *   baseline tracked in another mode (D-4). A mode without overload
     *   rules is not an error: its entry is
     *   [`EntryOutcome::Unsupported`](crate::EntryOutcome::Unsupported).
     * - [`EngineError::Repo`] if the store fails.
     */
    pub fn session_verdicts(&self, session: SessionId) -> Result<SessionVerdicts, EngineError> {
        self.store.read(|tx| queries::session_verdicts(tx, session))
    }
}

impl<P: Ports> fmt::Debug for GymOs<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GymOs").finish_non_exhaustive()
    }
}
