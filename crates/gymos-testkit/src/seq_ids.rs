/*! A deterministic ID generator. */

use std::sync::atomic::{AtomicU64, Ordering};

use gymos_domain::Uuid;
use gymos_engine::IdGen;

/**
 * Hands out `prefix + 1`, `prefix + 2`, ... as UUIDs. IDs increase in
 * creation order, as UUID v7 does in production, so ties on `started_at`
 * break the same way.
 */
#[derive(Debug)]
pub struct SeqIds {
    prefix: u128,
    next: AtomicU64,
}

impl SeqIds {
    /** Starts at `00000000-0000-0000-0000-000000000001`. */
    pub const fn new() -> Self {
        Self::with_prefix(0)
    }

    /**
     * Starts at `prefix + 1`. Two generators with different prefixes never
     * collide, which keeps command IDs visibly apart from entity IDs.
     */
    pub const fn with_prefix(prefix: u128) -> Self {
        Self {
            prefix,
            next: AtomicU64::new(1),
        }
    }
}

impl Default for SeqIds {
    fn default() -> Self {
        Self::new()
    }
}

impl IdGen for SeqIds {
    fn next_id(&self) -> Uuid {
        let n = self.next.fetch_add(1, Ordering::Relaxed);
        Uuid::from_u128(self.prefix | u128::from(n))
    }
}
