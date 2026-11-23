/*!
 * # gymos-testkit
 *
 * Shared test support for the GymOs crates: in-memory fakes of the engine
 * ports, a fixed clock, deterministic ID generators, readable builders,
 * store-generic contract scenarios and engine stories, and `proptest`
 * strategies for valid domain values.
 *
 * ## Rules for this crate
 *
 * - **Dev-only.** Depend on it from `[dev-dependencies]`, never from
 *   `[dependencies]`, so nothing in it can reach a release build.
 * - **Fakes must behave like the real store**, including rollback on error.
 *   Store-generic contract scenarios ([`scenarios`]) and engine stories
 *   ([`stories`], over a [`TestGym`] generic in its store) run against both
 *   to prove it.
 * - **Strategies only produce valid values** ([`strategies`]): they build
 *   everything through the domain's constructors.
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
pub mod stories;
pub mod strategies;

pub use builders::{SessionBuilder, SetBuilder, at, exercise, kg, lb, name, reps, rpe, working};
pub use fake_store::{FakeRead, FakeState, FakeStore, FakeWrite};
pub use fixed_clock::{FixedClock, SteppingClock};
pub use gym::{LoggedSession, SessionPlan, TestGym, TestPorts};
pub use seq_ids::SeqIds;
