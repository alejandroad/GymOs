//! Logged sets.
//!
//! A set's measurements are an enum over tracking modes (DD-08), not a struct
//! full of optional fields. A weight-reps set *cannot* be missing its load,
//! and a plank *cannot* have reps.

use crate::effort::Effort;
use crate::exercise::TrackingMode;
use crate::ids::SetId;
use crate::text::Note;
use crate::units::{Distance, DurationSecs, Load, Reps};

/// The role a set plays in the workout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SetType {
    /// Preparation. Excluded from verdicts and PRs by default.
    Warmup,
    /// A normal working set.
    Working,
    /// A drop set: reduced load straight after a working set.
    Drop,
    /// Taken to muscular failure.
    Failure,
    /// A lighter back-off set after the top set.
    Backoff,
}

impl SetType {
    /// Whether this set counts toward verdicts and PRs by default
    /// (product spec §3.1: warm-ups don't).
    pub const fn counts_by_default(self) -> bool {
        match self {
            Self::Warmup => false,
            Self::Working | Self::Drop | Self::Failure | Self::Backoff => true,
        }
    }
}

/// What was measured in a set. One variant per [`TrackingMode`].
///
/// Body weight is not stored here. Bodyweight modes resolve it from the most
/// recent body-weight measurement on or before the session date (§3.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SetMetrics {
    /// Load × reps.
    WeightReps {
        /// The load lifted.
        load: Load,
        /// Reps completed.
        reps: Reps,
    },
    /// Reps at body weight.
    BodyweightReps {
        /// Reps completed.
        reps: Reps,
    },
    /// Reps at body weight plus an added load (belt, vest).
    WeightedBodyweight {
        /// Load added on top of body weight.
        added: Load,
        /// Reps completed.
        reps: Reps,
    },
    /// Reps at body weight minus assistance (band, machine).
    AssistedBodyweight {
        /// Load the assistance took off.
        assistance: Load,
        /// Reps completed.
        reps: Reps,
    },
    /// A timed hold.
    Duration {
        /// How long the hold lasted.
        duration: DurationSecs,
        /// Extra load, if the hold was loaded.
        load: Option<Load>,
    },
    /// A distance covered under load.
    DistanceLoad {
        /// Distance covered.
        distance: Distance,
        /// Load carried or pushed.
        load: Load,
        /// Time taken, if recorded.
        duration: Option<DurationSecs>,
    },
    /// Reps with no load.
    RepsOnly {
        /// Reps completed.
        reps: Reps,
    },
    /// Reps within a time window.
    TimeReps {
        /// Reps completed.
        reps: Reps,
        /// The time window.
        duration: DurationSecs,
    },
}

impl SetMetrics {
    /// The tracking mode these measurements belong to.
    pub const fn mode(&self) -> TrackingMode {
        match self {
            Self::WeightReps { .. } => TrackingMode::WeightReps,
            Self::BodyweightReps { .. } => TrackingMode::BodyweightReps,
            Self::WeightedBodyweight { .. } => TrackingMode::WeightedBodyweight,
            Self::AssistedBodyweight { .. } => TrackingMode::AssistedBodyweight,
            Self::Duration { .. } => TrackingMode::Duration,
            Self::DistanceLoad { .. } => TrackingMode::DistanceLoad,
            Self::RepsOnly { .. } => TrackingMode::RepsOnly,
            Self::TimeReps { .. } => TrackingMode::TimeReps,
        }
    }
}

/// One set as logged.
///
/// Fields are public: each is already a validated type and no rule ties them
/// together. "The metrics match the exercise's mode" is checked when the set
/// is added to a [`SessionExercise`](crate::SessionExercise).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggedSet {
    /// Stable identity.
    pub id: SetId,
    /// The role this set played.
    pub set_type: SetType,
    /// What was measured.
    pub metrics: SetMetrics,
    /// RPE or RIR, if the lifter recorded one.
    pub effort: Option<Effort>,
    /// A note on this set.
    pub note: Option<Note>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_warmups_are_excluded_by_default() {
        assert!(!SetType::Warmup.counts_by_default());
        assert!(SetType::Working.counts_by_default());
        assert!(SetType::Drop.counts_by_default());
        assert!(SetType::Failure.counts_by_default());
        assert!(SetType::Backoff.counts_by_default());
    }

    #[test]
    fn metrics_report_their_mode() {
        let reps = Reps::new(10).unwrap();
        assert_eq!(
            SetMetrics::BodyweightReps { reps }.mode(),
            TrackingMode::BodyweightReps
        );
        assert_eq!(
            SetMetrics::Duration {
                duration: DurationSecs::new(60).unwrap(),
                load: None
            }
            .mode(),
            TrackingMode::Duration
        );
    }
}
