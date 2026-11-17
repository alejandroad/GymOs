/*!
 * Ports: the traits the engine needs from the outside world (DD-03).
 *
 * The engine defines them and adapters implement them: `gymos-store` for
 * the database, the shell for the clock and ID generation, and
 * `gymos-testkit` for deterministic fakes. A [`Ports`] bundle names one
 * concrete implementation of each, so [`GymOs`](crate::GymOs) has a single
 * type parameter and every call is statically dispatched
 * (kernel-architecture §5.1).
 *
 * Every port is synchronous (§5.2) and takes `&self`. Interior mutability
 * belongs inside the adapter.
 */

mod clock;
mod ids;
mod store;

pub use clock::Clock;
pub use ids::IdGen;
pub use store::{ReadTx, SessionHeader, Store, WriteTx};

/**
 * One concrete implementation of every port.
 *
 * Only the shell (and test code) names concrete types:
 *
 * ```text
 * struct Desktop;
 * impl Ports for Desktop {
 *     type Store = SqliteStore;
 *     type Clock = SystemClock;
 *     type Ids = UuidV7;
 * }
 * type App = GymOs<Desktop>;
 * ```
 */
pub trait Ports {
    /** Where aggregates are loaded from and saved to. */
    type Store: Store;
    /** The source of "now". */
    type Clock: Clock;
    /** The source of fresh IDs. */
    type Ids: IdGen;
}
