/*!
 * Round-trip properties (S-11): any valid session survives save → load
 * unchanged, and saving a second version of it replaces the first exactly.
 * Strategies come from `gymos_testkit::strategies`.
 */

#![expect(clippy::unwrap_used, reason = "test setup; a panic is a failed test")]

#[path = "support/sqlite.rs"]
mod support;

use gymos_domain::{Exercise, Session};
use gymos_engine::{EngineError, ReadTx, Store, WriteTx};
use gymos_store::SqliteStore;
use gymos_testkit::strategies::session_with_exercises;
use proptest::prelude::*;
use support::memory;

fn save(store: &SqliteStore, exercises: &[Exercise], session: &Session) {
    store
        .write(|tx| -> Result<(), EngineError> {
            for exercise in exercises {
                tx.save_exercise(exercise)?;
            }
            tx.save_session(session)?;
            Ok(())
        })
        .unwrap();
}

fn load(
    store: &SqliteStore,
    exercises: &[Exercise],
    session: &Session,
) -> (Vec<Exercise>, Session) {
    store
        .read(|tx| {
            let mut loaded = Vec::new();
            for exercise in exercises {
                loaded.push(tx.exercise(exercise.id)?.unwrap());
            }
            Ok((loaded, tx.session(session.id())?.unwrap()))
        })
        .unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn any_valid_session_round_trips((exercises, session) in session_with_exercises()) {
        let store = memory();
        save(&store, &exercises, &session);
        let (loaded_exercises, loaded) = load(&store, &exercises, &session);
        prop_assert_eq!(loaded_exercises, exercises);
        prop_assert_eq!(loaded, session);
    }

    /* Both values share the session ID and overlapping entry and set IDs,
     * so the second save exercises upserts, tombstones and revivals. */
    #[test]
    fn saving_another_version_replaces_the_first(
        (first_exercises, first) in session_with_exercises(),
        (exercises, second) in session_with_exercises(),
    ) {
        let store = memory();
        save(&store, &first_exercises, &first);
        save(&store, &exercises, &second);
        let (loaded_exercises, loaded) = load(&store, &exercises, &second);
        prop_assert_eq!(loaded_exercises, exercises);
        prop_assert_eq!(loaded, second);
    }
}
