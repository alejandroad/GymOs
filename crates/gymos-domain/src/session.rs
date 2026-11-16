/*!
 * Workout sessions: the aggregate the user logs.
 *
 * A [`Session`] owns its exercises, and each [`SessionExercise`] owns its
 * sets. Fields are private because these types have rules that span
 * fields (a session can't end before it starts, and every set must match its
 * exercise's tracking mode), so the only way to change them is through
 * methods that check those rules.
 */

use thiserror::Error;

use crate::exercise::{Exercise, TrackingMode};
use crate::ids::{ExerciseId, SessionId, TemplateId};
use crate::provenance::Source;
use crate::set::LoggedSet;
use crate::text::{Name, Note};
use crate::time::Timestamp;

/** Why a change to a session was rejected. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum SessionError {
    /** The end time is before the start time. */
    #[error("a session cannot end before it starts")]
    EndsBeforeStart,
    /** A set's measurements don't fit the exercise's tracking mode. */
    #[error("set is tracked as {found:?} but the exercise is tracked as {expected:?}")]
    ModeMismatch {
        /** The exercise's tracking mode. */
        expected: TrackingMode,
        /** The tracking mode of the rejected set. */
        found: TrackingMode,
    },
}

/** One exercise as performed within a session, with its sets in order. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionExercise {
    exercise: ExerciseId,
    mode: TrackingMode,
    sets: Vec<LoggedSet>,
    note: Option<Note>,
}

impl SessionExercise {
    /** Starts an empty entry for `exercise`, capturing its tracking mode. */
    pub fn new(exercise: &Exercise) -> Self {
        Self {
            exercise: exercise.id,
            mode: exercise.mode,
            sets: Vec::new(),
            note: None,
        }
    }

    /**
     * Appends a set.
     *
     * # Errors
     *
     * [`SessionError::ModeMismatch`] if the set's measurements belong to a
     * different tracking mode than the exercise.
     */
    pub fn push_set(&mut self, set: LoggedSet) -> Result<(), SessionError> {
        let found = set.metrics.mode();
        if found != self.mode {
            return Err(SessionError::ModeMismatch {
                expected: self.mode,
                found,
            });
        }
        self.sets.push(set);
        Ok(())
    }

    /** The exercise performed. */
    pub const fn exercise(&self) -> ExerciseId {
        self.exercise
    }

    /** The tracking mode every set here uses. */
    pub const fn mode(&self) -> TrackingMode {
        self.mode
    }

    /** The sets, in the order they were performed. */
    pub fn sets(&self) -> &[LoggedSet] {
        &self.sets
    }

    /** A note on this exercise for this session. */
    pub const fn note(&self) -> Option<&Note> {
        self.note.as_ref()
    }

    /** Replaces the note. */
    pub fn set_note(&mut self, note: Option<Note>) {
        self.note = note;
    }
}

/**
 * A workout: what was done, when, and in what order.
 *
 * Finished sessions stay editable by design: past sessions can be corrected
 * or deleted (product §3.5), and derived data such as records is recomputed.
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    id: SessionId,
    started_at: Timestamp,
    ended_at: Option<Timestamp>,
    title: Option<Name>,
    template: Option<TemplateId>,
    deload: bool,
    note: Option<Note>,
    exercises: Vec<SessionExercise>,
    source: Source,
}

impl Session {
    /** Starts an empty, in-progress session, recording where it came from. */
    pub const fn start(id: SessionId, started_at: Timestamp, source: Source) -> Self {
        Self {
            id,
            started_at,
            ended_at: None,
            title: None,
            template: None,
            deload: false,
            note: None,
            exercises: Vec::new(),
            source,
        }
    }

    /**
     * Marks the session finished. On an already finished session, this
     * corrects its end time.
     *
     * # Errors
     *
     * [`SessionError::EndsBeforeStart`] if `ended_at` is before the start.
     */
    pub fn finish(&mut self, ended_at: Timestamp) -> Result<(), SessionError> {
        if ended_at.is_before(self.started_at) {
            return Err(SessionError::EndsBeforeStart);
        }
        self.ended_at = Some(ended_at);
        Ok(())
    }

    /** Appends an exercise. The same exercise may appear more than once. */
    pub fn add_exercise(&mut self, exercise: SessionExercise) {
        self.exercises.push(exercise);
    }

    /** Stable identity. */
    pub const fn id(&self) -> SessionId {
        self.id
    }

    /** Typed in, or imported. */
    pub const fn source(&self) -> Source {
        self.source
    }

    /** When the session started. */
    pub const fn started_at(&self) -> Timestamp {
        self.started_at
    }

    /** When the session ended, or `None` while in progress. */
    pub const fn ended_at(&self) -> Option<Timestamp> {
        self.ended_at
    }

    /** The exercises, in the order they were performed. */
    pub fn exercises(&self) -> &[SessionExercise] {
        &self.exercises
    }

    /**
     * Mutable access for logging sets. Each [`SessionExercise`] still
     * enforces its own rules.
     */
    pub fn exercises_mut(&mut self) -> &mut [SessionExercise] {
        &mut self.exercises
    }

    /** An optional title, such as "Heavy legs". */
    pub const fn title(&self) -> Option<&Name> {
        self.title.as_ref()
    }

    /** Replaces the title. */
    pub fn set_title(&mut self, title: Option<Name>) {
        self.title = title;
    }

    /**
     * The template this session was started from, which decides what
     * "last time" compares against (§3.2).
     */
    pub const fn template(&self) -> Option<TemplateId> {
        self.template
    }

    /** Records which template this session came from. */
    pub fn set_template(&mut self, template: Option<TemplateId>) {
        self.template = template;
    }

    /**
     * Whether the user marked this session as a deload. A deload is never
     * counted as a regression (§3.3).
     */
    pub const fn is_deload(&self) -> bool {
        self.deload
    }

    /**
     * Marks or unmarks the session as a deload. Only the user does this;
     * the engine may only suggest it.
     */
    pub fn set_deload(&mut self, deload: bool) {
        self.deload = deload;
    }

    /** A note on the whole session. */
    pub const fn note(&self) -> Option<&Note> {
        self.note.as_ref()
    }

    /** Replaces the note. */
    pub fn set_note(&mut self, note: Option<Note>) {
        self.note = note;
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;
    use crate::ids::SetId;
    use crate::set::{SetMetrics, SetType};
    use crate::time::UtcOffset;
    use crate::units::{DurationSecs, Load, Reps, WeightUnit};

    fn bench() -> Exercise {
        Exercise {
            id: ExerciseId::from_uuid(Uuid::from_u128(1)),
            name: Name::new("Bench Press").unwrap(),
            mode: TrackingMode::WeightReps,
        }
    }

    fn set(metrics: SetMetrics) -> LoggedSet {
        LoggedSet {
            id: SetId::from_uuid(Uuid::from_u128(2)),
            set_type: SetType::Working,
            metrics,
            effort: None,
            note: None,
        }
    }

    fn at(unix_millis: i64) -> Timestamp {
        Timestamp::new(unix_millis, UtcOffset::UTC)
    }

    #[test]
    fn accepts_sets_matching_the_exercise_mode() {
        let mut entry = SessionExercise::new(&bench());
        let metrics = SetMetrics::WeightReps {
            load: Load::new(10_000, WeightUnit::Kilograms).unwrap(),
            reps: Reps::new(5).unwrap(),
        };
        entry.push_set(set(metrics)).unwrap();
        assert_eq!(entry.sets().len(), 1);
    }

    #[test]
    fn rejects_sets_from_another_mode() {
        let mut entry = SessionExercise::new(&bench());
        let plank = SetMetrics::Duration {
            duration: DurationSecs::new(60).unwrap(),
            load: None,
        };
        assert_eq!(
            entry.push_set(set(plank)),
            Err(SessionError::ModeMismatch {
                expected: TrackingMode::WeightReps,
                found: TrackingMode::Duration,
            })
        );
        assert_eq!(entry.sets(), []);
    }

    #[test]
    fn starts_in_progress_and_finishes() {
        let mut session = Session::start(
            SessionId::from_uuid(Uuid::from_u128(3)),
            at(1_000),
            Source::Manual,
        );
        assert_eq!(session.ended_at(), None);
        session.finish(at(5_000)).unwrap();
        assert_eq!(session.ended_at(), Some(at(5_000)));
    }

    #[test]
    fn cannot_end_before_it_starts() {
        let mut session = Session::start(
            SessionId::from_uuid(Uuid::from_u128(3)),
            at(5_000),
            Source::Manual,
        );
        assert_eq!(
            session.finish(at(1_000)),
            Err(SessionError::EndsBeforeStart)
        );
        assert_eq!(session.ended_at(), None);
    }

    #[test]
    fn logs_sets_through_the_session() {
        let mut session = Session::start(
            SessionId::from_uuid(Uuid::from_u128(3)),
            at(0),
            Source::Manual,
        );
        session.add_exercise(SessionExercise::new(&bench()));
        let metrics = SetMetrics::WeightReps {
            load: Load::new(6_000, WeightUnit::Kilograms).unwrap(),
            reps: Reps::new(8).unwrap(),
        };
        session.exercises_mut()[0].push_set(set(metrics)).unwrap();
        assert_eq!(session.exercises()[0].sets().len(), 1);
    }

    #[test]
    fn finished_sessions_stay_editable() {
        let mut session = Session::start(
            SessionId::from_uuid(Uuid::from_u128(3)),
            at(0),
            Source::Manual,
        );
        session.add_exercise(SessionExercise::new(&bench()));
        session.finish(at(5_000)).unwrap();
        session.finish(at(7_000)).unwrap();
        assert_eq!(session.ended_at(), Some(at(7_000)));
        let metrics = SetMetrics::WeightReps {
            load: Load::new(6_000, WeightUnit::Kilograms).unwrap(),
            reps: Reps::new(8).unwrap(),
        };
        session.exercises_mut()[0].push_set(set(metrics)).unwrap();
        assert_eq!(session.exercises()[0].sets().len(), 1);
    }
}
