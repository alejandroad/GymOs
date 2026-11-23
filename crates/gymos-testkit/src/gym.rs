/*!
 * A ready-made engine over the fake clock and IDs, with one-line helpers
 * for the commands tests run most.
 *
 * The store is a type parameter (S-9): [`FakeStore`] by default, or any
 * cloneable [`Store`], so the same stories ([`stories`](crate::stories))
 * run against the real SQLite store too.
 *
 * The helpers fail loudly: they are for setting up a scenario, where any
 * error is a bug in the test. To check an error, call
 * [`TestGym::run`] and inspect the result.
 */

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test setup helpers: an unexpected error or outcome is a failed test"
)]

use core::fmt;
use core::marker::PhantomData;
use gymos_domain::progression::ProgressionScheme;
use gymos_domain::{
    ExerciseId, SessionExerciseId, SessionId, SetId, Source, TemplateId, Timestamp, TrackingMode,
    UtcOffset,
};

use gymos_engine::{
    AddExercise, Command, CommandId, CommandKind, CommandOutcome, CreateExercise, EngineError,
    FinishSession, GymOs, IdGen, Ports, SetDeload, StartSession, Store,
};

use crate::builders::{SetBuilder, name};
use crate::fake_store::FakeStore;
use crate::fixed_clock::SteppingClock;
use crate::seq_ids::SeqIds;

/**
 * The test [`Ports`] bundle: store `S` (the fake by default), a
 * [`SteppingClock`] and [`SeqIds`]. A type-level marker only; it is never
 * constructed.
 */
pub struct TestPorts<S = FakeStore>(PhantomData<fn() -> S>);

impl<S> fmt::Debug for TestPorts<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TestPorts")
    }
}

impl<S: Store> Ports for TestPorts<S> {
    type Store = S;
    type Clock = SteppingClock;
    type Ids = SeqIds;
}

/** What [`TestGym::log_session`] logs. */
#[derive(Debug, Clone, Default)]
pub struct SessionPlan {
    template: Option<TemplateId>,
    deload: bool,
    unfinished: bool,
    entries: Vec<(ExerciseId, Vec<SetBuilder>)>,
}

impl SessionPlan {
    /** A finished, non-deload session with no template and no entries. */
    pub fn new() -> Self {
        Self::default()
    }

    /** Starts it from `template`. */
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

    /** Leaves it unfinished. */
    #[must_use]
    pub const fn unfinished(mut self) -> Self {
        self.unfinished = true;
        self
    }

    /** Adds an entry for `exercise` with these sets, in order. */
    #[must_use]
    pub fn exercise(
        mut self,
        exercise: ExerciseId,
        sets: impl IntoIterator<Item = SetBuilder>,
    ) -> Self {
        self.entries.push((exercise, sets.into_iter().collect()));
        self
    }
}

/** The IDs [`TestGym::log_session`] created. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggedSession {
    /** The session. */
    pub session: SessionId,
    /** One entry per planned exercise, in order. */
    pub entries: Vec<SessionExerciseId>,
}

/** 2026-01-05 07:00 UTC, when the clock starts. */
const CLOCK_START: Timestamp = Timestamp::new(1_767_596_400_000, UtcOffset::UTC);

/**
 * `GymOs` over store `S` (a [`FakeStore`] unless given another), a
 * [`SteppingClock`] (one hour per read, from [`TestGym::START`]) and
 * [`SeqIds`]. Command IDs come from their own generator, so they never
 * collide with entity IDs.
 *
 * `S` must be [`Clone`] so the gym keeps a second handle on the same
 * store, for snapshots and direct seeding.
 */
#[derive(Debug)]
pub struct TestGym<S: Store + Clone = FakeStore> {
    os: GymOs<TestPorts<S>>,
    store: S,
    commands: SeqIds,
}

impl Default for TestGym {
    fn default() -> Self {
        Self::new()
    }
}

impl TestGym {
    /** 2026-01-05 07:00 UTC, when the clock starts. */
    pub const START: Timestamp = CLOCK_START;

    /** A fresh engine over an empty [`FakeStore`]. */
    pub fn new() -> Self {
        Self::with_store(FakeStore::new())
    }
}

impl<S: Store + Clone> TestGym<S> {
    /**
     * A fresh engine over `store`, which should be empty for the clock and
     * ID sequences to make sense.
     */
    pub fn with_store(store: S) -> Self {
        let os = GymOs::new(
            store.clone(),
            SteppingClock::new(CLOCK_START, SteppingClock::HOUR),
            SeqIds::new(),
        );
        Self {
            os,
            store,
            commands: SeqIds::with_prefix(0xC << 124),
        }
    }

    /** The engine. */
    pub const fn os(&self) -> &GymOs<TestPorts<S>> {
        &self.os
    }

    /** A handle on the engine's store, for snapshots and direct seeding. */
    pub const fn store(&self) -> &S {
        &self.store
    }

    /** A command ID never used before. */
    pub fn command_id(&self) -> CommandId {
        CommandId::from_uuid(self.commands.next_id())
    }

    /**
     * Runs one command with a fresh command ID.
     *
     * # Errors
     *
     * Whatever the engine returns.
     */
    pub fn run(&self, kind: impl Into<CommandKind>) -> Result<CommandOutcome, EngineError> {
        self.os.execute(Command::new(self.command_id(), kind))
    }

    fn applied(&self, kind: impl Into<CommandKind>) -> CommandOutcome {
        self.run(kind).expect("test command succeeds")
    }

    /**
     * Creates an exercise.
     *
     * # Panics
     *
     * If the command fails.
     */
    pub fn create_exercise(
        &self,
        label: &str,
        mode: TrackingMode,
        scheme: Option<ProgressionScheme>,
    ) -> ExerciseId {
        let outcome = self.applied(CreateExercise {
            name: name(label),
            mode,
            scheme,
        });
        let CommandOutcome::ExerciseCreated(id) = outcome else {
            panic!("expected ExerciseCreated, got {outcome:?}");
        };
        id
    }

    /**
     * Starts a manual session now, from `template` if given.
     *
     * # Panics
     *
     * If the command fails.
     */
    pub fn start_session(&self, template: Option<TemplateId>) -> SessionId {
        let outcome = self.applied(StartSession {
            started_at: None,
            template,
            title: None,
            source: Source::Manual,
        });
        let CommandOutcome::SessionStarted(id) = outcome else {
            panic!("expected SessionStarted, got {outcome:?}");
        };
        id
    }

    /**
     * Adds an entry for `exercise` to `session`.
     *
     * # Panics
     *
     * If the command fails.
     */
    pub fn add_exercise(&self, session: SessionId, exercise: ExerciseId) -> SessionExerciseId {
        let outcome = self.applied(AddExercise { session, exercise });
        let CommandOutcome::ExerciseAdded(id) = outcome else {
            panic!("expected ExerciseAdded, got {outcome:?}");
        };
        id
    }

    /**
     * Logs one set.
     *
     * # Panics
     *
     * If the command fails.
     */
    pub fn log_set(&self, session: SessionId, entry: SessionExerciseId, set: &SetBuilder) -> SetId {
        let outcome = self.applied(set.log_into(session, entry));
        let CommandOutcome::SetLogged(id) = outcome else {
            panic!("expected SetLogged, got {outcome:?}");
        };
        id
    }

    /**
     * Finishes `session` now.
     *
     * # Panics
     *
     * If the command fails.
     */
    pub fn finish(&self, session: SessionId) {
        self.applied(FinishSession {
            session,
            ended_at: None,
        });
    }

    /**
     * Marks or unmarks `session` as a deload.
     *
     * # Panics
     *
     * If the command fails.
     */
    pub fn set_deload(&self, session: SessionId, deload: bool) {
        self.applied(SetDeload { session, deload });
    }

    /**
     * Logs a whole session through the engine: start, add each exercise
     * with its sets, mark the deload and finish, as the plan says.
     *
     * # Panics
     *
     * If any command fails.
     */
    pub fn log_session(&self, plan: &SessionPlan) -> LoggedSession {
        let session = self.start_session(plan.template);
        let mut entries = Vec::with_capacity(plan.entries.len());
        for (exercise, sets) in &plan.entries {
            let entry = self.add_exercise(session, *exercise);
            for set in sets {
                self.log_set(session, entry, set);
            }
            entries.push(entry);
        }
        if plan.deload {
            self.set_deload(session, true);
        }
        if !plan.unfinished {
            self.finish(session);
        }
        LoggedSession { session, entries }
    }
}
