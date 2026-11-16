/*!
 * Health values: body weight, VO2 max and heart rate.
 *
 * Like every other quantity, these are integers with checked constructors.
 * The ranges are wide enough for any real person and narrow enough to catch
 * a typo (a body weight of 8000, a heart rate of 2).
 */

use thiserror::Error;

use crate::units::{Load, Mass, NANOGRAMS_PER_KILOGRAM};

/** Why a health value was rejected. */
#[expect(
    clippy::enum_variant_names,
    reason = "each variant names the value that was out of range, as DD-07 errors do"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum HealthError {
    /** A body weight outside 20–400 kg. */
    #[error("body weight must be between 20 and 400 kg")]
    BodyWeightOutOfRange,
    /** A VO2 max outside 10.0–100.0 ml/kg/min. */
    #[error("VO2 max must be between 10.0 and 100.0, got {0} tenths")]
    Vo2MaxOutOfRange(u16),
    /** A heart rate outside 20–250 bpm. */
    #[error("heart rate must be between 20 and 250 bpm, got {0}")]
    HeartRateOutOfRange(u8),
}

/**
 * A person's body weight, kept exactly as entered (value and unit).
 *
 * Wraps a [`Load`] so it shares the exact nanogram arithmetic, with a range
 * that fits people rather than barbells.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BodyWeight(Load);

impl BodyWeight {
    /** The lightest body weight accepted: 20 kg. */
    pub const MIN: Mass = Mass::from_nanograms(20 * NANOGRAMS_PER_KILOGRAM);
    /** The heaviest body weight accepted: 400 kg. */
    pub const MAX: Mass = Mass::from_nanograms(400 * NANOGRAMS_PER_KILOGRAM);

    /**
     * Builds a body weight from an entered load.
     *
     * # Errors
     *
     * [`HealthError::BodyWeightOutOfRange`] outside 20–400 kg.
     */
    pub fn new(load: Load) -> Result<Self, HealthError> {
        let mass = load.mass();
        if mass < Self::MIN || mass > Self::MAX {
            return Err(HealthError::BodyWeightOutOfRange);
        }
        Ok(Self(load))
    }

    /** The weight as entered. */
    pub const fn load(self) -> Load {
        self.0
    }

    /** The exact mass, for comparisons and formulas. */
    pub fn mass(self) -> Mass {
        self.0.mass()
    }
}

/**
 * Maximal oxygen uptake in ml/kg/min, stored in tenths (52.3 is `523`).
 *
 * Entered by hand from a watch reading, or imported (product §3.6).
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Vo2Max(u16);

impl Vo2Max {
    /** 10.0 ml/kg/min, in tenths. */
    pub const MIN_TENTHS: u16 = 100;
    /** 100.0 ml/kg/min, in tenths. */
    pub const MAX_TENTHS: u16 = 1_000;

    /**
     * Builds a VO2 max from tenths of ml/kg/min.
     *
     * # Errors
     *
     * [`HealthError::Vo2MaxOutOfRange`] outside 100..=1000.
     */
    pub const fn from_tenths(tenths: u16) -> Result<Self, HealthError> {
        if tenths < Self::MIN_TENTHS || tenths > Self::MAX_TENTHS {
            return Err(HealthError::Vo2MaxOutOfRange(tenths));
        }
        Ok(Self(tenths))
    }

    /** The stored value in tenths. */
    pub const fn tenths(self) -> u16 {
        self.0
    }

    /** The value as a decimal, for display (`52.3`). */
    pub fn value(self) -> f64 {
        f64::from(self.0) / 10.0
    }
}

/** A heart rate in beats per minute. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HeartRate(u8);

impl HeartRate {
    /** The slowest heart rate accepted. */
    pub const MIN_BPM: u8 = 20;
    /** The fastest heart rate accepted. */
    pub const MAX_BPM: u8 = 250;

    /**
     * Builds a heart rate.
     *
     * # Errors
     *
     * [`HealthError::HeartRateOutOfRange`] outside 20..=250 bpm.
     */
    pub const fn from_bpm(bpm: u8) -> Result<Self, HealthError> {
        if bpm < Self::MIN_BPM || bpm > Self::MAX_BPM {
            return Err(HealthError::HeartRateOutOfRange(bpm));
        }
        Ok(Self(bpm))
    }

    /** Beats per minute. */
    pub const fn bpm(self) -> u8 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::WeightUnit::{Kilograms, Pounds};

    #[test]
    fn body_weight_keeps_the_entered_unit() {
        let weight = BodyWeight::new(Load::new(18_550, Pounds).unwrap()).unwrap();
        assert_eq!(weight.load().unit(), Pounds);
        assert_eq!(weight.load().hundredths(), 18_550);
    }

    #[test]
    fn body_weight_range_is_twenty_to_four_hundred_kg() {
        assert!(BodyWeight::new(Load::new(2_000, Kilograms).unwrap()).is_ok());
        assert!(BodyWeight::new(Load::new(40_000, Kilograms).unwrap()).is_ok());
        assert_eq!(
            BodyWeight::new(Load::new(1_999, Kilograms).unwrap()),
            Err(HealthError::BodyWeightOutOfRange)
        );
        assert_eq!(
            BodyWeight::new(Load::new(40_001, Kilograms).unwrap()),
            Err(HealthError::BodyWeightOutOfRange)
        );
    }

    #[test]
    fn vo2_max_is_stored_in_tenths() {
        let vo2 = Vo2Max::from_tenths(523).unwrap();
        assert!((vo2.value() - 52.3).abs() < 1e-9);
        assert_eq!(
            Vo2Max::from_tenths(99),
            Err(HealthError::Vo2MaxOutOfRange(99))
        );
        assert_eq!(
            Vo2Max::from_tenths(1_001),
            Err(HealthError::Vo2MaxOutOfRange(1_001))
        );
    }

    #[test]
    fn heart_rate_range() {
        assert_eq!(HeartRate::from_bpm(52).unwrap().bpm(), 52);
        assert!(HeartRate::from_bpm(19).is_err());
        assert!(HeartRate::from_bpm(251).is_err());
    }
}
