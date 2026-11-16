/*!
 * Measurements: point-in-time readings about the person (product §3.6).
 *
 * A measurement is one entry in a time series. "Current body weight" is not
 * stored anywhere; it is simply the latest body-weight measurement. Keeping
 * every entry gives full-history charts (principle §1.2.2) and lets
 * bodyweight exercises use the weight *on the session date*.
 */

use crate::health::{BodyWeight, HeartRate, Vo2Max};
use crate::ids::{MeasurementId, MetricId};
use crate::metric::{Better, CustomValue};
use crate::provenance::Source;
use crate::text::Note;
use crate::time::Timestamp;

/**
 * What was measured. One variant per kind, each with exactly its own type,
 * like [`SetMetrics`](crate::SetMetrics) does for sets.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeasurementValue {
    /** Body weight, as entered. */
    BodyWeight(BodyWeight),
    /** VO2 max, typically read off a watch. */
    Vo2Max(Vo2Max),
    /** Resting heart rate. */
    RestingHeartRate(HeartRate),
    /** A value of a user-defined metric. */
    Custom {
        /** Which custom metric this value belongs to. */
        metric: MetricId,
        /** The value, in thousandths of the metric's unit. */
        value: CustomValue,
    },
}

impl MeasurementValue {
    /** The kind of measurement, for grouping into series and records. */
    pub const fn kind(&self) -> MeasurementKind {
        match self {
            Self::BodyWeight(_) => MeasurementKind::BodyWeight,
            Self::Vo2Max(_) => MeasurementKind::Vo2Max,
            Self::RestingHeartRate(_) => MeasurementKind::RestingHeartRate,
            Self::Custom { metric, .. } => MeasurementKind::Custom(*metric),
        }
    }
}

/** A measurement series: every value of one kind forms one chart. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MeasurementKind {
    /** Body weight. */
    BodyWeight,
    /** VO2 max. */
    Vo2Max,
    /** Resting heart rate. */
    RestingHeartRate,
    /** A user-defined metric. */
    Custom(MetricId),
}

impl MeasurementKind {
    /**
     * Which direction is an improvement, for built-in kinds.
     *
     * - Body weight returns `None`: whether up or down is better depends on
     *   the person's goal, so it gets a chart but no "best" record.
     * - Custom kinds return `None` here because the direction lives on the
     *   [`CustomMetric`](crate::CustomMetric) definition.
     */
    pub const fn built_in_better(self) -> Option<Better> {
        match self {
            Self::Vo2Max => Some(Better::Higher),
            Self::RestingHeartRate => Some(Better::Lower),
            Self::BodyWeight | Self::Custom(_) => None,
        }
    }
}

/**
 * One measurement.
 *
 * Fields are public: each is already validated and no rule ties them
 * together.
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Measurement {
    /** Stable identity. */
    pub id: MeasurementId,
    /** When the reading was taken. */
    pub measured_at: Timestamp,
    /** What was measured. */
    pub value: MeasurementValue,
    /** An optional note ("after fasting", "new scale"). */
    pub note: Option<Note>,
    /** Typed in, or imported. */
    pub source: Source,
}

/**
 * The body weight in effect at `at`: the latest body-weight measurement
 * taken on or before that instant (product §3.6). Used to resolve body
 * weight for bodyweight exercises on the session date.
 *
 * If several weigh-ins share the latest instant, the one with the larger
 * [`MeasurementId`] wins (ids are UUID v7, so that is the one recorded
 * last), making the result independent of slice order.
 *
 * Returns `None` if there is no body-weight entry that early.
 */
pub fn body_weight_at(measurements: &[Measurement], at: Timestamp) -> Option<BodyWeight> {
    measurements
        .iter()
        .filter(|m| !at.is_before(m.measured_at))
        .filter_map(|m| match m.value {
            MeasurementValue::BodyWeight(weight) => {
                Some(((m.measured_at.unix_millis(), m.id), weight))
            }
            MeasurementValue::Vo2Max(_)
            | MeasurementValue::RestingHeartRate(_)
            | MeasurementValue::Custom { .. } => None,
        })
        .max_by_key(|(key, _)| *key)
        .map(|(_, weight)| weight)
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;
    use crate::time::UtcOffset;
    use crate::units::{Load, WeightUnit};

    fn at(unix_millis: i64) -> Timestamp {
        Timestamp::new(unix_millis, UtcOffset::UTC)
    }

    fn weigh_in(id: u128, unix_millis: i64, kg_hundredths: u32) -> Measurement {
        let load = Load::new(kg_hundredths, WeightUnit::Kilograms).unwrap();
        Measurement {
            id: MeasurementId::from_uuid(Uuid::from_u128(id)),
            measured_at: at(unix_millis),
            value: MeasurementValue::BodyWeight(BodyWeight::new(load).unwrap()),
            note: None,
            source: Source::Manual,
        }
    }

    fn vo2(id: u128, unix_millis: i64) -> Measurement {
        Measurement {
            id: MeasurementId::from_uuid(Uuid::from_u128(id)),
            measured_at: at(unix_millis),
            value: MeasurementValue::Vo2Max(Vo2Max::from_tenths(500).unwrap()),
            note: None,
            source: Source::Manual,
        }
    }

    #[test]
    fn uses_the_latest_weigh_in_on_or_before_the_date() {
        let history = [
            weigh_in(1, 1_000, 8_000),
            weigh_in(2, 3_000, 8_200),
            weigh_in(3, 5_000, 8_400),
            vo2(4, 4_000),
        ];
        let weight = body_weight_at(&history, at(4_000)).unwrap();
        assert_eq!(weight.load().hundredths(), 8_200);
        let same_instant = body_weight_at(&history, at(5_000)).unwrap();
        assert_eq!(same_instant.load().hundredths(), 8_400);
    }

    #[test]
    fn has_no_body_weight_before_the_first_weigh_in() {
        let history = [weigh_in(1, 1_000, 8_000)];
        assert_eq!(body_weight_at(&history, at(999)), None);
    }

    #[test]
    fn kinds_group_series_and_know_their_direction() {
        let metric = MetricId::from_uuid(Uuid::from_u128(9));
        let custom = MeasurementValue::Custom {
            metric,
            value: CustomValue::from_thousandths(1).unwrap(),
        };
        assert_eq!(custom.kind(), MeasurementKind::Custom(metric));
        assert_eq!(
            MeasurementKind::Vo2Max.built_in_better(),
            Some(Better::Higher)
        );
        assert_eq!(
            MeasurementKind::RestingHeartRate.built_in_better(),
            Some(Better::Lower)
        );
        assert_eq!(MeasurementKind::BodyWeight.built_in_better(), None);
    }

    #[test]
    fn ties_at_the_same_instant_go_to_the_larger_id() {
        let earlier_id = weigh_in(1, 2_000, 8_000);
        let later_id = weigh_in(2, 2_000, 8_100);
        let forward = [earlier_id.clone(), later_id.clone()];
        let backward = [later_id, earlier_id];
        let expected = Some(8_100);
        let hundredths = |history: &[Measurement]| {
            body_weight_at(history, at(3_000)).map(|w| w.load().hundredths())
        };
        assert_eq!(hundredths(&forward), expected);
        assert_eq!(hundredths(&backward), expected);
    }
}
