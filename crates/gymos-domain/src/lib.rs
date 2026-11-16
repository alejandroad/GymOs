/*!
 * # gymos-domain
 *
 * The pure core of GymOs: value types (newtypes such as reps and loads),
 * the set and session model, and the progression and PR rules.
 *
 * ## Rules for this crate
 *
 * - **Pure.** No I/O, no clock, no randomness, no environment. Every rule is
 *   a function from data to data, so it is deterministic and testable
 *   without infrastructure (product D9, DD-03).
 * - **Parse, don't validate.** Every value is constructed through a fallible
 *   constructor, so invalid states are unrepresentable (DD-08).
 * - **No wire formats.** Types here derive no `serde` or `specta`; the IPC,
 *   database and archive boundaries own their own representations.
 * - **Dependencies:** `std` and small pure crates only. Never `rusqlite`,
 *   `tauri` or anything that touches the OS.
 *
 * ## Model at a glance
 *
 * ```text
 * Session ──< SessionExercise ──< LoggedSet
 *   │             │                  ├─ SetType
 *   │             └─ ExerciseId      ├─ SetMetrics (one variant per TrackingMode)
 *   └─ Timestamp (UTC + offset)      └─ Effort (RPE | RIR)
 *
 * Measurement   time series: body weight, VO2 max, resting HR, custom metrics
 * Activity      runs, rides, classes: kind + elapsed time + checked details
 * Profile       display name, unit preferences, optional percentile inputs
 * Record        derived: Subject (open) × RecordKind (closed) → Quantity
 * ```
 *
 * The stats and medals page is not a type here. It is a view the engine
 * builds from the profile, the latest measurements and the records.
 */

mod activity;
mod effort;
mod exercise;
mod health;
mod ids;
mod measurement;
mod metric;
mod profile;
mod provenance;
mod records;
mod session;
mod set;
mod text;
mod time;
mod units;

/**
 * The version of the derived-data rules (overload verdicts, records).
 * Bump it whenever a rule changes; derived data is then recomputed locally
 * (product §3.5, DD-13).
 */
pub const ALGORITHM_VERSION: u32 = 1;

pub use activity::{Activity, ActivityDetails, ActivityError, ActivityKind};
pub use effort::{Effort, EffortError, PerceivedEffort, Rir, Rpe};
pub use exercise::{Exercise, TrackingMode};
pub use health::{BodyWeight, HealthError, HeartRate, Vo2Max};
pub use ids::{ActivityId, ExerciseId, MeasurementId, MetricId, SessionId, SetId, TemplateId};
pub use measurement::{Measurement, MeasurementKind, MeasurementValue, body_weight_at};
pub use metric::{Better, CustomMetric, CustomValue, MetricError};
pub use profile::{
    BirthYear, DistanceUnit, Profile, ProfileError, ReferencePopulation, UnitPreferences,
};
pub use provenance::{ImportFormat, Source};
pub use records::{Quantity, Record, RecordKind, RecordSource, StandardDistance, Subject};
pub use session::{Session, SessionError, SessionExercise};
pub use set::{LoggedSet, SetMetrics, SetType};
pub use text::{Name, Note, TextError, UnitLabel};
pub use time::{TimeError, Timestamp, UtcOffset};
pub use units::{
    Distance, DurationSecs, ElapsedTime, ElevationGain, Energy, Load, Mass, Reps, UnitError,
    WeightUnit,
};
