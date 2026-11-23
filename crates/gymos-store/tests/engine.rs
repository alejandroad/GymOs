/*!
 * The engine over SQLite (S-9): the shared stories from
 * `gymos_testkit::stories`, plus what only a real transaction can show:
 * rollback across every table, panic safety, and agreement with the fake
 * on a seeded history.
 */

#![expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test setup helpers; a panic is a failed test"
)]

#[path = "support/sqlite.rs"]
mod support;

use std::panic::{AssertUnwindSafe, catch_unwind};

use gymos_domain::{ExerciseId, Source, TrackingMode};
use gymos_engine::{
    Command, CommandOutcome, DomainError, EngineError, FinishSession, LogSet, ReadTx, Store,
    WriteTx,
};
use gymos_testkit::{
    SessionPlan, SetBuilder, TestGym, at, exercise, kg, name, reps, stories, working,
};
use support::{Shared, TABLES, TempDb, count, file_gym, memory, memory_gym};

/* ---- the shared stories ------------------------------------------------- */

/** Each story on an in-memory store and on a file. */
macro_rules! stories {
    ($($story:ident),* $(,)?) => {
        $(mod $story {
            use super::*;

            #[test]
            fn memory() {
                stories::$story(&memory_gym());
            }

            #[test]
            fn file() {
                let db = TempDb::new();
                stories::$story(&file_gym(&db));
            }
        })*
    };
}

stories!(
    walking_skeleton_matches_golden_example_a,
    a_double_progression_cycle_reads_as_progress_until_it_stalls,
    a_deload_week_is_skipped_and_the_block_resumes_from_before_it,
    rotating_templates_each_progress_against_their_own_history,
    importing_an_older_session_moves_the_baseline_of_later_ones,
    a_replayed_command_never_changes_a_verdict,
);

/* ---- rollback under a real transaction ---------------------------------- */

fn row_counts(db: &TempDb) -> Vec<(&'static str, i64)> {
    let conn = db.raw();
    TABLES.iter().map(|&t| (t, count(&conn, t))).collect()
}

#[test]
fn a_command_that_fails_after_begin_command_leaves_no_row_anywhere() {
    let db = TempDb::new();
    let gym = file_gym(&db);
    let bench = gym.create_exercise("Bench Press", TrackingMode::WeightReps, None);
    let session = gym.start_session(None);
    let entry = gym.add_exercise(session, bench);
    gym.log_set(session, entry, &working(kg(100, 0), 8));
    let before = row_counts(&db);

    /* The session started at the clock's first reading; ending at 0 is
     * before that, so the domain refuses after the command was begun and
     * the session loaded. */
    let id = gym.command_id();
    let too_early = FinishSession {
        session,
        ended_at: Some(at(0)),
    };
    assert!(matches!(
        gym.os().execute(Command::new(id, too_early)),
        Err(EngineError::Domain(DomainError::Session(_)))
    ));
    /* A set in the wrong mode fails after the session is loaded, too. */
    let plank = LogSet {
        metrics: gymos_domain::SetMetrics::RepsOnly { reps: reps(10) },
        ..working(kg(100, 0), 8).log_into(session, entry)
    };
    assert!(gym.os().execute(Command::new(id, plank)).is_err());
    assert_eq!(
        row_counts(&db),
        before,
        "no table changed, applied_commands included"
    );

    /* Nothing of the failed attempts was recorded, so the ID is still free. */
    let fixed = working(kg(100, 0), 8).log_into(session, entry);
    let outcome = gym.os().execute(Command::new(id, fixed)).unwrap();
    assert!(matches!(outcome, CommandOutcome::SetLogged(_)));
}

#[test]
fn a_failed_first_command_writes_nothing_at_all() {
    let db = TempDb::new();
    let gym = file_gym(&db);
    let missing = gymos_domain::SessionId::from_uuid(gymos_domain::Uuid::from_u128(404));
    let result = gym.run(FinishSession {
        session: missing,
        ended_at: None,
    });
    assert!(matches!(result, Err(EngineError::NotFound(_))));
    assert!(
        row_counts(&db).iter().all(|&(_, n)| n == 0),
        "{:?}",
        row_counts(&db)
    );
}

/* ---- panic safety ------------------------------------------------------- */

#[test]
fn a_panic_inside_write_rolls_back_and_the_store_stays_usable() {
    let store = memory();
    let bench = exercise(
        ExerciseId::from_uuid(gymos_domain::Uuid::from_u128(1)),
        "Bench Press",
        TrackingMode::WeightReps,
    );

    let panicked = catch_unwind(AssertUnwindSafe(|| {
        store.write(|tx| -> Result<(), EngineError> {
            tx.save_exercise(&bench)?;
            panic!("a bug inside a command");
        })
    }));
    assert!(panicked.is_err());

    /* The save was rolled back, and the next transactions work. */
    store
        .read(|tx| {
            assert_eq!(tx.exercise(bench.id)?, None);
            Ok(())
        })
        .unwrap();
    store.write(|tx| Ok(tx.save_exercise(&bench)?)).unwrap();
    store
        .read(|tx| {
            assert_eq!(tx.exercise(bench.id)?, Some(bench.clone()));
            Ok(())
        })
        .unwrap();
}

#[test]
fn a_panic_inside_read_leaves_the_store_usable() {
    let store = memory();
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        store.read(|_| -> Result<(), EngineError> { panic!("a bug inside a query") })
    }));
    assert!(panicked.is_err());
    let bench = ExerciseId::from_uuid(gymos_domain::Uuid::from_u128(1));
    assert_eq!(store.read(|tx| Ok(tx.exercise(bench)?)).unwrap(), None);
}

/* ---- agreement with the fake -------------------------------------------- */

/**
 * A mixed history: several exercises, an exercise logged twice in one
 * session, a deload, an unfinished session, templates and an import.
 */
fn seed_history<S: Store + Clone>(gym: &TestGym<S>) -> Vec<ExerciseId> {
    let bench = gym.create_exercise("Bench Press", TrackingMode::WeightReps, None);
    let squat = gym.create_exercise("Squat", TrackingMode::WeightReps, None);
    let pull_up = gym.create_exercise("Pull-up", TrackingMode::BodyweightReps, None);
    let sets = |n: u16| vec![working(kg(100, 0), n); 3];
    let pulls =
        || vec![SetBuilder::new(gymos_domain::SetMetrics::BodyweightReps { reps: reps(8) }); 2];
    let template = gymos_domain::TemplateId::from_uuid(gymos_domain::Uuid::from_u128(7));
    gym.log_session(
        &SessionPlan::new()
            .exercise(bench, sets(5))
            .exercise(squat, sets(5)),
    );
    gym.log_session(
        &SessionPlan::new()
            .template(template)
            .exercise(bench, sets(6))
            .exercise(bench, sets(4)),
    );
    gym.log_session(
        &SessionPlan::new()
            .deload()
            .exercise(squat, sets(3))
            .exercise(pull_up, pulls()),
    );
    gym.log_session(&SessionPlan::new().unfinished().exercise(pull_up, pulls()));
    let imported = gym
        .run(gymos_engine::StartSession {
            started_at: Some(at(1_700_000_000_000)),
            template: None,
            title: Some(name("Imported")),
            source: Source::Import {
                format: gymos_domain::ImportFormat::Hevy,
                importer_version: 2,
            },
        })
        .unwrap();
    let CommandOutcome::SessionStarted(imported) = imported else {
        panic!("expected SessionStarted, got {imported:?}");
    };
    let entry = gym.add_exercise(imported, bench);
    gym.log_set(imported, entry, &working(kg(90, 0), 5));
    vec![bench, squat, pull_up]
}

#[test]
fn sessions_containing_matches_the_fake_on_a_seeded_history() {
    let fake = TestGym::new();
    let sqlite = memory_gym();
    let exercises = seed_history(&fake);
    assert_eq!(seed_history(&sqlite), exercises, "IDs are deterministic");

    for exercise in exercises {
        let mut expected = fake
            .store()
            .read(|tx| Ok(tx.sessions_containing(exercise)?))
            .unwrap();
        let mut actual = sqlite
            .store()
            .read(|tx| Ok(tx.sessions_containing(exercise)?))
            .unwrap();
        assert_ne!(actual, [], "every seeded exercise was logged");
        expected.sort_by_key(|h| h.id);
        actual.sort_by_key(|h| h.id);
        assert_eq!(actual, expected, "{exercise}");
        for header in &actual {
            assert_eq!(
                sqlite
                    .store()
                    .read(|tx| Ok(tx.session(header.id)?))
                    .unwrap(),
                fake.store().read(|tx| Ok(tx.session(header.id)?)).unwrap(),
            );
            assert_eq!(
                sqlite.os().session_verdicts(header.id),
                fake.os().session_verdicts(header.id),
            );
        }
    }
}

#[test]
fn shared_handles_see_the_same_database() {
    let gym = memory_gym();
    let bench = gym.create_exercise("Bench Press", TrackingMode::WeightReps, None);
    let handle: &Shared = gym.store();
    assert!(handle.read(|tx| Ok(tx.exercise(bench)?)).unwrap().is_some());
}
