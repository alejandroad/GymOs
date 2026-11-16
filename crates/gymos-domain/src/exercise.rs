/*! Exercises and how they are tracked. */

use crate::ids::ExerciseId;
use crate::text::Name;

/**
 * How an exercise's sets are measured (product spec §3.1).
 *
 * The tracking mode decides which fields a set has, which PR types apply
 * and which overload rules run, so every rule matches on it exhaustively.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TrackingMode {
    /** Load × reps: bench press, squat, curl. */
    WeightReps,
    /** Reps at body weight: push-up, pull-up. */
    BodyweightReps,
    /** Body weight plus added load: weighted pull-up, dip. */
    WeightedBodyweight,
    /** Body weight minus assistance: assisted pull-up. */
    AssistedBodyweight,
    /** A timed hold, optionally loaded: plank, dead hang. */
    Duration,
    /** A distance under load, optionally timed: farmer's carry, sled push. */
    DistanceLoad,
    /** Reps with no load: burpees. */
    RepsOnly,
    /** Reps within a time window: kettlebell swings EMOM. */
    TimeReps,
}

/**
 * An exercise from the catalog or created by the user.
 *
 * Fields are public because every combination of valid values is a valid
 * exercise. Rules such as "a mode can't change once sets exist" need
 * history, so the engine enforces them.
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exercise {
    /** Stable identity. */
    pub id: ExerciseId,
    /** Display name, such as "Bench Press". */
    pub name: Name,
    /** How this exercise's sets are measured. */
    pub mode: TrackingMode,
}
