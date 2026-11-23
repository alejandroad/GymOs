/*!
 * Store-generic contract scenarios (kernel-architecture §6.4).
 *
 * Each scenario takes a fresh, empty store and checks one promise the
 * engine relies on. The testkit runs them against [`FakeStore`]; the SQLite
 * store runs the same functions, which proves the fake behaves like the
 * real thing.
 *
 * A broken promise fails an assertion, so these are for tests only.
 *
 * [`FakeStore`]: crate::FakeStore
 */

use gymos_domain::progression::{ProgressionScheme, RepRange};
use gymos_domain::{
    Distance, DurationSecs, Effort, Exercise, ExerciseId, ImportFormat, Note, Rir, Session,
    SessionExercise, SessionExerciseId, SessionId, SetId, SetMetrics, SetType, Source, TemplateId,
    TrackingMode, Uuid,
};
use gymos_engine::{
    CommandId, CommandOutcome, EngineError, IdGen, Missing, ReadTx, SessionHeader, Store, WriteTx,
};

use crate::builders::{SessionBuilder, SetBuilder, at, exercise, kg, lb, name, reps, rpe, working};
use crate::seq_ids::SeqIds;

fn bench() -> Exercise {
    exercise(
        ExerciseId::from_uuid(Uuid::from_u128(1)),
        "Bench Press",
        TrackingMode::WeightReps,
    )
}

fn squat() -> Exercise {
    exercise(
        ExerciseId::from_uuid(Uuid::from_u128(2)),
        "Squat",
        TrackingMode::WeightReps,
    )
}

fn session_id(n: u128) -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(0x5E55 << 16 | n))
}

fn entry_id(n: u128) -> SessionExerciseId {
    SessionExerciseId::from_uuid(Uuid::from_u128(0xE << 16 | n))
}

fn command(n: u128) -> CommandId {
    CommandId::from_uuid(Uuid::from_u128(0xC << 16 | n))
}

/** A finished session with three bench sets, from a template. */
fn bench_session(ids: &SeqIds) -> Session {
    SessionBuilder::new(session_id(1), at(1_000))
        .template(TemplateId::from_uuid(Uuid::from_u128(7)))
        .entry(entry_id(1), &bench(), vec![working(kg(100, 0), 8); 3])
        .finished_at(at(5_000))
        .build(ids)
}

fn seed<S: Store>(
    store: &S,
    exercises: &[Exercise],
    sessions: &[Session],
) -> Result<(), EngineError> {
    store.write(|tx| {
        for e in exercises {
            tx.save_exercise(e)?;
        }
        for s in sessions {
            tx.save_session(s)?;
        }
        Ok(())
    })
}

/**
 * Runs every scenario, each against a new store from `make`.
 *
 * # Errors
 *
 * The first store error.
 *
 * # Panics
 *
 * If the store breaks a promise.
 */
pub fn run_all<S: Store>(make: impl Fn() -> S) -> Result<(), EngineError> {
    missing_entities_read_as_none(&make())?;
    committed_writes_are_read_back_whole(&make())?;
    every_field_round_trips(&make())?;
    saving_an_exercise_replaces_it(&make())?;
    a_failed_write_changes_nothing(&make())?;
    a_write_sees_its_own_changes(&make())?;
    a_command_is_applied_once(&make())?;
    every_outcome_round_trips(&make())?;
    saving_a_session_replaces_it(&make())?;
    sessions_containing_lists_every_match(&make())?;
    Ok(())
}

/**
 * An empty store has no exercises, no sessions and no matches.
 *
 * # Errors
 *
 * Any store error.
 *
 * # Panics
 *
 * If the store breaks the promise.
 */
pub fn missing_entities_read_as_none<S: Store>(store: &S) -> Result<(), EngineError> {
    store.read(|tx| {
        assert_eq!(tx.exercise(bench().id)?, None);
        assert_eq!(tx.session(session_id(1))?, None);
        assert_eq!(tx.sessions_containing(bench().id)?, []);
        Ok(())
    })
}

/**
 * What one write saved is read back exactly, every field, entry and set.
 *
 * # Errors
 *
 * Any store error.
 *
 * # Panics
 *
 * If the store breaks the promise.
 */
pub fn committed_writes_are_read_back_whole<S: Store>(store: &S) -> Result<(), EngineError> {
    let session = bench_session(&SeqIds::new());
    seed(store, &[bench()], std::slice::from_ref(&session))?;
    store.read(|tx| {
        assert_eq!(tx.exercise(bench().id)?, Some(bench()));
        assert_eq!(tx.session(session.id())?, Some(session.clone()));
        Ok(())
    })
}

/** One set's measurements in every tracking mode. */
#[expect(
    clippy::expect_used,
    reason = "fixture built from known-valid literals; an invalid one is a bug in the scenario"
)]
fn every_mode_metrics() -> [(TrackingMode, SetMetrics); 8] {
    let secs = |s| DurationSecs::new(s).expect("valid duration");
    [
        (
            TrackingMode::WeightReps,
            SetMetrics::WeightReps {
                load: lb(225, 0),
                reps: reps(5),
            },
        ),
        (
            TrackingMode::BodyweightReps,
            SetMetrics::BodyweightReps { reps: reps(12) },
        ),
        (
            TrackingMode::WeightedBodyweight,
            SetMetrics::WeightedBodyweight {
                added: kg(20, 0),
                reps: reps(6),
            },
        ),
        (
            TrackingMode::AssistedBodyweight,
            SetMetrics::AssistedBodyweight {
                assistance: kg(15, 0),
                reps: reps(8),
            },
        ),
        (
            TrackingMode::Duration,
            SetMetrics::Duration {
                duration: secs(90),
                load: Some(kg(10, 0)),
            },
        ),
        (
            TrackingMode::DistanceLoad,
            SetMetrics::DistanceLoad {
                distance: Distance::from_millimetres(40_000).expect("valid distance"),
                load: kg(32, 0),
                duration: Some(secs(45)),
            },
        ),
        (
            TrackingMode::RepsOnly,
            SetMetrics::RepsOnly { reps: reps(20) },
        ),
        (
            TrackingMode::TimeReps,
            SetMetrics::TimeReps {
                reps: reps(15),
                duration: secs(60),
            },
        ),
    ]
}

/**
 * One exercise per tracking mode, each with a progression scheme, and a
 * finished, imported, titled, noted deload session that logs every set
 * type, both effort scales and a note in every mode. A store that drops any
 * field fails [`every_field_round_trips`].
 */
#[expect(
    clippy::expect_used,
    reason = "fixture built from known-valid literals; an invalid one is a bug in the scenario"
)]
fn rich_fixture() -> (Vec<Exercise>, Session) {
    let scheme = ProgressionScheme::new(
        RepRange::new(reps(6), reps(8)).expect("valid range"),
        kg(2, 50),
    )
    .expect("valid scheme");
    let rir = |r| Effort::Rir(Rir::new(r).expect("valid RIR"));
    let metrics = every_mode_metrics();
    let set_types = [
        SetType::Warmup,
        SetType::Working,
        SetType::Drop,
        SetType::Failure,
        SetType::TopSet,
        SetType::Backoff,
    ];

    let mut session = Session::start(
        session_id(1),
        at(1_000),
        Source::Import {
            format: ImportFormat::Strong,
            importer_version: 3,
        },
    );
    session.set_title(Some(name("Heavy day")));
    session.set_note(Some(
        Note::new("felt strong\nslept well").expect("valid note"),
    ));
    session.set_template(Some(TemplateId::from_uuid(Uuid::from_u128(7))));
    session.set_deload(true);
    let ids = SeqIds::with_prefix(0x5E7 << 64);
    let mut exercises = Vec::new();
    for (n, (mode, metrics)) in (1_u128..).zip(metrics) {
        let mut lift = exercise(
            ExerciseId::from_uuid(Uuid::from_u128(100 + n)),
            "Lift",
            mode,
        );
        lift.scheme = Some(scheme);
        let mut entry = SessionExercise::new(entry_id(n), &lift);
        entry.set_note(Some(Note::new("cue: brace").expect("valid note")));
        for (i, set_type) in set_types.into_iter().enumerate() {
            let effort = if i % 2 == 0 { rpe(17) } else { rir(2) };
            let set = SetBuilder::new(metrics)
                .set_type(set_type)
                .effort(effort)
                .note("set note");
            entry
                .push_set(set.build(SetId::from_uuid(ids.next_id())))
                .expect("set matches its entry's mode");
        }
        session.add_exercise(entry).expect("unique entry ID");
        exercises.push(lift);
    }
    session.finish(at(9_000)).expect("ends after it starts");
    (exercises, session)
}

/**
 * Every field of every type the engine stores survives a write and a read:
 * schemes, all tracking modes and set types, both effort scales, notes,
 * titles, templates, deload marks and import provenance.
 *
 * # Errors
 *
 * Any store error.
 *
 * # Panics
 *
 * If the store breaks the promise.
 */
pub fn every_field_round_trips<S: Store>(store: &S) -> Result<(), EngineError> {
    let (exercises, session) = rich_fixture();
    seed(store, &exercises, std::slice::from_ref(&session))?;
    store.read(|tx| {
        for e in &exercises {
            assert_eq!(tx.exercise(e.id)?.as_ref(), Some(e));
        }
        assert_eq!(tx.session(session.id())?, Some(session.clone()));
        Ok(())
    })
}

/**
 * Saving an exercise again replaces it, so a renamed exercise or a changed
 * scheme is what the next read sees (D-2).
 *
 * # Errors
 *
 * Any store error.
 *
 * # Panics
 *
 * If the store breaks the promise.
 */
pub fn saving_an_exercise_replaces_it<S: Store>(store: &S) -> Result<(), EngineError> {
    let (exercises, _) = rich_fixture();
    let Some(first) = exercises.first() else {
        return Ok(());
    };
    seed(store, std::slice::from_ref(first), &[])?;
    let mut changed = first.clone();
    changed.name = name("Renamed");
    changed.scheme = None;
    seed(store, std::slice::from_ref(&changed), &[])?;
    store.read(|tx| {
        assert_eq!(tx.exercise(first.id)?, Some(changed.clone()));
        Ok(())
    })
}

/**
 * A write whose closure fails leaves no trace: not its saves, not its
 * applied mark. The closure's error comes back unchanged.
 *
 * # Errors
 *
 * Any store error.
 *
 * # Panics
 *
 * If the store breaks the promise.
 */
pub fn a_failed_write_changes_nothing<S: Store>(store: &S) -> Result<(), EngineError> {
    let ids = SeqIds::new();
    let before = bench_session(&ids);
    seed(store, &[bench()], std::slice::from_ref(&before))?;

    let failure = EngineError::NotFound(Missing::Session(session_id(99)));
    let result: Result<(), EngineError> = store.write(|tx| {
        assert_eq!(tx.begin_command(command(1))?, None);
        tx.save_exercise(&squat())?;
        let mut changed = before.clone();
        changed.set_deload(true);
        tx.save_session(&changed)?;
        tx.finish_command(command(1), CommandOutcome::DeloadSet)?;
        Err(failure.clone())
    });
    assert_eq!(result, Err(failure));

    store.read(|tx| {
        assert_eq!(tx.exercise(squat().id)?, None);
        assert_eq!(tx.session(before.id())?, Some(before.clone()));
        Ok(())
    })?;
    store.write(|tx| {
        assert_eq!(
            tx.begin_command(command(1))?,
            None,
            "the command record was rolled back"
        );
        Ok(())
    })
}

/**
 * Inside a write, reads see the writes made earlier in the same closure.
 *
 * # Errors
 *
 * Any store error.
 *
 * # Panics
 *
 * If the store breaks the promise.
 */
pub fn a_write_sees_its_own_changes<S: Store>(store: &S) -> Result<(), EngineError> {
    let session = bench_session(&SeqIds::new());
    store.write(|tx| {
        tx.save_exercise(&bench())?;
        tx.save_session(&session)?;
        assert_eq!(tx.exercise(bench().id)?, Some(bench()));
        assert_eq!(tx.session(session.id())?, Some(session.clone()));
        assert_eq!(
            tx.sessions_containing(bench().id)?,
            [SessionHeader::of(&session)]
        );
        Ok(())
    })
}

/**
 * `begin_command` returns `None` for a command never finished, and the
 * outcome it was finished with ever after, within one write and across
 * writes (S-7).
 *
 * # Errors
 *
 * Any store error.
 *
 * # Panics
 *
 * If the store breaks the promise.
 */
pub fn a_command_is_applied_once<S: Store>(store: &S) -> Result<(), EngineError> {
    let started = CommandOutcome::SessionStarted(session_id(1));
    store.write(|tx| {
        assert_eq!(tx.begin_command(command(1))?, None);
        tx.finish_command(command(1), started)?;
        assert_eq!(tx.begin_command(command(1))?, Some(started));
        Ok(())
    })?;
    store.write(|tx| {
        assert_eq!(tx.begin_command(command(1))?, Some(started));
        assert_eq!(tx.begin_command(command(2))?, None);
        Ok(())
    })
}

/**
 * Every kind of [`CommandOutcome`], with its ID, is returned exactly as it
 * was recorded.
 *
 * # Errors
 *
 * Any store error.
 *
 * # Panics
 *
 * If the store breaks the promise.
 */
pub fn every_outcome_round_trips<S: Store>(store: &S) -> Result<(), EngineError> {
    let outcomes = [
        CommandOutcome::ExerciseCreated(bench().id),
        CommandOutcome::SessionStarted(session_id(1)),
        CommandOutcome::ExerciseAdded(entry_id(1)),
        CommandOutcome::SetLogged(SetId::from_uuid(Uuid::from_u128(0x5E7))),
        CommandOutcome::SessionFinished,
        CommandOutcome::DeloadSet,
    ];
    store.write(|tx| {
        for (n, outcome) in (1..).zip(outcomes) {
            tx.finish_command(command(n), outcome)?;
        }
        Ok(())
    })?;
    store.write(|tx| {
        for (n, outcome) in (1..).zip(outcomes) {
            assert_eq!(tx.begin_command(command(n))?, Some(outcome));
        }
        Ok(())
    })
}

/**
 * Saving a session again replaces the stored aggregate entirely: entries
 * and sets that are gone from the new value are gone from the store.
 *
 * # Errors
 *
 * Any store error.
 *
 * # Panics
 *
 * If the store breaks the promise.
 */
pub fn saving_a_session_replaces_it<S: Store>(store: &S) -> Result<(), EngineError> {
    let ids = SeqIds::new();
    let first = SessionBuilder::new(session_id(1), at(1_000))
        .entry(entry_id(1), &bench(), vec![working(kg(100, 0), 8); 3])
        .entry(entry_id(2), &squat(), [working(kg(140, 0), 5)])
        .build(&ids);
    let second = SessionBuilder::new(session_id(1), at(1_000))
        .entry(entry_id(1), &bench(), [working(kg(102, 50), 6)])
        .deload()
        .finished_at(at(9_000))
        .build(&ids);
    seed(store, &[bench(), squat()], &[first])?;
    seed(store, &[], std::slice::from_ref(&second))?;
    store.read(|tx| {
        assert_eq!(tx.session(session_id(1))?, Some(second.clone()));
        assert_eq!(tx.sessions_containing(squat().id)?, []);
        Ok(())
    })
}

/**
 * `sessions_containing` lists every session with the exercise, finished or
 * not and deload or not, once each, and nothing else.
 *
 * # Errors
 *
 * Any store error.
 *
 * # Panics
 *
 * If the store breaks the promise.
 */
pub fn sessions_containing_lists_every_match<S: Store>(store: &S) -> Result<(), EngineError> {
    let ids = SeqIds::new();
    let finished = bench_session(&ids);
    let squat_only = SessionBuilder::new(session_id(2), at(10_000))
        .entry(entry_id(2), &squat(), [working(kg(140, 0), 5)])
        .finished_at(at(11_000))
        .build(&ids);
    let twice_unfinished_deload = SessionBuilder::new(session_id(3), at(20_000))
        .entry(entry_id(3), &bench(), [working(kg(80, 0), 8)])
        .entry(entry_id(4), &bench(), [working(kg(60, 0), 12)])
        .deload()
        .build(&ids);
    seed(
        store,
        &[bench(), squat()],
        &[
            finished.clone(),
            squat_only.clone(),
            twice_unfinished_deload.clone(),
        ],
    )?;
    store.read(|tx| {
        let mut benches = tx.sessions_containing(bench().id)?;
        benches.sort_by_key(|h| h.id);
        assert_eq!(
            benches,
            [
                SessionHeader::of(&finished),
                SessionHeader::of(&twice_unfinished_deload)
            ]
        );
        assert_eq!(
            tx.sessions_containing(squat().id)?,
            [SessionHeader::of(&squat_only)]
        );
        let unknown = ExerciseId::from_uuid(Uuid::from_u128(404));
        assert_eq!(tx.sessions_containing(unknown)?, []);
        Ok(())
    })
}
