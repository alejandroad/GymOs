/*!
 * # gymos-testkit
 *
 * Shared test support for the GymOs crates: in-memory fakes of the engine
 * ports, a fixed clock, deterministic ID generators, readable builders and
 * store-generic contract scenarios.
 *
 * ## Rules for this crate
 *
 * - **Dev-only.** Depend on it from `[dev-dependencies]`, never from
 *   `[dependencies]`, so nothing in it can reach a release build.
 * - **Fakes must behave like the real store**, including rollback on error.
 *   Store-generic contract scenarios ([`scenarios`]) run against both to
 *   prove it.
 *
 * ## Quick start
 *
 * ```
 * use gymos_domain::TrackingMode;
 * use gymos_testkit::{SessionPlan, TestGym, kg, working};
 *
 * let gym = TestGym::new();
 * let bench = gym.create_exercise("Bench Press", TrackingMode::WeightReps, None);
 * let first = gym.log_session(&SessionPlan::new().exercise(bench, [working(kg(100, 0), 8)]));
 * let verdicts = gym.os().session_verdicts(first.session).unwrap();
 * assert_eq!(verdicts.tally.insufficient, 1);
 * ```
 */

mod builders;
mod fake_store;
mod fixed_clock;
mod gym;
pub mod scenarios;
mod seq_ids;

pub use builders::{SessionBuilder, SetBuilder, at, exercise, kg, lb, name, reps, rpe, working};
pub use fake_store::{FakeRead, FakeState, FakeStore, FakeWrite};
pub use fixed_clock::{FixedClock, SteppingClock};
pub use gym::{LoggedSession, SessionPlan, TestGym, TestPorts};
pub use seq_ids::SeqIds;
