/*! The ID generator port. */

use gymos_domain::Uuid;

/**
 * The source of fresh IDs. The engine wraps each UUID in the right ID type
 * (`SessionId`, `SetId`, ...), so one generator serves every entity.
 *
 * Production uses UUID v7, which sorts by creation time. "Last time"
 * breaks `started_at` ties by `SessionId` (DD-15), so time-ordered IDs make
 * the later-created session win.
 */
pub trait IdGen: Send + Sync {
    /** A UUID never returned before by this generator. */
    fn next_id(&self) -> Uuid;
}
