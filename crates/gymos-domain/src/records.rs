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

use crate::activity::ActivityKind;
use crate::health::{HeartRate, Vo2Max};
use crate::ids::{ActivityId, ExerciseId, MeasurementId, SessionId, SetId};
use crate::measurement::MeasurementKind;
use crate::metric::{Better, CustomValue};
use crate::time::Timestamp;
use crate::units::{Distance, DurationSecs, ElapsedTime, Load, Mass, Reps};

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
 * Grouped as in the product spec §3.5.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecordKind {
    /* Strength */
    /** Heaviest load lifted for at least this many reps (1RM–20RM). */
    RepMax(Reps),
    /** Highest estimated 1RM (Epley, DD-13). */
    EstimatedOneRepMax,
    /** Most reps in one set at this load. */
    MaxRepsAtLoad(Load),
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
 * Fields are public: records are produced only by the record rules, never
 * typed in, and any edit to history recomputes them.
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /** What the record is about. */
    pub subject: Subject,
    /** Which record it is. */
    pub kind: RecordKind,
    /** The record value. */
    pub value: Quantity,
    /** Which direction counts as beating it. */
    pub better: Better,
    /** When it was set. */
    pub achieved_at: Timestamp,
    /** What earned it. */
    pub source: RecordSource,
    /** The record it replaced, if any, for "117.5 → 120 kg". */
    pub previous: Option<Quantity>,
    /** The rule version that computed it. */
    pub algorithm_version: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
