/*!
 * Records: the data behind the stats and medals page (product §3.5).
 *
 * One [`Record`] type covers every PR, so a single page can show a bench
 * 5RM, a 5 km time and a best VO2 max side by side.
 *
 * - The **subject** is open: every exercise, every activity kind and every
 *   measurement series (including custom metrics) can hold records.
 * - The **kind** is a closed enum: each kind is a rule with its own detector
 *   and its own explanation, and the compiler checks every rule handles it.
 *
 * Records are **derived data**. They are computed from sessions, activities
 * and measurements, keyed by [`ALGORITHM_VERSION`](crate::ALGORITHM_VERSION),
 * fully recomputable, and never synced or exported. This module defines the
 * types; the detectors arrive with the PR engine.
 */

use core::cmp::Ordering;

use thiserror::Error;

use crate::activity::ActivityKind;
use crate::health::{HeartRate, Vo2Max};
use crate::ids::{ActivityId, ExerciseId, MeasurementId, SessionId, SetId};
use crate::measurement::MeasurementKind;
use crate::metric::{Better, CustomValue};
use crate::time::Timestamp;
use crate::units::{Distance, DurationSecs, ElapsedTime, Mass, Reps};

/** A record's value: one variant per kind of number a record can hold. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Quantity {
    /** A mass: a load, an e1RM, a volume per set. */
    Mass(Mass),
    /** A rep count. */
    Reps(Reps),
    /** A set duration (a hold). */
    Duration(DurationSecs),
    /** An activity time. */
    Elapsed(ElapsedTime),
    /** A distance. */
    Distance(Distance),
    /** A plain count: sessions in a month, weeks in a streak. */
    Count(u32),
    /** A VO2 max. */
    Vo2Max(Vo2Max),
    /** A heart rate. */
    HeartRate(HeartRate),
    /** A custom-metric value. */
    Custom(CustomValue),
}

impl Quantity {
    /**
     * Compares two values of the same variant. Returns `None` when the
     * variants differ, because a mass and a distance can't be ranked.
     */
    pub fn compare(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::Mass(a), Self::Mass(b)) => Some(a.cmp(b)),
            (Self::Reps(a), Self::Reps(b)) => Some(a.cmp(b)),
            (Self::Duration(a), Self::Duration(b)) => Some(a.cmp(b)),
            (Self::Elapsed(a), Self::Elapsed(b)) => Some(a.cmp(b)),
            (Self::Distance(a), Self::Distance(b)) => Some(a.cmp(b)),
            (Self::Count(a), Self::Count(b)) => Some(a.cmp(b)),
            (Self::Vo2Max(a), Self::Vo2Max(b)) => Some(a.cmp(b)),
            (Self::HeartRate(a), Self::HeartRate(b)) => Some(a.cmp(b)),
            (Self::Custom(a), Self::Custom(b)) => Some(a.cmp(b)),
            _ => None,
        }
    }

    /**
     * Whether `self` beats `other` in the `better` direction. A tie is not
     * an improvement. `None` when the two can't be compared.
     */
    pub fn beats(&self, other: &Self, better: Better) -> Option<bool> {
        self.compare(other).map(|ordering| match better {
            Better::Higher => ordering == Ordering::Greater,
            Better::Lower => ordering == Ordering::Less,
        })
    }
}

/** What a record is about. Open-ended: any exercise, kind or series. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Subject {
    /** A specific exercise, such as bench press. */
    Exercise(ExerciseId),
    /** A kind of activity, such as running. */
    Activity(ActivityKind),
    /** A measurement series, built-in or custom. */
    Measurement(MeasurementKind),
    /** Training as a whole: streaks, busiest month. */
    Overall,
}

/** A standard race distance for "fastest time" records. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StandardDistance {
    /** One mile (1609.344 m). */
    Mile,
    /** 5 km. */
    FiveK,
    /** 10 km. */
    TenK,
    /** Half marathon (21.0975 km). */
    HalfMarathon,
    /** Marathon (42.195 km). */
    Marathon,
}

impl StandardDistance {
    /** The distance in millimetres. */
    pub const fn millimetres(self) -> u32 {
        match self {
            Self::Mile => 1_609_344,
            Self::FiveK => 5_000_000,
            Self::TenK => 10_000_000,
            Self::HalfMarathon => 21_097_500,
            Self::Marathon => 42_195_000,
        }
    }
}

/**
 * Which record this is. Closed: each kind is a rule with a detector.
 * Grouped as in the product spec §3.5. Ordered so records can be keyed by
 * `(Subject, RecordKind)` in a `BTreeMap`.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RecordKind {
    /* Strength */
    /** Heaviest load lifted for at least this many reps (1RM–20RM). */
    RepMax(Reps),
    /** Highest estimated 1RM (Epley, DD-13). */
    EstimatedOneRepMax,
    /**
     * Most reps in one set at exactly this mass, regardless of the unit it
     * was entered in.
     */
    MaxRepsAtLoad(Mass),
    /** Highest load × reps in one set. */
    BestSetVolume,
    /** Highest Σ load × reps for the exercise in one session. */
    BestSessionVolume,
    /** Heaviest load ever lifted, any reps. */
    HeaviestLoad,
    /** Most reps in one set (bodyweight). */
    MaxRepsInSet,
    /** Most total reps in one session (bodyweight). */
    MaxRepsInSession,
    /** Least assistance needed for at least this many reps. */
    LeastAssistance(Reps),
    /** Longest timed hold. */
    LongestHold,

    /* Endurance and carries */
    /** Fastest time over a standard distance (whole-activity level only). */
    FastestDistance(StandardDistance),
    /** Longest distance in one activity or carry. */
    LongestDistance,
    /** Longest single activity. */
    LongestDuration,
    /** Most distance in one calendar week. */
    BiggestWeek,
    /** Most distance in one calendar month. */
    BiggestMonth,

    /* Measurements */
    /** Best value of a measurement series, in its own better direction. */
    BestValue,

    /* Overall */
    /** Longest run of consecutive training weeks. */
    LongestStreakWeeks,
    /** Most sessions in one calendar month. */
    MostSessionsInMonth,
}

impl RecordKind {
    /**
     * The direction that beats this kind of record, when the kind alone
     * decides it. `None` for [`RecordKind::BestValue`], whose direction
     * comes from the measurement series or custom metric.
     */
    pub const fn fixed_better(self) -> Option<Better> {
        match self {
            Self::FastestDistance(_) | Self::LeastAssistance(_) => Some(Better::Lower),
            Self::BestValue => None,
            Self::RepMax(_)
            | Self::EstimatedOneRepMax
            | Self::MaxRepsAtLoad(_)
            | Self::BestSetVolume
            | Self::BestSessionVolume
            | Self::HeaviestLoad
            | Self::MaxRepsInSet
            | Self::MaxRepsInSession
            | Self::LongestHold
            | Self::LongestDistance
            | Self::LongestDuration
            | Self::BiggestWeek
            | Self::BiggestMonth
            | Self::LongestStreakWeeks
            | Self::MostSessionsInMonth => Some(Better::Higher),
        }
    }
}

/** Why a record was rejected. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RecordError {
    /** The `better` direction contradicts the record kind's fixed direction. */
    #[error("record kind {kind:?} is beaten by {expected:?} values, not {found:?}")]
    DirectionMismatch {
        /** The record kind. */
        kind: RecordKind,
        /** The direction the kind requires. */
        expected: Better,
        /** The direction that was given. */
        found: Better,
    },
    /** The previous value is a different kind of quantity than the record. */
    #[error("the previous value cannot be compared with the record value")]
    Incomparable,
}

/** What earned a record, so the UI can link back to it. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecordSource {
    /** A single set. */
    Set(SetId),
    /** A whole session. */
    Session(SessionId),
    /** An activity. */
    Activity(ActivityId),
    /** A measurement. */
    Measurement(MeasurementId),
    /** A span of training, such as a streak, ending at `achieved_at`. */
    Span,
}

/**
 * One record on the stats and medals page.
 *
 * Records are produced only by the record rules, never typed in, and any
 * edit to history recomputes them. Fields are private so [`Record::new`] can
 * check that the direction fits the kind and that the previous value is
 * comparable.
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    subject: Subject,
    kind: RecordKind,
    value: Quantity,
    better: Better,
    achieved_at: Timestamp,
    source: RecordSource,
    previous: Option<Quantity>,
    algorithm_version: u32,
}

impl Record {
    /**
     * Builds a record.
     *
     * # Errors
     *
     * - [`RecordError::DirectionMismatch`] if `kind` has a fixed direction
     *   and `better` differs from it.
     * - [`RecordError::Incomparable`] if `previous` is a different kind of
     *   quantity than `value`.
     */
    #[expect(
        clippy::too_many_arguments,
        reason = "a record has eight independent facts; a builder would add nothing the checks need"
    )]
    pub fn new(
        subject: Subject,
        kind: RecordKind,
        value: Quantity,
        better: Better,
        achieved_at: Timestamp,
        source: RecordSource,
        previous: Option<Quantity>,
        algorithm_version: u32,
    ) -> Result<Self, RecordError> {
        if let Some(expected) = kind.fixed_better()
            && expected != better
        {
            return Err(RecordError::DirectionMismatch {
                kind,
                expected,
                found: better,
            });
        }
        if let Some(previous) = &previous
            && value.compare(previous).is_none()
        {
            return Err(RecordError::Incomparable);
        }
        Ok(Self {
            subject,
            kind,
            value,
            better,
            achieved_at,
            source,
            previous,
            algorithm_version,
        })
    }

    /** What the record is about. */
    pub const fn subject(&self) -> Subject {
        self.subject
    }

    /** Which record it is. */
    pub const fn kind(&self) -> RecordKind {
        self.kind
    }

    /** The record value. */
    pub const fn value(&self) -> Quantity {
        self.value
    }

    /** Which direction counts as beating it. */
    pub const fn better(&self) -> Better {
        self.better
    }

    /** When it was set. */
    pub const fn achieved_at(&self) -> Timestamp {
        self.achieved_at
    }

    /** What earned it. */
    pub const fn source(&self) -> RecordSource {
        self.source
    }

    /** The record it replaced, if any, for "117.5 → 120 kg". */
    pub const fn previous(&self) -> Option<Quantity> {
        self.previous
    }

    /** The rule version that computed it. */
    pub const fn algorithm_version(&self) -> u32 {
        self.algorithm_version
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use uuid::Uuid;

    use super::*;
    use crate::time::UtcOffset;
    use crate::units::{Load, WeightUnit};

    fn secs(seconds: u32) -> Quantity {
        Quantity::Elapsed(ElapsedTime::new(seconds).unwrap())
    }

    #[test]
    fn faster_beats_slower_when_lower_is_better() {
        assert_eq!(secs(1_350).beats(&secs(1_400), Better::Lower), Some(true));
        assert_eq!(secs(1_400).beats(&secs(1_350), Better::Lower), Some(false));
    }

    #[test]
    fn heavier_beats_lighter_when_higher_is_better() {
        let reps = |n| Quantity::Reps(Reps::new(n).unwrap());
        assert_eq!(reps(9).beats(&reps(8), Better::Higher), Some(true));
    }

    #[test]
    fn a_tie_is_not_a_record() {
        assert_eq!(secs(1_350).beats(&secs(1_350), Better::Lower), Some(false));
    }

    #[test]
    fn different_quantities_cannot_be_compared() {
        assert_eq!(secs(1).compare(&Quantity::Count(1)), None);
    }

    #[test]
    fn standard_distances_are_exact() {
        assert_eq!(StandardDistance::Mile.millimetres(), 1_609_344);
        assert_eq!(StandardDistance::Marathon.millimetres(), 42_195_000);
    }

    #[test]
    fn records_can_key_a_btree_map() {
        let mut map = BTreeMap::new();
        let bench = Subject::Exercise(ExerciseId::from_uuid(Uuid::from_u128(1)));
        let load = Load::new(10_000, WeightUnit::Kilograms).unwrap();
        map.insert((bench, RecordKind::RepMax(Reps::new(5).unwrap())), 1_u8);
        map.insert((bench, RecordKind::MaxRepsAtLoad(load.mass())), 2);
        map.insert(
            (
                Subject::Activity(ActivityKind::Run),
                RecordKind::FastestDistance(StandardDistance::FiveK),
            ),
            3,
        );
        map.insert((bench, RecordKind::RepMax(Reps::new(5).unwrap())), 4);
        assert_eq!(map.len(), 3);
        assert_eq!(
            map.get(&(bench, RecordKind::RepMax(Reps::new(5).unwrap()))),
            Some(&4)
        );
    }

    #[test]
    fn max_reps_at_load_is_keyed_by_mass_not_entry() {
        let kg = Load::new(10_000, WeightUnit::Kilograms).unwrap();
        let same_kg = Load::new(10_000, WeightUnit::Kilograms).unwrap();
        let lb = Load::new(22_046, WeightUnit::Pounds).unwrap();
        assert_eq!(
            RecordKind::MaxRepsAtLoad(kg.mass()),
            RecordKind::MaxRepsAtLoad(same_kg.mass())
        );
        /* 220.46 lb is not exactly 100 kg, so it is a different load. */
        assert_ne!(
            RecordKind::MaxRepsAtLoad(kg.mass()),
            RecordKind::MaxRepsAtLoad(lb.mass())
        );
    }

    fn five_k(
        value: Quantity,
        better: Better,
        previous: Option<Quantity>,
    ) -> Result<Record, RecordError> {
        Record::new(
            Subject::Activity(ActivityKind::Run),
            RecordKind::FastestDistance(StandardDistance::FiveK),
            value,
            better,
            Timestamp::new(1_000, UtcOffset::UTC),
            RecordSource::Activity(ActivityId::from_uuid(Uuid::from_u128(7))),
            previous,
            crate::ALGORITHM_VERSION,
        )
    }

    #[test]
    fn rejects_a_direction_that_contradicts_the_kind() {
        assert_eq!(
            five_k(secs(1_350), Better::Higher, None),
            Err(RecordError::DirectionMismatch {
                kind: RecordKind::FastestDistance(StandardDistance::FiveK),
                expected: Better::Lower,
                found: Better::Higher,
            })
        );
    }

    #[test]
    fn rejects_an_incomparable_previous_value() {
        assert_eq!(
            five_k(secs(1_350), Better::Lower, Some(Quantity::Count(3))),
            Err(RecordError::Incomparable)
        );
    }

    #[test]
    fn a_valid_record_round_trips_through_getters() {
        let record = five_k(secs(1_350), Better::Lower, Some(secs(1_400))).unwrap();
        assert_eq!(record.subject(), Subject::Activity(ActivityKind::Run));
        assert_eq!(
            record.kind(),
            RecordKind::FastestDistance(StandardDistance::FiveK)
        );
        assert_eq!(record.value(), secs(1_350));
        assert_eq!(record.better(), Better::Lower);
        assert_eq!(record.achieved_at(), Timestamp::new(1_000, UtcOffset::UTC));
        assert_eq!(
            record.source(),
            RecordSource::Activity(ActivityId::from_uuid(Uuid::from_u128(7)))
        );
        assert_eq!(record.previous(), Some(secs(1_400)));
        assert_eq!(record.algorithm_version(), crate::ALGORITHM_VERSION);
    }

    #[test]
    fn best_value_direction_is_left_to_the_series() {
        assert_eq!(RecordKind::BestValue.fixed_better(), None);
        assert_eq!(
            RecordKind::HeaviestLoad.fixed_better(),
            Some(Better::Higher)
        );
        assert_eq!(
            RecordKind::LeastAssistance(Reps::new(5).unwrap()).fixed_better(),
            Some(Better::Lower)
        );
    }
}
