/*!
 * Physical quantities: mass, load, reps, duration and distance.
 *
 * All quantities are integers, never floats, so records compare exactly and
 * nothing drifts when values round-trip through storage (product D10).
 *
 * Mass is stored in **nanograms** because both units convert to it exactly:
 * 0.01 kg is 10,000,000,000 ng and 0.01 lb is 4,535,923,700 ng (the pound is
 * defined as exactly 0.453 592 37 kg). So 225 lb and 102.06 kg are both
 * stored without rounding, and a PR tie is a true tie.
 */

use thiserror::Error;

/** Why a quantity was rejected. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum UnitError {
    /** Reps outside [`Reps::MIN`]..=[`Reps::MAX`]. */
    #[error("reps must be between 1 and 1000, got {0}")]
    RepsOutOfRange(u16),
    /** A load heavier than [`Load::MAX`]. */
    #[error("load must not exceed 1000 kg")]
    LoadTooHeavy,
    /** A duration outside [`DurationSecs::MIN`]..=[`DurationSecs::MAX`]. */
    #[error("duration must be between 1 second and 24 hours, got {0} s")]
    DurationOutOfRange(u32),
    /** A distance outside [`Distance::MIN_MM`]..=[`Distance::MAX_MM`]. */
    #[error("distance must be between 1 mm and 1000 km, got {0} mm")]
    DistanceOutOfRange(u32),
}

const NANOGRAMS_PER_KILOGRAM: i64 = 1_000_000_000_000;

/* Mass */

/** A mass in whole nanograms: the unit-independent value used to compare loads. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mass(i64);

impl Mass {
    /** No mass at all. */
    pub const ZERO: Self = Self(0);

    /** The exact value in nanograms. */
    pub const fn nanograms(self) -> i64 {
        self.0
    }

    /** The value in kilograms, for display and for formulas such as e1RM. */
    pub fn kilograms(self) -> f64 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "masses are capped at 1e15 ng, below 2^53, so the conversion is exact"
        )]
        let nanograms = self.0 as f64;
        nanograms / 1e12
    }
}

/* Load */

/** The unit a load was entered in. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeightUnit {
    /** Kilograms. */
    Kilograms,
    /** Pounds (international avoirdupois pound). */
    Pounds,
}

impl WeightUnit {
    const fn nanograms_per_hundredth(self) -> i64 {
        match self {
            Self::Kilograms => 10_000_000_000,
            Self::Pounds => 4_535_923_700,
        }
    }
}

/**
 * A load exactly as the user entered it: a value with two decimal places,
 * plus its unit.
 *
 * Equality means "entered the same way": 100 kg and 220.46 lb are not
 * equal. To ask which is heavier, compare [`Load::mass`].
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Load {
    hundredths: u32,
    unit: WeightUnit,
}

impl Load {
    /**
     * The heaviest load accepted: 1000 kg, well past any human lift, so a
     * typo such as 10000 is caught at entry.
     */
    pub const MAX: Mass = Mass(1_000 * NANOGRAMS_PER_KILOGRAM);

    /**
     * Builds a load from hundredths of a unit: `Load::new(10_250, Kilograms)`
     * is 102.50 kg.
     *
     * # Errors
     *
     * [`UnitError::LoadTooHeavy`] above [`Load::MAX`].
     */
    pub fn new(hundredths: u32, unit: WeightUnit) -> Result<Self, UnitError> {
        let nanograms = i64::from(hundredths)
            .checked_mul(unit.nanograms_per_hundredth())
            .ok_or(UnitError::LoadTooHeavy)?;
        if nanograms > Self::MAX.0 {
            return Err(UnitError::LoadTooHeavy);
        }
        Ok(Self { hundredths, unit })
    }

    /** The entered value in hundredths of [`Load::unit`]. */
    pub const fn hundredths(self) -> u32 {
        self.hundredths
    }

    /** The unit the value was entered in. */
    pub const fn unit(self) -> WeightUnit {
        self.unit
    }

    /** The exact mass, independent of the entry unit. */
    pub fn mass(self) -> Mass {
        // Cannot overflow: `new` guarantees the product is at most `MAX`.
        Mass(i64::from(self.hundredths) * self.unit.nanograms_per_hundredth())
    }
}

/* Reps, duration, distance */

/** The number of repetitions in a set. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Reps(u16);

impl Reps {
    /** The fewest reps a logged set can have. */
    pub const MIN: u16 = 1;
    /** The most reps a logged set can have. */
    pub const MAX: u16 = 1_000;

    /**
     * Builds a rep count.
     *
     * # Errors
     *
     * [`UnitError::RepsOutOfRange`] outside [`Reps::MIN`]..=[`Reps::MAX`].
     */
    pub const fn new(reps: u16) -> Result<Self, UnitError> {
        if reps < Self::MIN || reps > Self::MAX {
            return Err(UnitError::RepsOutOfRange(reps));
        }
        Ok(Self(reps))
    }

    /** The rep count. */
    pub const fn get(self) -> u16 {
        self.0
    }
}

/** A duration in whole seconds (a plank hold, a timed set). */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DurationSecs(u32);

impl DurationSecs {
    /** The shortest duration accepted. */
    pub const MIN: u32 = 1;
    /** The longest duration accepted: 24 hours. */
    pub const MAX: u32 = 24 * 60 * 60;

    /**
     * Builds a duration.
     *
     * # Errors
     *
     * [`UnitError::DurationOutOfRange`] outside
     * [`DurationSecs::MIN`]..=[`DurationSecs::MAX`].
     */
    pub const fn new(seconds: u32) -> Result<Self, UnitError> {
        if seconds < Self::MIN || seconds > Self::MAX {
            return Err(UnitError::DurationOutOfRange(seconds));
        }
        Ok(Self(seconds))
    }

    /** The duration in seconds. */
    pub const fn get(self) -> u32 {
        self.0
    }
}

/** A distance in whole millimetres (a farmer's carry, a sled push). */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Distance(u32);

impl Distance {
    /** The shortest distance accepted. */
    pub const MIN_MM: u32 = 1;
    /** The longest distance accepted: 1000 km. */
    pub const MAX_MM: u32 = 1_000_000_000;

    /**
     * Builds a distance from millimetres.
     *
     * # Errors
     *
     * [`UnitError::DistanceOutOfRange`] outside
     * [`Distance::MIN_MM`]..=[`Distance::MAX_MM`].
     */
    pub const fn from_millimetres(mm: u32) -> Result<Self, UnitError> {
        if mm < Self::MIN_MM || mm > Self::MAX_MM {
            return Err(UnitError::DistanceOutOfRange(mm));
        }
        Ok(Self(mm))
    }

    /** The distance in millimetres. */
    pub const fn millimetres(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::WeightUnit::{Kilograms, Pounds};
    use super::*;

    #[test]
    fn converts_pounds_to_mass_exactly() {
        let load = Load::new(22_500, Pounds).unwrap(); // 225 lb
        assert_eq!(load.mass().nanograms(), 102_058_283_250_000);
        assert!((load.mass().kilograms() - 102.058_283_25).abs() < 1e-12);
    }

    #[test]
    fn keeps_the_entered_value_and_unit() {
        let load = Load::new(10_250, Kilograms).unwrap();
        assert_eq!((load.hundredths(), load.unit()), (10_250, Kilograms));
    }

    #[test]
    fn equality_is_about_entry_but_mass_compares_weight() {
        let kg = Load::new(10_000, Kilograms).unwrap(); // 100 kg
        let lb = Load::new(22_046, Pounds).unwrap(); // 220.46 lb ≈ 99.9997 kg
        assert_ne!(kg, lb);
        assert!(lb.mass() < kg.mass());
    }

    #[test]
    fn accepts_zero_and_the_maximum_load() {
        assert_eq!(Load::new(0, Kilograms).unwrap().mass(), Mass::ZERO);
        assert_eq!(Load::new(100_000, Kilograms).unwrap().mass(), Load::MAX);
    }

    #[test]
    fn rejects_loads_above_the_maximum() {
        assert_eq!(Load::new(100_001, Kilograms), Err(UnitError::LoadTooHeavy));
        assert_eq!(Load::new(u32::MAX, Kilograms), Err(UnitError::LoadTooHeavy));
        assert_eq!(Load::new(220_463, Pounds), Err(UnitError::LoadTooHeavy));
    }

    #[test]
    fn validates_rep_range() {
        assert_eq!(Reps::new(0), Err(UnitError::RepsOutOfRange(0)));
        assert_eq!(Reps::new(1).unwrap().get(), 1);
        assert_eq!(Reps::new(1_000).unwrap().get(), 1_000);
        assert_eq!(Reps::new(1_001), Err(UnitError::RepsOutOfRange(1_001)));
    }

    #[test]
    fn validates_duration_and_distance_ranges() {
        assert!(DurationSecs::new(0).is_err());
        assert!(DurationSecs::new(DurationSecs::MAX).is_ok());
        assert!(DurationSecs::new(DurationSecs::MAX + 1).is_err());
        assert!(Distance::from_millimetres(0).is_err());
        assert!(Distance::from_millimetres(Distance::MAX_MM).is_ok());
        assert!(Distance::from_millimetres(Distance::MAX_MM + 1).is_err());
    }
}
