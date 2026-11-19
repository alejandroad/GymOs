/*!
 * Command orchestration on `GymOs<TestPorts>`: what each command writes,
 * idempotency (D-5) and rollback (kernel-architecture §5.3).
 */

#![expect(
    clippy::unwrap_used,
    reason = "test helpers build fixtures from known-valid literals; a panic is a failed test"
)]

use gymos_domain::{
    DurationSecs, ExerciseId, SessionError, SetMetrics, SetType, Source, TrackingMode, Uuid,
};
use gymos_engine::{
    AddExercise, Command, CommandOutcome, CreateExercise, DomainError, EngineError, FinishSession,
    LogSet, Missing, SetDeload, StartSession,
};
use gymos_testkit::{SessionPlan, TestGym, at, kg, name, working};

fn plank_set() -> SetMetrics {
    SetMetrics::Duration {
        duration: DurationSecs::new(60).unwrap(),
        load: None,
    }
}

#[test]
fn commands_build_a_session_through_the_domain() {
    let gym = TestGym::new();
    let bench = gym.create_exercise("Bench Press", TrackingMode::WeightReps, None);
    let session = gym.start_session(None);
    let entry = gym.add_exercise(session, bench);
    let set = gym.log_set(
        session,
        entry,
        &working(kg(100, 0), 8).set_type(SetType::TopSet),
    );
    gym.set_deload(session, true);
    gym.finish(session);

    let state = gym.store().snapshot();
    assert_eq!(state.exercises()[&bench].name, name("Bench Press"));
    let stored = &state.sessions()[&session];
    assert_eq!(stored.started_at(), TestGym::START);
    assert!(
        stored
            .ended_at()
            .is_some_and(|end| TestGym::START.is_before(end))
    );
    assert!(stored.is_deload());
    let logged = stored.entry(entry).unwrap();
    assert_eq!(logged.exercise(), bench);
    assert_eq!(logged.sets().len(), 1);
    assert_eq!(logged.sets()[0].id, set);
    assert_eq!(logged.sets()[0].set_type, SetType::TopSet);
    assert_eq!(state.applied().len(), 6);
}

#[test]
fn explicit_times_override_the_clock() {
    let gym = TestGym::new();
    let outcome = gym
        .run(StartSession {
            started_at: Some(at(1_000)),
            template: None,
            title: Some(name("Imported")),
            source: Source::Manual,
        })
        .unwrap();
    let CommandOutcome::SessionStarted(session) = outcome else {
        panic!("expected SessionStarted, got {outcome:?}");
    };
    gym.run(FinishSession {
        session,
        ended_at: Some(at(4_000)),
    })
    .unwrap();
    let state = gym.store().snapshot();
    let stored = &state.sessions()[&session];
    assert_eq!(
        (stored.started_at(), stored.ended_at()),
        (at(1_000), Some(at(4_000)))
    );
    assert_eq!(stored.title(), Some(&name("Imported")));
}

#[test]
fn finished_sessions_accept_more_sets() {
    let gym = TestGym::new();
    let bench = gym.create_exercise("Bench Press", TrackingMode::WeightReps, None);
    let logged = gym.log_session(&SessionPlan::new().exercise(bench, [working(kg(100, 0), 8)]));
    gym.log_set(logged.session, logged.entries[0], &working(kg(100, 0), 7));
    let state = gym.store().snapshot();
    let stored = &state.sessions()[&logged.session];
    assert!(stored.ended_at().is_some());
    assert_eq!(stored.entry(logged.entries[0]).unwrap().sets().len(), 2);
}

/* ---- idempotency (D-5) -------------------------------------------------- */

#[test]
fn a_replayed_command_is_already_applied_and_changes_nothing() {
    let gym = TestGym::new();
    let bench = gym.create_exercise("Bench Press", TrackingMode::WeightReps, None);
    let session = gym.start_session(None);
    let entry = gym.add_exercise(session, bench);

    let log = Command::new(
        gym.command_id(),
        working(kg(100, 0), 8).log_into(session, entry),
    );
    let first = gym.os().execute(log.clone()).unwrap();
    assert!(matches!(first, CommandOutcome::SetLogged(_)));
    let after_first = gym.store().snapshot();

    assert_eq!(
        gym.os().execute(log).unwrap(),
        CommandOutcome::AlreadyApplied
    );
    assert_eq!(gym.store().snapshot(), after_first);
    assert_eq!(
        after_first.sessions()[&session]
            .entry(entry)
            .unwrap()
            .sets()
            .len(),
        1
    );
}

#[test]
fn every_command_kind_is_idempotent() {
    let gym = TestGym::new();
    let bench = gym.create_exercise("Bench Press", TrackingMode::WeightReps, None);
    let session = gym.start_session(None);
    let entry = gym.add_exercise(session, bench);
    let commands = [
        Command::new(
            gym.command_id(),
            CreateExercise {
                name: name("Squat"),
                mode: TrackingMode::WeightReps,
                scheme: None,
            },
        ),
        Command::new(
            gym.command_id(),
            StartSession {
                started_at: None,
                template: None,
                title: None,
                source: Source::Manual,
            },
        ),
        Command::new(
            gym.command_id(),
            AddExercise {
                session,
                exercise: bench,
            },
        ),
        Command::new(
            gym.command_id(),
            working(kg(100, 0), 8).log_into(session, entry),
        ),
        Command::new(
            gym.command_id(),
            SetDeload {
                session,
                deload: true,
            },
        ),
        Command::new(
            gym.command_id(),
            FinishSession {
                session,
                ended_at: None,
            },
        ),
    ];
    for command in commands {
        let outcome = gym.os().execute(command.clone()).unwrap();
        assert_ne!(outcome, CommandOutcome::AlreadyApplied, "{command:?}");
        let before = gym.store().snapshot();
        assert_eq!(
            gym.os().execute(command.clone()).unwrap(),
            CommandOutcome::AlreadyApplied,
            "{command:?}"
        );
        assert_eq!(gym.store().snapshot(), before, "{command:?}");
    }
}

/* ---- rollback ----------------------------------------------------------- */

#[test]
fn a_failed_command_leaves_the_store_unchanged_and_can_be_retried() {
    let gym = TestGym::new();
    let bench = gym.create_exercise("Bench Press", TrackingMode::WeightReps, None);
    let session = gym.start_session(None);
    let entry = gym.add_exercise(session, bench);
    let before = gym.store().snapshot();

    /* mark_applied has already written when push_set refuses the set. */
    let id = gym.command_id();
    let wrong_mode = LogSet {
        metrics: plank_set(),
        ..working(kg(100, 0), 8).log_into(session, entry)
    };
    assert_eq!(
        gym.os().execute(Command::new(id, wrong_mode)),
        Err(EngineError::Domain(DomainError::Session(
            SessionError::ModeMismatch {
                expected: TrackingMode::WeightReps,
                found: TrackingMode::Duration,
            }
        )))
    );
    assert_eq!(gym.store().snapshot(), before);

    /* The applied mark was rolled back too, so the same ID still works. */
    let fixed = working(kg(100, 0), 8).log_into(session, entry);
    let outcome = gym.os().execute(Command::new(id, fixed)).unwrap();
    assert!(matches!(outcome, CommandOutcome::SetLogged(_)));
}

#[test]
fn ending_before_the_start_is_refused() {
    let gym = TestGym::new();
    let session = gym.start_session(None);
    let before = gym.store().snapshot();
    let result = gym.run(FinishSession {
        session,
        ended_at: Some(at(0)),
    });
    assert_eq!(
        result,
        Err(EngineError::Domain(DomainError::Session(
            SessionError::EndsBeforeStart
        )))
    );
    assert_eq!(gym.store().snapshot(), before);
}

#[test]
fn missing_entities_are_not_found_and_change_nothing() {
    let gym = TestGym::new();
    let bench = gym.create_exercise("Bench Press", TrackingMode::WeightReps, None);
    let session = gym.start_session(None);
    let entry = gym.add_exercise(session, bench);
    let before = gym.store().snapshot();

    let nowhere = gymos_domain::SessionId::from_uuid(Uuid::from_u128(404));
    let no_exercise = ExerciseId::from_uuid(Uuid::from_u128(404));
    let no_entry = gymos_domain::SessionExerciseId::from_uuid(Uuid::from_u128(404));

    assert_eq!(
        gym.run(AddExercise {
            session,
            exercise: no_exercise
        }),
        Err(EngineError::NotFound(Missing::Exercise(no_exercise)))
    );
    assert_eq!(
        gym.run(AddExercise {
            session: nowhere,
            exercise: bench
        }),
        Err(EngineError::NotFound(Missing::Session(nowhere)))
    );
    assert_eq!(
        gym.run(working(kg(100, 0), 8).log_into(session, no_entry)),
        Err(EngineError::NotFound(Missing::Entry {
            session,
            entry: no_entry
        }))
    );
    assert_eq!(
        gym.run(working(kg(100, 0), 8).log_into(nowhere, entry)),
        Err(EngineError::NotFound(Missing::Session(nowhere)))
    );
    assert_eq!(
        gym.run(SetDeload {
            session: nowhere,
            deload: true
        }),
        Err(EngineError::NotFound(Missing::Session(nowhere)))
    );
    assert_eq!(
        gym.run(FinishSession {
            session: nowhere,
            ended_at: None
        }),
        Err(EngineError::NotFound(Missing::Session(nowhere)))
    );
    assert_eq!(gym.store().snapshot(), before);
}
