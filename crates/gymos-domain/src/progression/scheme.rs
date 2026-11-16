/*!
 * Progression schemes: the rep range an exercise is trained in and the load
 * step taken once the top of that range is reached (product §3.2).
 *
 * The overload rules need the scheme for one signal: `LoadStep` (DD-13
 * gap 1). Under double progression, 100 kg × 8 → 102.5 kg × 6 is progress,
 * even though the e1RM and the volume both fall. Where schemes are stored
 * (per exercise or per template) is decided with the engine.
 */

use thiserror::Error;

use crate::units::{Load, Mass, Reps};

/** Why a progression scheme was rejected. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum SchemeError {
    /** The bottom of the rep range is above the top. */
    #[error("rep range minimum {min} is above its maximum {max}")]
    InvertedRepRange {
        /** The rejected minimum. */
        min: u16,
        /** The rejected maximum. */
        max: u16,
    },
    /** A load step of zero, which could never move the load. */
    #[error("the load increment must be greater than zero")]
    ZeroIncrement,
}

/** An inclusive rep range such as 6–8. A single number (5–5) is allowed. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RepRange {
    min: Reps,
    max: Reps,
}

impl RepRange {
    /**
     * Builds a rep range.
     *
     * # Errors
     *
     * [`SchemeError::InvertedRepRange`] if `min` is above `max`.
     */
    pub const fn new(min: Reps, max: Reps) -> Result<Self, SchemeError> {
        if min.get() > max.get() {
            return Err(SchemeError::InvertedRepRange {
                min: min.get(),
                max: max.get(),
            });
        }
        Ok(Self { min, max })
    }

    /** The bottom of the range: the reps a new load must still reach. */
    pub const fn min(self) -> Reps {
        self.min
    }

    /** The top of the range: reaching it means adding load next time. */
    pub const fn max(self) -> Reps {
        self.max
    }

    /** Whether `reps` falls inside the range, both ends included. */
    pub const fn contains(self, reps: Reps) -> bool {
        reps.get() >= self.min.get() && reps.get() <= self.max.get()
    }
}

/**
 * How an exercise progresses: work up through `reps`, then add `increment`
 * and drop back to the bottom of the range (double progression).
 *
 * Fields are private so that a scheme always has a positive increment
 * (DD-08).
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProgressionScheme {
    reps: RepRange,
    increment: Load,
}

impl ProgressionScheme {
    /**
     * Builds a scheme.
     *
     * # Errors
     *
     * [`SchemeError::ZeroIncrement`] if `increment` is zero.
     */
    pub fn new(reps: RepRange, increment: Load) -> Result<Self, SchemeError> {
        if increment.mass() == Mass::ZERO {
            return Err(SchemeError::ZeroIncrement);
        }
        Ok(Self { reps, increment })
    }

    /** The rep range trained in. */
    pub const fn reps(self) -> RepRange {
        self.reps
    }

    /**
     * The planned load step. Any load increase counts toward `LoadStep`,
     * because micro-plates are real progress. The increment is what the
     * engine suggests next; it is not a threshold.
     */
    pub const fn increment(self) -> Load {
        self.increment
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::WeightUnit::Kilograms;

    fn reps(n: u16) -> Reps {
        Reps::new(n).unwrap()
    }

    #[test]
    fn accepts_ordered_and_single_value_ranges() {
        let range = RepRange::new(reps(6), reps(8)).unwrap();
        assert_eq!((range.min(), range.max()), (reps(6), reps(8)));
        assert!(RepRange::new(reps(5), reps(5)).is_ok());
    }

    #[test]
    fn rejects_inverted_ranges() {
        assert_eq!(
            RepRange::new(reps(8), reps(6)),
            Err(SchemeError::InvertedRepRange { min: 8, max: 6 })
        );
    }

    #[test]
    fn contains_both_ends() {
        let range = RepRange::new(reps(6), reps(8)).unwrap();
        assert!(!range.contains(reps(5)));
        assert!(range.contains(reps(6)));
        assert!(range.contains(reps(8)));
        assert!(!range.contains(reps(9)));
    }

    #[test]
    fn rejects_a_zero_increment() {
        let range = RepRange::new(reps(6), reps(8)).unwrap();
        let zero = Load::new(0, Kilograms).unwrap();
        assert_eq!(
            ProgressionScheme::new(range, zero),
            Err(SchemeError::ZeroIncrement)
        );
        let step = Load::new(250, Kilograms).unwrap();
        let scheme = ProgressionScheme::new(range, step).unwrap();
        assert_eq!((scheme.reps(), scheme.increment()), (range, step));
    }
}
