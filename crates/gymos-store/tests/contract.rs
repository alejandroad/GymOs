/*!
 * The store contract (`gymos_testkit::scenarios`) on SQLite, in memory and
 * on a temporary file. The fake runs the same scenarios in the testkit, so
 * passing here proves the fake and the real store agree.
 */

#[path = "support/sqlite.rs"]
mod support;

use gymos_engine::EngineError;
use gymos_testkit::scenarios;
use support::{TempDb, memory};

/** One test per scenario per backing: `name::memory` and `name::file`. */
macro_rules! contract {
    ($($scenario:ident),* $(,)?) => {
        $(mod $scenario {
            use super::*;

            #[test]
            fn memory() -> Result<(), EngineError> {
                scenarios::$scenario(&support::memory())
            }

            #[test]
            fn file() -> Result<(), EngineError> {
                let db = TempDb::new();
                scenarios::$scenario(&db.open())
            }
        })*
    };
}

contract!(
    missing_entities_read_as_none,
    committed_writes_are_read_back_whole,
    every_field_round_trips,
    saving_an_exercise_replaces_it,
    a_failed_write_changes_nothing,
    a_write_sees_its_own_changes,
    a_command_is_applied_once,
    every_outcome_round_trips,
    saving_a_session_replaces_it,
    sessions_containing_lists_every_match,
);

#[test]
fn run_all_covers_every_scenario() -> Result<(), EngineError> {
    scenarios::run_all(memory)
}
