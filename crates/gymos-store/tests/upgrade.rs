/*!
 * Upgrade tests (DD-04, S-10): the committed fixture for schema v1 opens,
 * migrates to the latest schema, and reads back exactly what v1 wrote.
 *
 * `tests/fixtures/schema_v1.sqlite` was written by
 * `generate_schema_v1_fixture` below. Once v1 is released the fixture is
 * frozen: never regenerate it. Each new schema version adds its own
 * fixture and generator.
 *
 * The check regenerates the same history in memory with the current code
 * and compares the two through the store API, so it keeps working as the
 * schema changes underneath.
 */

#![expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test setup helpers; a panic is a failed test"
)]

#[path = "support/sqlite.rs"]
mod support;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use gymos_domain::progression::{ProgressionScheme, RepRange};
use gymos_domain::{
    Distance, DurationSecs, Effort, ExerciseId, ImportFormat, LoggedSet, Note, Rir, Session,
    SessionExercise, SessionExerciseId, SessionId, SetId, SetMetrics, SetType, Source, TemplateId,
    TrackingMode, Uuid,
};
use gymos_engine::{CommandId, CommandOutcome, ReadTx, StartSession, Store, WriteTx};
use gymos_store::{SqliteStore, StoreConfig};
use gymos_testkit::{SessionPlan, SetBuilder, TestGym, at, kg, lb, name, reps, rpe, working};
use support::{Shared, TempDb, memory_gym, user_version};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/schema_v1.sqlite")
}

/* ---- the v1 history ----------------------------------------------------- */

/** What the history created, so the two copies can be compared. */
struct History {
    exercises: Vec<ExerciseId>,
    sessions: Vec<SessionId>,
}

/** One set's measurements in every tracking mode. */
fn every_mode() -> [SetMetrics; 8] {
    let secs = |s| DurationSecs::new(s).unwrap();
    [
        SetMetrics::WeightReps {
            load: lb(225, 0),
            reps: reps(5),
        },
        SetMetrics::BodyweightReps { reps: reps(12) },
        SetMetrics::WeightedBodyweight {
            added: kg(20, 0),
            reps: reps(6),
        },
        SetMetrics::AssistedBodyweight {
            assistance: kg(15, 0),
            reps: reps(8),
        },
        SetMetrics::Duration {
            duration: secs(90),
            load: Some(kg(10, 0)),
        },
        SetMetrics::DistanceLoad {
            distance: Distance::from_millimetres(40_000).unwrap(),
            load: kg(32, 0),
            duration: Some(secs(45)),
        },
        SetMetrics::RepsOnly { reps: reps(20) },
        SetMetrics::TimeReps {
            reps: reps(15),
            duration: secs(60),
        },
    ]
}

const SET_TYPES: [SetType; 6] = [
    SetType::Warmup,
    SetType::Working,
    SetType::Drop,
    SetType::Failure,
    SetType::TopSet,
    SetType::Backoff,
];

fn rir(n: u8) -> Effort {
    Effort::Rir(Rir::new(n).unwrap())
}

/**
 * Every mode and set type, both effort scales, schemes, two templates, a
 * deload, an exercise twice in a session, an unfinished session, an import
 * and notes everywhere. Commands go through the engine; the notes, which
 * no command writes yet, are seeded straight into the store.
 */
fn seed<S: Store + Clone>(gym: &TestGym<S>) -> History {
    let scheme =
        ProgressionScheme::new(RepRange::new(reps(6), reps(8)).unwrap(), kg(2, 50)).unwrap();
    let lifts: Vec<(ExerciseId, SetMetrics)> = every_mode()
        .into_iter()
        .map(|metrics| {
            let with_scheme = matches!(metrics, SetMetrics::WeightReps { .. }).then_some(scheme);
            (
                gym.create_exercise("Lift", metrics.mode(), with_scheme),
                metrics,
            )
        })
        .collect();
    let template = |n: u128| TemplateId::from_uuid(Uuid::from_u128(0x7E << 64 | n));

    let mut every = SessionPlan::new().template(template(1));
    for &(exercise, metrics) in &lifts {
        let sets = SET_TYPES.iter().enumerate().map(|(i, &t)| {
            let effort = if i % 2 == 0 { rpe(17) } else { rir(2) };
            SetBuilder::new(metrics)
                .set_type(t)
                .effort(effort)
                .note("set note")
        });
        every = every.exercise(exercise, sets);
    }
    let bench = lifts.first().unwrap().0;
    let mut sessions = vec![
        gym.log_session(&every).session,
        gym.log_session(
            &SessionPlan::new()
                .template(template(1))
                .deload()
                .exercise(bench, vec![working(lb(135, 0), 8); 3]),
        )
        .session,
        gym.log_session(
            &SessionPlan::new()
                .template(template(2))
                .exercise(bench, vec![working(lb(225, 0), 5); 3])
                .exercise(bench, vec![working(lb(185, 0), 10); 2]),
        )
        .session,
        gym.log_session(
            &SessionPlan::new()
                .unfinished()
                .exercise(bench, [working(lb(230, 0), 5)]),
        )
        .session,
    ];

    let CommandOutcome::SessionStarted(imported) = gym
        .run(StartSession {
            started_at: Some(at(1_735_689_600_000)),
            template: None,
            title: Some(name("Imported from Strong")),
            source: Source::Import {
                format: ImportFormat::Strong,
                importer_version: 3,
            },
        })
        .unwrap()
    else {
        panic!("expected SessionStarted");
    };
    let entry = gym.add_exercise(imported, bench);
    gym.log_set(imported, entry, &working(lb(205, 50), 6));
    sessions.push(imported);

    sessions.push(seed_notes(gym, &lifts));
    History {
        exercises: lifts.into_iter().map(|(id, _)| id).collect(),
        sessions,
    }
}

/** A session with a title and notes on the session, an entry and a set. */
fn seed_notes<S: Store + Clone>(gym: &TestGym<S>, lifts: &[(ExerciseId, SetMetrics)]) -> SessionId {
    let fixed = |n: u128| Uuid::from_u128(0xF1 << 64 | n);
    let (exercise, metrics) = *lifts.last().unwrap();
    let mode: TrackingMode = metrics.mode();
    let mut session = Session::start(
        SessionId::from_uuid(fixed(1)),
        at(1_767_000_000_000),
        Source::Manual,
    );
    session.set_title(Some(name("Notes everywhere")));
    session.set_note(Some(Note::new("felt strong\nslept well").unwrap()));
    let carrier = gymos_testkit::exercise(exercise, "Lift", mode);
    let mut entry = SessionExercise::new(SessionExerciseId::from_uuid(fixed(2)), &carrier);
    entry.set_note(Some(Note::new("cue:\tbrace").unwrap()));
    entry
        .push_set(LoggedSet {
            id: SetId::from_uuid(fixed(3)),
            set_type: SetType::TopSet,
            metrics,
            effort: Some(rir(0)),
            note: Some(Note::new("ünïcödé").unwrap()),
        })
        .unwrap();
    session.add_exercise(entry).unwrap();
    session.finish(at(1_767_000_600_000)).unwrap();
    gym.store()
        .write(|tx| Ok(tx.save_session(&session)?))
        .unwrap();
    session.id()
}

/* ---- the fixture -------------------------------------------------------- */

/**
 * Writes the fixture. Does nothing unless `GYMOS_WRITE_V1_FIXTURE` is set,
 * so `cargo test -- --include-ignored` can't trip over it:
 * `GYMOS_WRITE_V1_FIXTURE=1 cargo test -p gymos-store --test upgrade -- --ignored`.
 */
#[test]
#[ignore = "writes tests/fixtures/schema_v1.sqlite; run only to create the fixture before v1 is released"]
fn generate_schema_v1_fixture() {
    if std::env::var_os("GYMOS_WRITE_V1_FIXTURE").is_none() {
        return;
    }
    let path = fixture();
    assert!(
        !path.exists(),
        "the fixture exists; delete it first only if v1 is not released yet"
    );
    let store = SqliteStore::open(&StoreConfig::new(&path, "unused")).unwrap();
    seed(&TestGym::with_store(Shared(Arc::new(store))));
    /* Fold the WAL into the main file so the fixture is one file. */
    rusqlite::Connection::open(&path)
        .unwrap()
        .pragma_update(None, "journal_mode", "DELETE")
        .unwrap();
}

#[test]
fn the_fixture_is_schema_v1() {
    assert_eq!(user_version(&fixture()), 1);
}

#[test]
fn the_v1_fixture_migrates_and_reads_back_what_v1_wrote() {
    /* Work on a copy: the committed fixture is never modified. */
    let db = TempDb::new();
    std::fs::copy(fixture(), &db.path).unwrap();
    let upgraded = TestGym::with_store(Shared(Arc::new(db.open())));
    let expected = memory_gym();
    let history = seed(&expected);

    for &exercise in &history.exercises {
        let read = |gym: &TestGym<Shared>| {
            gym.store()
                .read(|tx| Ok((tx.exercise(exercise)?, tx.sessions_containing(exercise)?)))
                .unwrap()
        };
        let (stored, containing) = read(&upgraded);
        assert!(stored.is_some(), "{exercise}");
        assert_eq!((stored, containing), read(&expected), "{exercise}");
    }
    for &session in &history.sessions {
        let read = |gym: &TestGym<Shared>| gym.store().read(|tx| Ok(tx.session(session)?)).unwrap();
        assert!(read(&upgraded).is_some(), "{session}");
        assert_eq!(read(&upgraded), read(&expected), "{session}");
        assert_eq!(
            upgraded.os().session_verdicts(session),
            expected.os().session_verdicts(session),
            "{session}"
        );
    }
    /* Every applied command replays to the same outcome. */
    let applied = support::count(&db.raw(), "applied_commands");
    assert!(applied > 40, "the history ran its commands: {applied}");
    for n in 1..=u128::try_from(applied).unwrap() + 1 {
        let id = CommandId::from_uuid(Uuid::from_u128(0xC << 124 | n));
        let outcome =
            |gym: &TestGym<Shared>| gym.store().write(|tx| Ok(tx.begin_command(id)?)).unwrap();
        assert_eq!(outcome(&upgraded), outcome(&expected), "command {n}");
    }
}
