/*!
 * Queries, one file per question. Each runs inside one `store.read` and
 * returns a view model built for the screen, never a stored aggregate the
 * caller could mistake for a write handle.
 */

mod last_time;
mod session_verdicts;

pub use last_time::{Baseline, BaselineRule};
pub use session_verdicts::{EntryOutcome, EntryVerdict, SessionVerdicts};

pub(crate) use last_time::last_time;
pub(crate) use session_verdicts::session_verdicts;
