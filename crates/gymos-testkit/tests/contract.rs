/*!
 * The store contract, run against the in-memory fake. The SQLite store
 * runs the same scenarios in its own tests.
 */

use gymos_engine::EngineError;
use gymos_testkit::{FakeStore, scenarios};

#[test]
fn missing_entities_read_as_none() -> Result<(), EngineError> {
    scenarios::missing_entities_read_as_none(&FakeStore::new())
}

#[test]
fn committed_writes_are_read_back_whole() -> Result<(), EngineError> {
    scenarios::committed_writes_are_read_back_whole(&FakeStore::new())
}

#[test]
fn every_field_round_trips() -> Result<(), EngineError> {
    scenarios::every_field_round_trips(&FakeStore::new())
}

#[test]
fn saving_an_exercise_replaces_it() -> Result<(), EngineError> {
    scenarios::saving_an_exercise_replaces_it(&FakeStore::new())
}

#[test]
fn a_failed_write_changes_nothing() -> Result<(), EngineError> {
    scenarios::a_failed_write_changes_nothing(&FakeStore::new())
}

#[test]
fn a_write_sees_its_own_changes() -> Result<(), EngineError> {
    scenarios::a_write_sees_its_own_changes(&FakeStore::new())
}

#[test]
fn a_command_is_applied_once() -> Result<(), EngineError> {
    scenarios::a_command_is_applied_once(&FakeStore::new())
}

#[test]
fn every_outcome_round_trips() -> Result<(), EngineError> {
    scenarios::every_outcome_round_trips(&FakeStore::new())
}

#[test]
fn saving_a_session_replaces_it() -> Result<(), EngineError> {
    scenarios::saving_a_session_replaces_it(&FakeStore::new())
}

#[test]
fn sessions_containing_lists_every_match() -> Result<(), EngineError> {
    scenarios::sessions_containing_lists_every_match(&FakeStore::new())
}

#[test]
fn run_all_covers_every_scenario() -> Result<(), EngineError> {
    scenarios::run_all(FakeStore::new)
}
