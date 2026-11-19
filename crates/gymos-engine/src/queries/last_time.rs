/*!
 * "Last time": which earlier entry an exercise is compared against (D-1,
 * DD-15).
 *
 * 1. If the current session came from a template: the most recent
 *    finished, non-deload session from that template that contains the
 *    exercise.
 * 2. Otherwise, or if there is none: the most recent finished, non-deload
 *    session containing the exercise.
 * 3. Only sessions before the current one count, so a past session's
 *    baseline doesn't move when later sessions are logged. "Before" and
 *    "most recent" order by `started_at` (UTC), then by `SessionId`.
 * 4. If the exercise appears more than once, entries match by occurrence
 *    (1st with 1st). An extra current entry uses the baseline's last one.
 * 5. A session only "contains" the exercise if that matched entry has at
 *    least one counted (non-warm-up) set. An empty or warm-up-only entry
 *    can't be compared with anything, so the search moves on to the next
 *    older session instead of hiding it.
 *
 * Unfinished sessions and deloads are never a baseline (DD-13 gap 2).
 */

use core::cmp::Reverse;

use gymos_domain::{
    ExerciseId, Session, SessionExercise, SessionExerciseId, SessionId, TemplateId, Timestamp,
};

use crate::error::{EngineError, Missing, RepoError};
use crate::load;
use crate::ports::{ReadTx, SessionHeader};

/** Which rule picked the baseline, so the UI can explain it. */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaselineRule {
    /** The most recent session from the same template (rule 1). */
    SameTemplate(TemplateId),
    /** The most recent session containing the same exercise (rule 2). */
    SameExercise,
}

/** The entry an exercise is compared against. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Baseline {
    /** The baseline session. */
    pub session: SessionId,
    /** When the baseline session started. */
    pub started_at: Timestamp,
    /** Which rule chose it. */
    pub rule: BaselineRule,
    /** The baseline entry, with its sets. */
    pub entry: SessionExercise,
}

/** Finds the baseline for one entry of a session the caller already loaded. */
pub(crate) fn resolve(
    tx: &impl ReadTx,
    session: &Session,
    entry: SessionExerciseId,
) -> Result<Option<Baseline>, EngineError> {
    let entries = session.exercises();
    let (position, current) = entries
        .iter()
        .enumerate()
        .find(|(_, e)| e.id() == entry)
        .ok_or(EngineError::NotFound(Missing::Entry {
            session: session.id(),
            entry,
        }))?;
    let exercise = current.exercise();
    let occurrence = entries
        .iter()
        .take(position)
        .filter(|e| e.exercise() == exercise)
        .count();

    let before = order_key(&SessionHeader::of(session));
    let mut candidates: Vec<SessionHeader> = tx
        .sessions_containing(exercise)?
        .into_iter()
        .filter(|h| h.ended_at.is_some() && !h.deload && order_key(h) < before)
        .collect();
    candidates.sort_by_key(|h| Reverse(order_key(h)));

    if let Some(template) = session.template() {
        let from_template = candidates.iter().filter(|h| h.template == Some(template));
        let rule = BaselineRule::SameTemplate(template);
        if let Some(baseline) = first_qualifying(tx, from_template, exercise, occurrence, rule)? {
            return Ok(Some(baseline));
        }
    }
    /* Sessions from the current template were all rejected above. */
    let others = candidates
        .iter()
        .filter(|h| session.template().is_none() || h.template != session.template());
    first_qualifying(tx, others, exercise, occurrence, BaselineRule::SameExercise)
}

/** The first of `headers` (newest first) that qualifies as a baseline. */
fn first_qualifying<'h>(
    tx: &impl ReadTx,
    headers: impl Iterator<Item = &'h SessionHeader>,
    exercise: ExerciseId,
    occurrence: usize,
    rule: BaselineRule,
) -> Result<Option<Baseline>, EngineError> {
    for header in headers {
        if let Some(baseline) = qualifying(tx, header, exercise, occurrence, rule)? {
            return Ok(Some(baseline));
        }
    }
    Ok(None)
}

/**
 * Loads a candidate and returns it as the baseline if its matched entry
 * has counted sets (rule 5), or `None` to move on to the next candidate.
 */
fn qualifying(
    tx: &impl ReadTx,
    header: &SessionHeader,
    exercise: ExerciseId,
    occurrence: usize,
    rule: BaselineRule,
) -> Result<Option<Baseline>, EngineError> {
    /* The store just listed this session, so failing to load it means the
     * store is inconsistent, not that the caller named a missing session. */
    let session = tx.session(header.id)?.ok_or_else(|| {
        RepoError::Corrupt(format!(
            "session {} was listed but can't be loaded",
            header.id
        ))
    })?;
    let entry = matching_entry(&session, exercise, occurrence).ok_or_else(|| {
        RepoError::Corrupt(format!(
            "session {} was listed as containing exercise {exercise} but has no entry for it",
            header.id
        ))
    })?;
    let has_counted_sets = entry
        .sets()
        .iter()
        .any(|set| set.set_type.counts_by_default());
    Ok(has_counted_sets.then(|| Baseline {
        session: session.id(),
        started_at: session.started_at(),
        rule,
        entry: entry.clone(),
    }))
}

/** The `last_time` query: loads the session, then resolves the baseline. */
pub(crate) fn last_time(
    tx: &impl ReadTx,
    session: SessionId,
    entry: SessionExerciseId,
) -> Result<Option<Baseline>, EngineError> {
    resolve(tx, &load::session(tx, session)?, entry)
}

/** "Most recent" order: start time in UTC, then ID (DD-15 rule 3). */
fn order_key(header: &SessionHeader) -> (i64, SessionId) {
    (header.started_at.unix_millis(), header.id)
}

/** The `occurrence`-th entry for `exercise`, or the last one if there are fewer. */
fn matching_entry(
    session: &Session,
    exercise: ExerciseId,
    occurrence: usize,
) -> Option<&SessionExercise> {
    let same: Vec<&SessionExercise> = session
        .exercises()
        .iter()
        .filter(|e| e.exercise() == exercise)
        .collect();
    same.get(occurrence).or_else(|| same.last()).copied()
}
