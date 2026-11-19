/*! The clock port. */

use gymos_domain::Timestamp;

/**
 * The source of "now". The domain never reads the clock, and the engine
 * reads it only through this port, so time is deterministic in tests.
 */
pub trait Clock: Send + Sync {
    /** The current instant, with the user's local offset at that instant. */
    fn now(&self) -> Timestamp;
}
