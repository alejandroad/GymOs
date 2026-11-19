/*! Deterministic clocks. */

use std::sync::atomic::{AtomicI64, Ordering};

use gymos_domain::{Timestamp, UtcOffset};
use gymos_engine::Clock;

/** A clock that always returns the same instant. */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedClock(pub Timestamp);

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        self.0
    }
}

/**
 * A clock that starts at a given instant and moves forward by a fixed step
 * every time it is read, so consecutive commands get distinct, increasing
 * times.
 */
#[derive(Debug)]
pub struct SteppingClock {
    next_millis: AtomicI64,
    step_millis: i64,
    offset: UtcOffset,
}

impl SteppingClock {
    /** One hour, in milliseconds. */
    pub const HOUR: i64 = 3_600_000;

    /** Starts at `start` and advances `step_millis` after each read. */
    pub const fn new(start: Timestamp, step_millis: i64) -> Self {
        Self {
            next_millis: AtomicI64::new(start.unix_millis()),
            step_millis,
            offset: start.offset(),
        }
    }
}

impl Clock for SteppingClock {
    fn now(&self) -> Timestamp {
        let millis = self
            .next_millis
            .fetch_add(self.step_millis, Ordering::Relaxed);
        Timestamp::new(millis, self.offset)
    }
}
