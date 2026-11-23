/*!
 * How `save_session` writes an aggregate (S-3): removed children are
 * tombstoned, not deleted; a tombstoned ID that comes back is revived; and
 * an ID the store can't save without losing data is refused.
 */

#![expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test setup helpers; a panic is a failed test"
)]

#[path = "support/sqlite.rs"]
mod support;

use gymos_domain::{
    Exercise, ExerciseId, LoggedSet, Session, SessionExercise, SessionExerciseId, SessionId, SetId,
    Source, TrackingMode, Uuid,
};
use gymos_engine::{EngineError, ReadTx, RepoError, Store, WriteTx};
use gymos_store::SqliteStore;
use gymos_testkit::{at, exercise, kg, working};
use support::TempDb;

fn bench() -> Exercise {
    exercise(
        ExerciseId::from_uuid(Uuid::from_u128(1)),
        "Bench Press",
        TrackingMode::WeightReps,
    )
}

fn session_id(n: u128) -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(0x5E55 << 16 | n))
}

fn entry_id(n: u128) -> SessionExerciseId {
    SessionExerciseId::from_uuid(Uuid::from_u128(0xE << 16 | n))
}

fn set(n: u128) -> LoggedSet {
    working(kg(100, 0), 5).build(SetId::from_uuid(Uuid::from_u128(0x5E7 << 16 | n)))
}

/** A session with one bench entry per `(entry, sets)` pair. */
fn session(id: SessionId, entries: &[(SessionExerciseId, &[u128])]) -> Session {
    let mut session = Session::start(id, at(1_000), Source::Manual);
    for &(entry, sets) in entries {
        let mut e = SessionExercise::new(entry, &bench());
        for &n in sets {
            e.push_set(set(n)).unwrap();
        }
        session.add_exercise(e).unwrap();
    }
    session
}

fn save(store: &SqliteStore, session: &Session) -> Result<(), EngineError> {
    store.write(|tx| {
        tx.save_exercise(&bench())?;
        tx.save_session(session)?;
        Ok(())
    })
}

fn load(store: &SqliteStore, id: SessionId) -> Option<Session> {
    store.read(|tx| Ok(tx.session(id)?)).unwrap()
}

/** `(live, tombstoned)` row counts of `table`. */
fn rows(db: &TempDb, table: &str) -> (i64, i64) {
    let sql = match table {
        "sets" => {
            "SELECT count(*) FILTER (WHERE deleted = 0), count(*) FILTER (WHERE deleted = 1) FROM sets"
        }
        "session_entries" => {
            "SELECT count(*) FILTER (WHERE deleted = 0), count(*) FILTER (WHERE deleted = 1) FROM session_entries"
        }
        other => unreachable!("no such table in this test: {other}"),
    };
    db.raw()
        .query_row(sql, [], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
}

#[test]
fn removed_children_are_tombstoned_not_deleted() {
    let db = TempDb::new();
    let store = db.open();
    save(
        &store,
        &session(
            session_id(1),
            &[(entry_id(1), &[1, 2]), (entry_id(2), &[3])],
        ),
    )
    .unwrap();
    save(&store, &session(session_id(1), &[(entry_id(1), &[1])])).unwrap();

    assert_eq!(rows(&db, "session_entries"), (1, 1));
    assert_eq!(rows(&db, "sets"), (1, 2), "set 2 and entry 2's set 3");
}

#[test]
fn an_id_tombstoned_in_a_committed_write_is_revived() {
    let db = TempDb::new();
    let store = db.open();
    let full = session(session_id(1), &[(entry_id(1), &[1, 2])]);
    save(&store, &full).unwrap();
    save(&store, &session(session_id(1), &[])).unwrap();
    assert_eq!(rows(&db, "sets"), (0, 2));

    save(&store, &full).unwrap();
    assert_eq!(rows(&db, "sets"), (2, 0));
    assert_eq!(rows(&db, "session_entries"), (1, 0));
    assert_eq!(load(&store, session_id(1)), Some(full));
}

#[test]
fn a_set_may_move_between_entries_of_one_session() {
    let store = support::memory();
    save(
        &store,
        &session(session_id(1), &[(entry_id(1), &[1]), (entry_id(2), &[])]),
    )
    .unwrap();
    let moved = session(session_id(1), &[(entry_id(1), &[]), (entry_id(2), &[1])]);
    save(&store, &moved).unwrap();
    assert_eq!(load(&store, session_id(1)), Some(moved));
}

fn refused(result: Result<(), EngineError>) -> String {
    match result {
        Err(EngineError::Repo(RepoError::Backend(detail))) => detail,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_set_id_used_twice_in_a_session_is_refused() {
    let store = support::memory();
    /* The domain allows it (push_set doesn't check set IDs); one row can't
     * hold two sets, so the store refuses rather than drop one. */
    let twice = session(session_id(1), &[(entry_id(1), &[1]), (entry_id(2), &[1])]);
    assert!(refused(save(&store, &twice)).contains("appears twice"));
    assert_eq!(load(&store, session_id(1)), None, "nothing was written");
}

#[test]
fn an_entry_of_another_session_is_refused_not_moved() {
    let store = support::memory();
    let first = session(session_id(1), &[(entry_id(1), &[1])]);
    save(&store, &first).unwrap();

    let thief = session(session_id(2), &[(entry_id(1), &[2])]);
    assert!(refused(save(&store, &thief)).contains("another session"));
    assert_eq!(load(&store, session_id(1)), Some(first));
    assert_eq!(load(&store, session_id(2)), None);
}

#[test]
fn a_set_of_another_session_is_refused_not_moved() {
    let store = support::memory();
    let first = session(session_id(1), &[(entry_id(1), &[1])]);
    save(&store, &first).unwrap();

    let thief = session(session_id(2), &[(entry_id(2), &[1])]);
    assert!(refused(save(&store, &thief)).contains("another session"));
    assert_eq!(load(&store, session_id(1)), Some(first));
}

#[test]
fn a_tombstoned_entry_still_belongs_to_its_session() {
    let store = support::memory();
    save(&store, &session(session_id(1), &[(entry_id(1), &[1])])).unwrap();
    save(&store, &session(session_id(1), &[])).unwrap();

    /* Removed from session 1, but the ID is still session 1's: reusing it
     * elsewhere would rewrite history that sync may need to replay. */
    let reuse = session(session_id(2), &[(entry_id(1), &[])]);
    assert!(refused(save(&store, &reuse)).contains("another session"));
}
