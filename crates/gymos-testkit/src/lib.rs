/*!
 * # gymos-testkit
 *
 * Shared test support for the GymOs crates: in-memory fakes of the engine
 * ports, a fixed clock, deterministic ID generators, readable builders and
 * property-test strategies.
 *
 * ## Rules for this crate
 *
 * - **Dev-only.** Depend on it from `[dev-dependencies]`, never from
 *   `[dependencies]`, so nothing in it can reach a release build.
 * - **Fakes must behave like the real store**, including rollback on error.
 *   Store-generic contract scenarios run against both to prove it.
 */
