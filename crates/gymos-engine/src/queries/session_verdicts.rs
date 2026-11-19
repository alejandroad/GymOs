/*!
 * Session verdicts: one overload verdict per entry, plus the session's
 * counts (DD-13, DD-15).
 *
 * ## Why verdicts are computed on read (D-6)
 *
 * The point of a verdict is that the user can open any past workout, not
 * just today's, and see whether it was productive: which exercises
 * progressed, matched or regressed, and the signals that explain each one.
 * That history view must always be right, so nothing is stored:
 *
 * - **Past workouts stay accurate.** Fixing a typo in an old set, logging a
 *   forgotten set, marking a session as a deload afterwards, importing an
 *   older session that slots in between two others, or changing an
 *   exercise's scheme all change what "last time" was or how it is judged.
 *   A stored verdict would be stale until something invalidated it; a
 *   computed one is correct on the next read.
 * - **A past workout's verdict is stable otherwise.** The baseline is always
 *   an earlier session, so logging new sessions never changes how an old
 *   one is judged.
 * - **Rule upgrades reach the whole history.** A new `ALGORITHM_VERSION`
 *   re-judges every past workout the next time it is opened, with no
 *   migration, and each verdict carries the version it was computed with.
 *
 * The cost is a few lookups per entry, small for one user's history. Each
 * verdict uses the exercise's *current* scheme (D-2). Records are the
 * exception to compute-on-read (DD-14): they are materialized, because "best
 * ever" needs the whole history.
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
