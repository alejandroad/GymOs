/*!
 * Readable test setup: values from literals, sets and whole sessions.
 *
 * Builders take literals written in a test, so an invalid one is a bug in
 * that test. They fail loudly instead of returning a `Result`.
 */

#![expect(
    clippy::expect_used,
    reason = "builders take test literals; an invalid literal is a bug in the test, so failing loudly is right"
)]

use gymos_domain::{
    Effort, Exercise, ExerciseId, Load, LoggedSet, Name, Note, Reps, Rpe, Session, SessionExercise,
    SessionExerciseId, SessionId, SetId, SetMetrics, SetType, Source, TemplateId, Timestamp,
    TrackingMode, UtcOffset, WeightUnit,
};
use gymos_engine::{IdGen, LogSet};

/**
 * `kg(102, 50)` is 102.50 kg.
 *
 * # Panics
 *
 * If the load is out of range.
 */
pub fn kg(whole: u32, hundredths: u32) -> Load {
    Load::new(whole * 100 + hundredths, WeightUnit::Kilograms).expect("valid test load")
}

/**
 * `lb(220, 46)` is 220.46 lb.
 *
 * # Panics
 *
 * If the load is out of range.
 */
pub fn lb(whole: u32, hundredths: u32) -> Load {
    Load::new(whole * 100 + hundredths, WeightUnit::Pounds).expect("valid test load")
}

/**
 * A rep count.
 *
 * # Panics
 *
 * If `n` is out of range.
 */
pub fn reps(n: u16) -> Reps {
    Reps::new(n).expect("valid test reps")
}

/**
 * An RPE in half points: `rpe(17)` is RPE 8.5.
 *
 * # Panics
 *
 * If the value is out of range.
 */
pub fn rpe(half_points: u8) -> Effort {
    Effort::Rpe(Rpe::from_half_points(half_points).expect("valid test RPE"))
}

/**
 * A name.
 *
 * # Panics
 *
 * If the text is not a valid name.
 */
pub fn name(text: &str) -> Name {
    Name::new(text).expect("valid test name")
}

/** A UTC timestamp from Unix milliseconds. */
pub const fn at(unix_millis: i64) -> Timestamp {
    Timestamp::new(unix_millis, UtcOffset::UTC)
}

/** An exercise value, for seeding a store directly. */
pub fn exercise(id: ExerciseId, label: &str, mode: TrackingMode) -> Exercise {
    Exercise {
        id,
        name: name(label),
        mode,
        scheme: None,
    }
}

/** One set to log, `Working` unless changed. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetBuilder {
    set_type: SetType,
    metrics: SetMetrics,
    effort: Option<Effort>,
    note: Option<Note>,
}

/** A working weight × reps set. */
pub fn working(load: Load, n: u16) -> SetBuilder {
    SetBuilder::new(SetMetrics::WeightReps {
        load,
        reps: reps(n),
    })
}

impl SetBuilder {
    /** A working set with these measurements. */
    pub const fn new(metrics: SetMetrics) -> Self {
        Self {
            set_type: SetType::Working,
            metrics,
            effort: None,
            note: None,
        }
    }

    /** Changes the set's role. */
    #[must_use]
    pub const fn set_type(mut self, set_type: SetType) -> Self {
        self.set_type = set_type;
        self
    }

    /** Records an effort. */
    #[must_use]
    pub const fn effort(mut self, effort: Effort) -> Self {
        self.effort = Some(effort);
        self
    }

    /**
     * Adds a note.
     *
     * # Panics
     *
     * If the text is not a valid note.
     */
    #[must_use]
    pub fn note(mut self, text: &str) -> Self {
        self.note = Some(Note::new(text).expect("valid test note"));
        self
    }

    /** The `LogSet` command that logs this set into `entry`. */
    pub fn log_into(&self, session: SessionId, entry: SessionExerciseId) -> LogSet {
        LogSet {
            session,
            entry,
            set_type: self.set_type,
            metrics: self.metrics,
            effort: self.effort,
            note: self.note.clone(),
        }
    }

    /** The set as a domain value. */
    pub fn build(&self, id: SetId) -> LoggedSet {
        LoggedSet {
            id,
            set_type: self.set_type,
            metrics: self.metrics,
            effort: self.effort,
            note: self.note.clone(),
        }
    }
}

/**
 * A whole session as a domain value, for seeding a store directly. Seeding
 * bypasses the engine, which is how tests create data the engine would
 * refuse (D-4).
 */
#[derive(Debug, Clone)]
pub struct SessionBuilder {
    id: SessionId,
    started_at: Timestamp,
    ended_at: Option<Timestamp>,
    template: Option<TemplateId>,
    deload: bool,
    entries: Vec<(SessionExerciseId, Exercise, Vec<SetBuilder>)>,
}

impl SessionBuilder {
    /** An unfinished, manual, non-deload session with no entries. */
    pub const fn new(id: SessionId, started_at: Timestamp) -> Self {
        Self {
            id,
            started_at,
            ended_at: None,
            template: None,
            deload: false,
            entries: Vec::new(),
        }
    }

    /** Finishes the session at `ended_at`. */
    #[must_use]
    pub const fn finished_at(mut self, ended_at: Timestamp) -> Self {
        self.ended_at = Some(ended_at);
        self
    }

    /** Records the template it came from. */
    #[must_use]
    pub const fn template(mut self, template: TemplateId) -> Self {
        self.template = Some(template);
        self
    }

    /** Marks it as a deload. */
    #[must_use]
    pub const fn deload(mut self) -> Self {
        self.deload = true;
        self
    }

    /**
     * Appends an entry for `exercise`. The entry captures the tracking mode
     * of the `exercise` value given here, whatever the store holds.
     */
    #[must_use]
    pub fn entry(
        mut self,
        id: SessionExerciseId,
        exercise: &Exercise,
        sets: impl IntoIterator<Item = SetBuilder>,
    ) -> Self {
        self.entries
            .push((id, exercise.clone(), sets.into_iter().collect()));
        self
    }

    /**
     * Builds the session, taking set IDs from `ids`.
     *
     * # Panics
     *
     * If a set doesn't match its entry's mode, an entry ID repeats, or the
     * session ends before it starts.
     */
    pub fn build(&self, ids: &impl IdGen) -> Session {
        let mut session = Session::start(self.id, self.started_at, Source::Manual);
        session.set_template(self.template);
        session.set_deload(self.deload);
        for (id, exercise, sets) in &self.entries {
            let mut entry = SessionExercise::new(*id, exercise);
            for set in sets {
                entry
                    .push_set(set.build(SetId::from_uuid(ids.next_id())))
                    .expect("test set matches its entry's mode");
            }
            session.add_exercise(entry).expect("unique test entry ID");
        }
        if let Some(ended_at) = self.ended_at {
            session
                .finish(ended_at)
                .expect("test session ends after it starts");
        }
        session
    }
}
