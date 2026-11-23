/*!
 * Stable text codes for every enum the store writes (S-4).
 *
 * Each enum has one function per direction. Encoding is an exhaustive
 * `match`, so a new variant fails to compile until it has a code. Decoding
 * returns `None` for an unknown code, which the caller reports as corrupt.
 * Codes are part of the schema: never rename one, add new ones only with
 * a migration that extends the column's `CHECK`.
 */

use gymos_domain::{
    Effort, ExerciseId, ImportFormat, Rir, Rpe, SessionExerciseId, SessionId, SetId, SetType,
    Source, TrackingMode, Uuid, WeightUnit,
};
use gymos_engine::CommandOutcome;

/** A tracking mode's code. */
pub(crate) const fn mode_code(mode: TrackingMode) -> &'static str {
    match mode {
        TrackingMode::WeightReps => "weight_reps",
        TrackingMode::BodyweightReps => "bodyweight_reps",
        TrackingMode::WeightedBodyweight => "weighted_bodyweight",
        TrackingMode::AssistedBodyweight => "assisted_bodyweight",
        TrackingMode::Duration => "duration",
        TrackingMode::DistanceLoad => "distance_load",
        TrackingMode::RepsOnly => "reps_only",
        TrackingMode::TimeReps => "time_reps",
    }
}

/** The tracking mode for a code. */
pub(crate) fn mode(code: &str) -> Option<TrackingMode> {
    Some(match code {
        "weight_reps" => TrackingMode::WeightReps,
        "bodyweight_reps" => TrackingMode::BodyweightReps,
        "weighted_bodyweight" => TrackingMode::WeightedBodyweight,
        "assisted_bodyweight" => TrackingMode::AssistedBodyweight,
        "duration" => TrackingMode::Duration,
        "distance_load" => TrackingMode::DistanceLoad,
        "reps_only" => TrackingMode::RepsOnly,
        "time_reps" => TrackingMode::TimeReps,
        _ => return None,
    })
}

/** A set type's code. */
pub(crate) const fn set_type_code(set_type: SetType) -> &'static str {
    match set_type {
        SetType::Warmup => "warmup",
        SetType::Working => "working",
        SetType::Drop => "drop",
        SetType::Failure => "failure",
        SetType::TopSet => "top_set",
        SetType::Backoff => "backoff",
    }
}

/** The set type for a code. */
pub(crate) fn set_type(code: &str) -> Option<SetType> {
    Some(match code {
        "warmup" => SetType::Warmup,
        "working" => SetType::Working,
        "drop" => SetType::Drop,
        "failure" => SetType::Failure,
        "top_set" => SetType::TopSet,
        "backoff" => SetType::Backoff,
        _ => return None,
    })
}

/** A weight unit's code. */
pub(crate) const fn unit_code(unit: WeightUnit) -> &'static str {
    match unit {
        WeightUnit::Kilograms => "kg",
        WeightUnit::Pounds => "lb",
    }
}

/** The weight unit for a code. */
pub(crate) fn unit(code: &str) -> Option<WeightUnit> {
    Some(match code {
        "kg" => WeightUnit::Kilograms,
        "lb" => WeightUnit::Pounds,
        _ => return None,
    })
}

/** An import format's code. */
pub(crate) const fn import_format_code(format: ImportFormat) -> &'static str {
    match format {
        ImportFormat::Strong => "strong",
        ImportFormat::Hevy => "hevy",
        ImportFormat::GarminActivities => "garmin_activities",
        ImportFormat::GarminVo2Max => "garmin_vo2max",
    }
}

/** The import format for a code. */
pub(crate) fn import_format(code: &str) -> Option<ImportFormat> {
    Some(match code {
        "strong" => ImportFormat::Strong,
        "hevy" => ImportFormat::Hevy,
        "garmin_activities" => ImportFormat::GarminActivities,
        "garmin_vo2max" => ImportFormat::GarminVo2Max,
        _ => return None,
    })
}

/*
 * Enums that carry data are stored as a code plus columns for the data.
 * Their mappings live here too, so every stored code is in one file.
 */

/** An effort's scale code and stored value: RPE in half points, RIR in reps. */
pub(crate) fn effort_code(effort: Effort) -> (&'static str, i64) {
    match effort {
        Effort::Rpe(rpe) => ("rpe", i64::from(rpe.half_points())),
        Effort::Rir(rir) => ("rir", i64::from(rir.get())),
    }
}

/** The effort for a scale code and value, if both are valid. */
pub(crate) fn effort(scale: &str, value: i64) -> Option<Effort> {
    let value = u8::try_from(value).ok()?;
    match scale {
        "rpe" => Rpe::from_half_points(value).ok().map(Effort::Rpe),
        "rir" => Rir::new(value).ok().map(Effort::Rir),
        _ => None,
    }
}

/** Provenance as columns: kind code, import format code, importer version. */
pub(crate) const fn source_code(
    source: Source,
) -> (&'static str, Option<&'static str>, Option<u16>) {
    match source {
        Source::Manual => ("manual", None, None),
        Source::Import {
            format,
            importer_version,
        } => (
            "import",
            Some(import_format_code(format)),
            Some(importer_version),
        ),
    }
}

/** Provenance from its columns, if they are valid and fit together. */
pub(crate) fn source(kind: &str, format: Option<&str>, version: Option<i64>) -> Option<Source> {
    match (kind, format, version) {
        ("manual", None, None) => Some(Source::Manual),
        ("import", Some(format), Some(version)) => Some(Source::Import {
            format: import_format(format)?,
            importer_version: u16::try_from(version).ok()?,
        }),
        _ => None,
    }
}

/** A command outcome's kind code and the ID it carries, if any. */
pub(crate) const fn outcome_code(outcome: CommandOutcome) -> (&'static str, Option<Uuid>) {
    match outcome {
        CommandOutcome::ExerciseCreated(id) => ("exercise_created", Some(id.as_uuid())),
        CommandOutcome::SessionStarted(id) => ("session_started", Some(id.as_uuid())),
        CommandOutcome::ExerciseAdded(id) => ("exercise_added", Some(id.as_uuid())),
        CommandOutcome::SetLogged(id) => ("set_logged", Some(id.as_uuid())),
        CommandOutcome::SessionFinished => ("session_finished", None),
        CommandOutcome::DeloadSet => ("deload_set", None),
    }
}

/** The outcome for a kind code and ID, if they fit together. */
pub(crate) fn outcome(kind: &str, id: Option<Uuid>) -> Option<CommandOutcome> {
    Some(match (kind, id) {
        ("exercise_created", Some(id)) => {
            CommandOutcome::ExerciseCreated(ExerciseId::from_uuid(id))
        }
        ("session_started", Some(id)) => CommandOutcome::SessionStarted(SessionId::from_uuid(id)),
        ("exercise_added", Some(id)) => {
            CommandOutcome::ExerciseAdded(SessionExerciseId::from_uuid(id))
        }
        ("set_logged", Some(id)) => CommandOutcome::SetLogged(SetId::from_uuid(id)),
        ("session_finished", None) => CommandOutcome::SessionFinished,
        ("deload_set", None) => CommandOutcome::DeloadSet,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_effort_round_trips() {
        let all = [
            Effort::Rpe(Rpe::from_half_points(Rpe::MIN_HALF_POINTS).unwrap()),
            Effort::Rpe(Rpe::from_half_points(Rpe::MAX_HALF_POINTS).unwrap()),
            Effort::Rir(Rir::new(0).unwrap()),
            Effort::Rir(Rir::new(Rir::MAX).unwrap()),
        ];
        for e in all {
            let (scale, value) = effort_code(e);
            assert_eq!(effort(scale, value), Some(e));
        }
        assert_eq!(effort("rpe", 21), None, "out of range");
        assert_eq!(effort("rpe", -1), None);
        assert_eq!(effort("borg", 5), None);
    }

    #[test]
    fn every_source_round_trips() {
        let all = [
            Source::Manual,
            Source::Import {
                format: ImportFormat::Strong,
                importer_version: u16::MAX,
            },
            Source::Import {
                format: ImportFormat::GarminVo2Max,
                importer_version: 0,
            },
        ];
        for s in all {
            let (kind, format, version) = source_code(s);
            assert_eq!(source(kind, format, version.map(i64::from)), Some(s));
        }
        assert_eq!(source("manual", Some("hevy"), None), None, "parts must fit");
        assert_eq!(source("import", Some("hevy"), Some(70_000)), None);
    }

    #[test]
    fn every_outcome_round_trips() {
        let id = Uuid::from_u128(7);
        let all = [
            CommandOutcome::ExerciseCreated(ExerciseId::from_uuid(id)),
            CommandOutcome::SessionStarted(SessionId::from_uuid(id)),
            CommandOutcome::ExerciseAdded(SessionExerciseId::from_uuid(id)),
            CommandOutcome::SetLogged(SetId::from_uuid(id)),
            CommandOutcome::SessionFinished,
            CommandOutcome::DeloadSet,
        ];
        for o in all {
            let (kind, id) = outcome_code(o);
            assert_eq!(outcome(kind, id), Some(o));
        }
        assert_eq!(outcome("set_logged", None), None, "the ID is required");
        assert_eq!(outcome("deload_set", Some(id)), None, "and forbidden here");
    }

    /*
     * Each list names every variant. The encoders' exhaustive matches catch
     * a new variant at compile time; add it here too.
     */

    #[test]
    fn every_mode_round_trips() {
        let all = [
            TrackingMode::WeightReps,
            TrackingMode::BodyweightReps,
            TrackingMode::WeightedBodyweight,
            TrackingMode::AssistedBodyweight,
            TrackingMode::Duration,
            TrackingMode::DistanceLoad,
            TrackingMode::RepsOnly,
            TrackingMode::TimeReps,
        ];
        for m in all {
            assert_eq!(mode(mode_code(m)), Some(m));
        }
        assert_eq!(mode("WeightReps"), None, "never a Debug string");
    }

    #[test]
    fn every_set_type_round_trips() {
        let all = [
            SetType::Warmup,
            SetType::Working,
            SetType::Drop,
            SetType::Failure,
            SetType::TopSet,
            SetType::Backoff,
        ];
        for t in all {
            assert_eq!(set_type(set_type_code(t)), Some(t));
        }
        assert_eq!(set_type(""), None);
    }

    #[test]
    fn every_unit_round_trips() {
        for u in [WeightUnit::Kilograms, WeightUnit::Pounds] {
            assert_eq!(unit(unit_code(u)), Some(u));
        }
        assert_eq!(unit("KG"), None, "codes are case-sensitive");
    }

    #[test]
    fn every_import_format_round_trips() {
        let all = [
            ImportFormat::Strong,
            ImportFormat::Hevy,
            ImportFormat::GarminActivities,
            ImportFormat::GarminVo2Max,
        ];
        for f in all {
            assert_eq!(import_format(import_format_code(f)), Some(f));
        }
        assert_eq!(import_format("csv"), None);
    }
}
