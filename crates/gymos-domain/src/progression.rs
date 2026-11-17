/*!
 * Progressive overload: did this exercise progress since last time?
 * (product §3.3, DD-13)
 *
 * Every rule here is a pure function of two logged exercises and an optional
 * progression scheme. Finding the baseline ("last time", skipping deloads)
 * needs history, so it is an engine query and not part of this module.
 */

mod compare;
mod e1rm;
mod pairing;
mod scheme;
mod signals;
mod verdict;

pub use e1rm::{E1RM_REP_CAP, E1rm};
pub use pairing::{Pair, Pairing, is_pairable, pair_sets};
pub use scheme::{ProgressionScheme, RepRange, SchemeError};
pub use signals::{Direction, Signal, Skipped, Volume};
pub use verdict::{
    ExerciseVerdict, SessionVerdict, Verdict, VerdictError, VerdictInput, exercise_verdict,
};
