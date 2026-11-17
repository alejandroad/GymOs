/*!
 * Verdicts: the answer to "did I progress?" for one exercise, and the counts
 * for a whole session (DD-13).
 *
 * A verdict is one [`Verdict`] plus every [`Signal`] that fired, positive and
 * negative. The yes/no answer is derived from it
 * ([`ExerciseVerdict::is_progressive_overload`]). A stored boolean would
 * lose first sessions, deloads, matched sessions and mixed results.
 */

use thiserror::Error;

use super::scheme::ProgressionScheme;
use super::signals::{Detection, Direction, Signal, Skipped, WeightRepsSet, detect_weight_reps};
use crate::ALGORITHM_VERSION;
use crate::exercise::TrackingMode;
use crate::ids::ExerciseId;
use crate::session::SessionExercise;
use crate::set::SetMetrics;

/** Why a verdict could not be computed. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum VerdictError {
    /** Overload rules for this tracking mode don't exist yet. */
    #[error("overload verdicts are not defined for {0:?} exercises yet")]
    UnsupportedMode(TrackingMode),
    /** The baseline is a different exercise, so nothing can be compared. */
    #[error("baseline exercise {baseline:?} is not the current exercise {current:?}")]
    ExerciseMismatch {
        /** The baseline's exercise. */
        baseline: ExerciseId,
        /** The current exercise. */
        current: ExerciseId,
    },
    /** The baseline was tracked in another mode. */
    #[error("baseline is tracked as {baseline:?} but the current exercise as {current:?}")]
    ModeMismatch {
        /** The baseline's tracking mode. */
        baseline: TrackingMode,
        /** The current tracking mode. */
        current: TrackingMode,
    },
}

/** The outcome for one exercise in one session. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Verdict {
    /** At least one signal improved ("any one counts", §3.3). */
    Progressed,
    /** Nothing changed. */
    Matched,
    /** Nothing improved and something got worse. */
    Regressed,
    /** There is nothing to compare: a first session, or no counted sets. */
    InsufficientData,
    /** The session is a deload, which is never a regression. */
    Deload,
}

/** The verdict for one exercise, with everything needed to explain it. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExerciseVerdict {
    /** The outcome. */
    pub verdict: Verdict,
    /** Every signal that fired, strongest kind first. */
    pub signals: Vec<Signal>,
    /** Rules that could not be evaluated, such as e1RM above the rep cap. */
    pub skipped: Vec<Skipped>,
    /** The rules version this was computed with. */
    pub algorithm_version: u32,
}

impl ExerciseVerdict {
    /** Whether this counts as progressive overload: exactly `Progressed`. */
    pub fn is_progressive_overload(&self) -> bool {
        self.verdict == Verdict::Progressed
    }
}

/** What to compare. */
#[derive(Debug, Clone, Copy)]
pub struct VerdictInput<'a> {
    /**
     * Last time's entry for the same exercise, or `None` for a first
     * session. **The caller must not pass a deload session here**: deloads
     * are never a baseline (DD-13 gap 2). The engine's "last time" query
     * skips them.
     */
    pub baseline: Option<&'a SessionExercise>,
    /** This session's entry. */
    pub current: &'a SessionExercise,
    /** Whether the current session is marked as a deload. */
    pub current_is_deload: bool,
    /** The exercise's scheme, if any. Without one, `LoadStep` never fires. */
    pub scheme: Option<&'a ProgressionScheme>,
}

/**
 * Computes the verdict for one exercise. The rules are evaluated in order
 * (DD-13):
 *
 * 1. A tracking mode without rules is an error.
 * 2. A deload session is `Deload`, and its signals are still listed.
 * 3. No baseline, or no counted sets on either side, is `InsufficientData`.
 * 4. Any `Up` signal (a `LoadStep` is always `Up`) is `Progressed`.
 * 5. Otherwise, any `Down` signal is `Regressed`.
 * 6. Otherwise, `Matched`.
 *
 * # Errors
 *
 * - [`VerdictError::UnsupportedMode`] unless the current exercise is
 *   tracked as `WeightReps`.
 * - [`VerdictError::ExerciseMismatch`] if the baseline is another exercise.
 * - [`VerdictError::ModeMismatch`] if the baseline was tracked in another
 *   mode.
 */
pub fn exercise_verdict(input: &VerdictInput<'_>) -> Result<ExerciseVerdict, VerdictError> {
    let current = weight_reps_sets(input.current)?;
    let detection = match input.baseline {
        Some(baseline) => {
            check_same_exercise(baseline, input.current)?;
            detect_weight_reps(&weight_reps_sets(baseline)?, &current, input.scheme)
        }
        None => Detection::default(),
    };
    let comparable =
        input.baseline.is_some_and(has_counted_sets) && has_counted_sets(input.current);

    let verdict = if input.current_is_deload {
        Verdict::Deload
    } else if !comparable {
        Verdict::InsufficientData
    } else if has_direction(&detection.signals, Direction::Up) {
        Verdict::Progressed
    } else if has_direction(&detection.signals, Direction::Down) {
        Verdict::Regressed
    } else {
        Verdict::Matched
    };

    Ok(ExerciseVerdict {
        verdict,
        signals: detection.signals,
        skipped: detection.skipped,
        algorithm_version: ALGORITHM_VERSION,
    })
}

fn has_direction(signals: &[Signal], direction: Direction) -> bool {
    signals.iter().any(|signal| signal.direction() == direction)
}

fn check_same_exercise(
    baseline: &SessionExercise,
    current: &SessionExercise,
) -> Result<(), VerdictError> {
    if baseline.exercise() != current.exercise() {
        return Err(VerdictError::ExerciseMismatch {
            baseline: baseline.exercise(),
            current: current.exercise(),
        });
    }
    if baseline.mode() != current.mode() {
        return Err(VerdictError::ModeMismatch {
            baseline: baseline.mode(),
            current: current.mode(),
        });
    }
    Ok(())
}

fn has_counted_sets(entry: &SessionExercise) -> bool {
    entry.sets().iter().any(|s| s.set_type.counts_by_default())
}

/**
 * Reads an entry's sets as weight × reps. A [`SessionExercise`] guarantees
 * that every set matches its mode, so the error arms only catch a mode with
 * no rules yet.
 */
fn weight_reps_sets(entry: &SessionExercise) -> Result<Vec<WeightRepsSet>, VerdictError> {
    let unsupported = || VerdictError::UnsupportedMode(entry.mode());
    match entry.mode() {
        TrackingMode::WeightReps => {}
        TrackingMode::BodyweightReps
        | TrackingMode::WeightedBodyweight
        | TrackingMode::AssistedBodyweight
        | TrackingMode::Duration
        | TrackingMode::DistanceLoad
        | TrackingMode::RepsOnly
        | TrackingMode::TimeReps => return Err(unsupported()),
    }
    entry
        .sets()
        .iter()
        .map(|set| match set.metrics {
            SetMetrics::WeightReps { load, reps } => Ok(WeightRepsSet {
                set_type: set.set_type,
                load,
                reps,
                effort: set.effort,
            }),
            SetMetrics::BodyweightReps { .. }
            | SetMetrics::WeightedBodyweight { .. }
            | SetMetrics::AssistedBodyweight { .. }
            | SetMetrics::Duration { .. }
            | SetMetrics::DistanceLoad { .. }
            | SetMetrics::RepsOnly { .. }
            | SetMetrics::TimeReps { .. } => Err(unsupported()),
        })
        .collect()
}

/**
 * How a session's exercises fared: "4 of 6 exercises progressed".
 *
 * Counts only. A single yes/no for the whole session is a product decision
 * that hasn't been made yet (design.md open question 6).
 */
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct SessionVerdict {
    /** Exercises that progressed. */
    pub progressed: u16,
    /** Exercises that matched last time. */
    pub matched: u16,
    /** Exercises that regressed. */
    pub regressed: u16,
    /** Exercises with nothing to compare. */
    pub insufficient: u16,
    /** Exercises in a deload session. */
    pub deload: u16,
}

impl SessionVerdict {
    /**
     * Counts the given verdicts. Each count saturates at `u16::MAX`, far
     * beyond any real session.
     */
    pub fn tally<'a>(verdicts: impl IntoIterator<Item = &'a ExerciseVerdict>) -> Self {
        let mut tally = Self::default();
        for exercise in verdicts {
            let count = match exercise.verdict {
                Verdict::Progressed => &mut tally.progressed,
                Verdict::Matched => &mut tally.matched,
                Verdict::Regressed => &mut tally.regressed,
                Verdict::InsufficientData => &mut tally.insufficient,
                Verdict::Deload => &mut tally.deload,
            };
            *count = count.saturating_add(1);
        }
        tally
    }

    /** How many exercises were counted. */
    pub fn total(&self) -> u32 {
        [
            self.progressed,
            self.matched,
            self.regressed,
            self.insufficient,
            self.deload,
        ]
        .into_iter()
        .map(u32::from)
        .sum()
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;
    use crate::exercise::Exercise;
    use crate::ids::SetId;
    use crate::set::{LoggedSet, SetType};
    use crate::text::Name;
    use crate::units::{DurationSecs, Load, Reps, WeightUnit};

    fn exercise(id: u128, mode: TrackingMode) -> Exercise {
        Exercise {
            id: ExerciseId::from_uuid(Uuid::from_u128(id)),
            name: Name::new("Lift").unwrap(),
            mode,
        }
    }

    fn entry(exercise: &Exercise, set_type: SetType) -> SessionExercise {
        let mut entry = SessionExercise::new(exercise);
        let metrics = if exercise.mode == TrackingMode::Duration {
            SetMetrics::Duration {
                duration: DurationSecs::new(60).unwrap(),
                load: None,
            }
        } else {
            SetMetrics::WeightReps {
                load: Load::new(10_000, WeightUnit::Kilograms).unwrap(),
                reps: Reps::new(8).unwrap(),
            }
        };
        entry
            .push_set(LoggedSet {
                id: SetId::from_uuid(Uuid::from_u128(9)),
                set_type,
                metrics,
                effort: None,
                note: None,
            })
            .unwrap();
        entry
    }

    fn verdict(
        baseline: Option<&SessionExercise>,
        current: &SessionExercise,
    ) -> Result<ExerciseVerdict, VerdictError> {
        exercise_verdict(&VerdictInput {
            baseline,
            current,
            current_is_deload: false,
            scheme: None,
        })
    }

    #[test]
    fn refuses_to_compare_different_exercises() {
        let bench = entry(&exercise(1, TrackingMode::WeightReps), SetType::Working);
        let squat = entry(&exercise(2, TrackingMode::WeightReps), SetType::Working);
        assert_eq!(
            verdict(Some(&squat), &bench),
            Err(VerdictError::ExerciseMismatch {
                baseline: squat.exercise(),
                current: bench.exercise(),
            })
        );
    }

    #[test]
    fn refuses_a_baseline_tracked_in_another_mode() {
        let held = entry(&exercise(1, TrackingMode::Duration), SetType::Working);
        let lifted = entry(&exercise(1, TrackingMode::WeightReps), SetType::Working);
        assert_eq!(
            verdict(Some(&held), &lifted),
            Err(VerdictError::ModeMismatch {
                baseline: TrackingMode::Duration,
                current: TrackingMode::WeightReps,
            })
        );
    }

    #[test]
    fn warmups_alone_are_insufficient_data() {
        let lift = exercise(1, TrackingMode::WeightReps);
        let warmup = entry(&lift, SetType::Warmup);
        let working = entry(&lift, SetType::Working);
        let v = verdict(Some(&warmup), &working).unwrap();
        assert_eq!((v.verdict, v.signals.len()), (Verdict::InsufficientData, 0));
        let v = verdict(Some(&working), &warmup).unwrap();
        assert_eq!(v.verdict, Verdict::InsufficientData);
    }

    #[test]
    fn a_first_deload_session_is_still_a_deload() {
        let working = entry(&exercise(1, TrackingMode::WeightReps), SetType::Working);
        let v = exercise_verdict(&VerdictInput {
            baseline: None,
            current: &working,
            current_is_deload: true,
            scheme: None,
        })
        .unwrap();
        assert_eq!(v.verdict, Verdict::Deload);
        assert_eq!(v.algorithm_version, ALGORITHM_VERSION);
    }
}
