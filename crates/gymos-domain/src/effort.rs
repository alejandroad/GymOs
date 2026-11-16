/*!
 * Perceived effort: RPE or RIR per set, and a 1–10 rating per activity.
 */

use thiserror::Error;

/** Why an effort rating was rejected. */
#[expect(
    clippy::enum_variant_names,
    reason = "each variant names the value that was out of range, as DD-07 errors do"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum EffortError {
    /** An RPE outside 1.0–10.0. */
    #[error("RPE must be between 1.0 and 10.0 (2 to 20 half-points), got {0} half-points")]
    RpeOutOfRange(u8),
    /** An RIR above [`Rir::MAX`]. */
    #[error("RIR must be between 0 and 10, got {0}")]
    RirOutOfRange(u8),
    /** A perceived effort outside 1–10. */
    #[error("perceived effort must be between 1 and 10, got {0}")]
    PerceivedEffortOutOfRange(u8),
}

/**
 * How hard a whole activity felt, 1 (very easy) to 10 (maximal).
 *
 * A coarser, whole-number scale than the per-set [`Rpe`], matching what
 * watches and activity logs record.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PerceivedEffort(u8);

impl PerceivedEffort {
    /** The easiest rating. */
    pub const MIN: u8 = 1;
    /** The hardest rating. */
    pub const MAX: u8 = 10;

    /**
     * Builds a rating.
     *
     * # Errors
     *
     * [`EffortError::PerceivedEffortOutOfRange`] outside 1..=10.
     */
    pub const fn new(rating: u8) -> Result<Self, EffortError> {
        if rating < Self::MIN || rating > Self::MAX {
            return Err(EffortError::PerceivedEffortOutOfRange(rating));
        }
        Ok(Self(rating))
    }

    /** The rating. */
    pub const fn get(self) -> u8 {
        self.0
    }
}

/**
 * Rate of perceived exertion, 1.0–10.0 in half-point steps.
 *
 * Stored as half-points (RPE 8.5 is `17`) so it stays an exact integer.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rpe(u8);

impl Rpe {
    /** RPE 1.0, in half-points. */
    pub const MIN_HALF_POINTS: u8 = 2;
    /** RPE 10.0, in half-points. */
    pub const MAX_HALF_POINTS: u8 = 20;

    /**
     * Builds an RPE from half-points: `Rpe::from_half_points(17)` is RPE 8.5.
     *
     * # Errors
     *
     * [`EffortError::RpeOutOfRange`] outside 2..=20.
     */
    pub const fn from_half_points(half_points: u8) -> Result<Self, EffortError> {
        if half_points < Self::MIN_HALF_POINTS || half_points > Self::MAX_HALF_POINTS {
            return Err(EffortError::RpeOutOfRange(half_points));
        }
        Ok(Self(half_points))
    }

    /** The stored value in half-points. */
    pub const fn half_points(self) -> u8 {
        self.0
    }

    /** The RPE as a decimal, for display (`8.5`). */
    pub fn value(self) -> f64 {
        f64::from(self.0) / 2.0
    }
}

/** Reps in reserve: how many more reps were left in the tank. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rir(u8);

impl Rir {
    /** The largest RIR accepted. Beyond this the estimate is meaningless. */
    pub const MAX: u8 = 10;

    /**
     * Builds an RIR.
     *
     * # Errors
     *
     * [`EffortError::RirOutOfRange`] above [`Rir::MAX`].
     */
    pub const fn new(reps_in_reserve: u8) -> Result<Self, EffortError> {
        if reps_in_reserve > Self::MAX {
            return Err(EffortError::RirOutOfRange(reps_in_reserve));
        }
        Ok(Self(reps_in_reserve))
    }

    /** The reps in reserve. */
    pub const fn get(self) -> u8 {
        self.0
    }
}

/** The effort a lifter recorded for a set: one scale or the other, never both. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Effort {
    /** Rated on the RPE scale. */
    Rpe(Rpe),
    /** Rated as reps in reserve. */
    Rir(Rir),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpe_uses_half_point_steps() {
        let rpe = Rpe::from_half_points(17).unwrap();
        assert_eq!(rpe.half_points(), 17);
        assert!((rpe.value() - 8.5).abs() < f64::EPSILON);
    }

    #[test]
    fn rpe_rejects_values_outside_one_to_ten() {
        assert_eq!(Rpe::from_half_points(1), Err(EffortError::RpeOutOfRange(1)));
        assert_eq!(
            Rpe::from_half_points(21),
            Err(EffortError::RpeOutOfRange(21))
        );
    }

    #[test]
    fn rir_accepts_zero_to_ten() {
        assert_eq!(Rir::new(0).unwrap().get(), 0);
        assert_eq!(Rir::new(10).unwrap().get(), 10);
        assert_eq!(Rir::new(11), Err(EffortError::RirOutOfRange(11)));
    }
}
