/*!
 * Session verdicts: one overload verdict per entry, plus the session's
 * counts (DD-13, DD-15).
 *
 * Verdicts are computed on read and never stored (D-6). An edit, a deload
 * toggle, a scheme change or a new `ALGORITHM_VERSION` shows up at once,
 * with nothing to invalidate. Each verdict uses the exercise's *current*
 * scheme (D-2).
 */

use gymos_domain::progression::{
    ExerciseVerdict, SessionVerdict, VerdictError, VerdictInput, exercise_verdict,
};
use gymos_domain::{ExerciseId, SessionExerciseId, SessionId, TrackingMode};

use super::last_time::{Baseline, resolve};
use crate::error::EngineError;
use crate::load;
use crate::ports::ReadTx;

/** The verdicts for every entry of one session. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionVerdicts {
    /** The session. */
    pub session: SessionId,
    /** One result per entry, in the session's order. */
    pub entries: Vec<EntryVerdict>,
    /**
     * The counts over entries that have a verdict. Entries in a mode
     * without rules are not counted.
     */
    pub tally: SessionVerdict,
}

/** The result for one entry. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryVerdict {
    /** The entry. */
    pub entry: SessionExerciseId,
    /** The exercise performed. */
    pub exercise: ExerciseId,
    /** What it was compared against, or `None` if nothing qualified. */
    pub baseline: Option<Baseline>,
    /** The verdict, or why there is none. */
    pub outcome: EntryOutcome,
}

/** Whether an entry got a verdict. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryOutcome {
    /** The verdict, with every signal that explains it. */
    Verdict(ExerciseVerdict),
    /**
     * Overload rules for this tracking mode don't exist yet. An expected
     * outcome, so it doesn't fail the rest of the session.
     */
    Unsupported(TrackingMode),
}

/** The `session_verdicts` query. */
pub(crate) fn session_verdicts(
    tx: &impl ReadTx,
    session: SessionId,
) -> Result<SessionVerdicts, EngineError> {
    let session = load::session(tx, session)?;
    let mut entries = Vec::with_capacity(session.exercises().len());
    for current in session.exercises() {
        let baseline = resolve(tx, &session, current.id())?;
        let exercise = load::exercise(tx, current.exercise())?;
        let result = exercise_verdict(&VerdictInput {
            baseline: baseline.as_ref().map(|b| &b.entry),
            current,
            current_is_deload: session.is_deload(),
            scheme: exercise.scheme.as_ref(),
        });
        let outcome = match result {
            Ok(verdict) => EntryOutcome::Verdict(verdict),
            Err(VerdictError::UnsupportedMode(mode)) => EntryOutcome::Unsupported(mode),
            /* The engine never pairs different exercises and modes can't
             * change (D-4), so these mean stored data is inconsistent. */
            Err(
                error @ (VerdictError::ExerciseMismatch { .. } | VerdictError::ModeMismatch { .. }),
            ) => {
                return Err(error.into());
            }
        };
        entries.push(EntryVerdict {
            entry: current.id(),
            exercise: current.exercise(),
            baseline,
            outcome,
        });
    }
    let tally = SessionVerdict::tally(entries.iter().filter_map(|e| match &e.outcome {
        EntryOutcome::Verdict(verdict) => Some(verdict),
        EntryOutcome::Unsupported(_) => None,
    }));
    Ok(SessionVerdicts {
        session: session.id(),
        entries,
        tally,
    })
}
