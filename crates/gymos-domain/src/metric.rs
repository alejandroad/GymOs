/*!
 * Custom metrics: the extension point for "whatever this athlete tracks".
 *
 * Built-in measurements (body weight, VO2 max, resting heart rate) have
 * their own types. Anything else (grip strength, vertical jump, waist size,
 * a mile time) is a [`CustomMetric`] the user defines once and then logs
 * values for. Every custom metric automatically gets a history chart and a
 * best-value record, because the metric says which direction is better.
 */

use thiserror::Error;

use crate::ids::MetricId;
use crate::text::{Name, UnitLabel};

/** Why a custom-metric value was rejected. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum MetricError {
    /** A value whose magnitude exceeds [`CustomValue::MAX_ABS_THOUSANDTHS`]. */
    #[error("custom metric value is out of range")]
    ValueOutOfRange,
    /** More display decimals than [`CustomMetric::MAX_DECIMALS`]. */
    #[error("a custom metric can show at most 3 decimals, got {0}")]
    TooManyDecimals(u8),
}

/** Which direction counts as an improvement. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Better {
    /** Bigger is better: VO2 max, a heavier lift, a longer run. */
    Higher,
    /** Smaller is better: resting heart rate, a faster time. */
    Lower,
}

/**
 * A value of a custom metric, stored exactly in thousandths of its unit
 * (`12.5` cm is `12_500`). Three decimals cover any hand-entered value.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CustomValue(i64);

impl CustomValue {
    /** The largest magnitude accepted, in thousandths (one trillion units). */
    pub const MAX_ABS_THOUSANDTHS: i64 = 1_000_000_000_000_000;

    /**
     * Builds a value from thousandths of the metric's unit. Negative values
     * are allowed (a temperature, a balance delta).
     *
     * # Errors
     *
     * [`MetricError::ValueOutOfRange`] beyond
     * ±[`CustomValue::MAX_ABS_THOUSANDTHS`].
     */
    pub const fn from_thousandths(thousandths: i64) -> Result<Self, MetricError> {
        if thousandths.unsigned_abs() > Self::MAX_ABS_THOUSANDTHS.unsigned_abs() {
            return Err(MetricError::ValueOutOfRange);
        }
        Ok(Self(thousandths))
    }

    /** The stored value in thousandths. */
    pub const fn thousandths(self) -> i64 {
        self.0
    }
}

/**
 * A metric the user defined, such as "Grip strength (kg), higher is better".
 *
 * Fields are public because every combination of valid values is valid.
 * The display precision is checked through [`CustomMetric::new`].
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomMetric {
    /** Stable identity. */
    pub id: MetricId,
    /** Display name, such as "Grip strength". */
    pub name: Name,
    /** Unit shown next to values, such as "kg". */
    pub unit: UnitLabel,
    /** Which direction is an improvement; drives the best-value record. */
    pub better: Better,
    decimals: u8,
}

impl CustomMetric {
    /** The most decimals a value can be shown with (matches storage). */
    pub const MAX_DECIMALS: u8 = 3;

    /**
     * Defines a metric.
     *
     * # Errors
     *
     * [`MetricError::TooManyDecimals`] if `decimals` exceeds
     * [`CustomMetric::MAX_DECIMALS`].
     */
    pub fn new(
        id: MetricId,
        name: Name,
        unit: UnitLabel,
        better: Better,
        decimals: u8,
    ) -> Result<Self, MetricError> {
        if decimals > Self::MAX_DECIMALS {
            return Err(MetricError::TooManyDecimals(decimals));
        }
        Ok(Self {
            id,
            name,
            unit,
            better,
            decimals,
        })
    }

    /** How many decimals to show values with. Display only; storage is exact. */
    pub const fn decimals(&self) -> u8 {
        self.decimals
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    #[test]
    fn values_are_exact_thousandths_and_may_be_negative() {
        assert_eq!(
            CustomValue::from_thousandths(-1_500).unwrap().thousandths(),
            -1_500
        );
        assert!(CustomValue::from_thousandths(CustomValue::MAX_ABS_THOUSANDTHS).is_ok());
        assert_eq!(
            CustomValue::from_thousandths(i64::MIN),
            Err(MetricError::ValueOutOfRange)
        );
    }

    #[test]
    fn display_decimals_are_capped_at_storage_precision() {
        let make = |decimals| {
            CustomMetric::new(
                MetricId::from_uuid(Uuid::from_u128(1)),
                Name::new("Grip strength").unwrap(),
                UnitLabel::new("kg").unwrap(),
                Better::Higher,
                decimals,
            )
        };
        assert_eq!(make(1).unwrap().decimals(), 1);
        assert_eq!(make(4), Err(MetricError::TooManyDecimals(4)));
    }
}
