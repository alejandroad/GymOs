/*!
 * The `sets` table: one row per logged set, whatever its tracking mode
 * (S-4). The `mode` column says which measurement columns are in use.
 */

use gymos_domain::{
    Distance, DurationSecs, Effort, Load, LoggedSet, Note, Reps, SessionExerciseId, SetId,
    SetMetrics, TrackingMode,
};
use gymos_engine::RepoError;
use rusqlite::{Connection, Row, params};

use super::{
    Invalid, LoadColumns, SCHEMA_VERSION, blob, codes, note, nullable, optional_load, uuid,
};
use crate::error::{IntoRepo, corrupt};

const TABLE: &str = "sets";

/** A set's measurements, flattened into the columns every mode shares. */
struct Measures {
    load: Option<Load>,
    reps: Option<Reps>,
    duration: Option<DurationSecs>,
    distance: Option<Distance>,
}

impl Measures {
    const fn of(metrics: SetMetrics) -> Self {
        let none = Self {
            load: None,
            reps: None,
            duration: None,
            distance: None,
        };
        match metrics {
            SetMetrics::BodyweightReps { reps } | SetMetrics::RepsOnly { reps } => Self {
                reps: Some(reps),
                ..none
            },
            SetMetrics::WeightReps { load, reps }
            | SetMetrics::WeightedBodyweight { added: load, reps }
            | SetMetrics::AssistedBodyweight {
                assistance: load,
                reps,
            } => Self {
                load: Some(load),
                reps: Some(reps),
                ..none
            },
            SetMetrics::Duration { duration, load } => Self {
                load,
                duration: Some(duration),
                ..none
            },
            SetMetrics::DistanceLoad {
                distance,
                load,
                duration,
            } => Self {
                load: Some(load),
                duration,
                distance: Some(distance),
                ..none
            },
            SetMetrics::TimeReps { reps, duration } => Self {
                reps: Some(reps),
                duration: Some(duration),
                ..none
            },
        }
    }

    /**
     * Rebuilds the measurements for `mode`. `None` if a column the mode
     * needs is missing or one it doesn't use is set.
     */
    const fn into_metrics(self, mode: TrackingMode) -> Option<SetMetrics> {
        let Self {
            load,
            reps,
            duration,
            distance,
        } = self;
        Some(match (mode, load, reps, duration, distance) {
            (TrackingMode::WeightReps, Some(load), Some(reps), None, None) => {
                SetMetrics::WeightReps { load, reps }
            }
            (TrackingMode::BodyweightReps, None, Some(reps), None, None) => {
                SetMetrics::BodyweightReps { reps }
            }
            (TrackingMode::WeightedBodyweight, Some(added), Some(reps), None, None) => {
                SetMetrics::WeightedBodyweight { added, reps }
            }
            (TrackingMode::AssistedBodyweight, Some(assistance), Some(reps), None, None) => {
                SetMetrics::AssistedBodyweight { assistance, reps }
            }
            (TrackingMode::Duration, load, None, Some(duration), None) => {
                SetMetrics::Duration { duration, load }
            }
            (TrackingMode::DistanceLoad, Some(load), None, duration, Some(distance)) => {
                SetMetrics::DistanceLoad {
                    distance,
                    load,
                    duration,
                }
            }
            (TrackingMode::RepsOnly, None, Some(reps), None, None) => SetMetrics::RepsOnly { reps },
            (TrackingMode::TimeReps, None, Some(reps), Some(duration), None) => {
                SetMetrics::TimeReps { reps, duration }
            }
            (
                TrackingMode::WeightReps
                | TrackingMode::BodyweightReps
                | TrackingMode::WeightedBodyweight
                | TrackingMode::AssistedBodyweight
                | TrackingMode::Duration
                | TrackingMode::DistanceLoad
                | TrackingMode::RepsOnly
                | TrackingMode::TimeReps,
                ..,
            ) => return None,
        })
    }
}

/** An effort from its columns: both null, or a valid scale and value. */
fn effort(scale: Option<&str>, value: Option<i64>) -> Result<Option<Effort>, Invalid> {
    match (scale, value) {
        (None, None) => Ok(None),
        (Some(scale), Some(value)) => codes::effort(scale, value).map(Some).ok_or(Invalid),
        _ => Err(Invalid),
    }
}

/**
 * Writes one set at `position` in `entry`, reviving it if tombstoned. A
 * set may move between entries of the same session; a set ID that belongs
 * to another session is refused rather than moved out of it.
 */
pub(super) fn save_set(
    conn: &Connection,
    entry: SessionExerciseId,
    position: i64,
    set: &LoggedSet,
) -> Result<(), RepoError> {
    let measures = Measures::of(set.metrics);
    let load = measures.load.map(LoadColumns::of);
    let effort = set.effort.map(codes::effort_code);
    let written = conn
        .prepare_cached(
            "INSERT INTO sets (id, entry_id, position, set_type, mode,
                 load_hundredths, load_unit, load_ng, reps, duration_s, distance_mm,
                 effort_scale, effort_value, note, deleted, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, 0, ?15)
             ON CONFLICT (id) DO UPDATE SET
                 entry_id = excluded.entry_id,
                 position = excluded.position,
                 set_type = excluded.set_type,
                 mode = excluded.mode,
                 load_hundredths = excluded.load_hundredths,
                 load_unit = excluded.load_unit,
                 load_ng = excluded.load_ng,
                 reps = excluded.reps,
                 duration_s = excluded.duration_s,
                 distance_mm = excluded.distance_mm,
                 effort_scale = excluded.effort_scale,
                 effort_value = excluded.effort_value,
                 note = excluded.note,
                 deleted = 0,
                 schema_version = excluded.schema_version
             WHERE (SELECT session_id FROM session_entries WHERE id = sets.entry_id)
                 = (SELECT session_id FROM session_entries WHERE id = excluded.entry_id)",
        )
        .repo()?
        .execute(params![
            blob(set.id.as_uuid()),
            blob(entry.as_uuid()),
            position,
            codes::set_type_code(set.set_type),
            codes::mode_code(set.metrics.mode()),
            load.as_ref().map(|l| l.hundredths),
            load.as_ref().map(|l| l.unit),
            load.as_ref().map(|l| l.nanograms),
            measures.reps.map(Reps::get),
            measures.duration.map(DurationSecs::get),
            measures.distance.map(Distance::millimetres),
            effort.map(|(scale, _)| scale),
            effort.map(|(_, value)| value),
            set.note.as_ref().map(Note::as_str),
            SCHEMA_VERSION,
        ])
        .repo()?;
    if written != 1 {
        return Err(RepoError::Backend(format!(
            "set {} belongs to another session than entry {entry}'s",
            set.id
        )));
    }
    Ok(())
}

/** The live sets of `entry`, in order. */
pub(super) fn load_sets(
    conn: &Connection,
    entry: SessionExerciseId,
) -> Result<Vec<LoggedSet>, RepoError> {
    let mut statement = conn
        .prepare_cached(
            "SELECT id, set_type, mode, load_hundredths, load_unit, load_ng,
                    reps, duration_s, distance_mm, effort_scale, effort_value, note
             FROM sets WHERE entry_id = ?1 AND deleted = 0
             ORDER BY position",
        )
        .repo()?;
    let rows = statement
        .query_map([blob(entry.as_uuid())], Raw::read)
        .repo()?;
    let mut sets = Vec::new();
    for raw in rows {
        sets.push(raw.repo()?.into_set(entry)?);
    }
    Ok(sets)
}

/** The columns of one row, before validation. */
struct Raw {
    id: Vec<u8>,
    set_type: String,
    mode: String,
    load_hundredths: Option<i64>,
    load_unit: Option<String>,
    load_ng: Option<i64>,
    reps: Option<i64>,
    duration_s: Option<i64>,
    distance_mm: Option<i64>,
    effort_scale: Option<String>,
    effort_value: Option<i64>,
    note: Option<String>,
}

impl Raw {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            set_type: row.get(1)?,
            mode: row.get(2)?,
            load_hundredths: row.get(3)?,
            load_unit: row.get(4)?,
            load_ng: row.get(5)?,
            reps: row.get(6)?,
            duration_s: row.get(7)?,
            distance_mm: row.get(8)?,
            effort_scale: row.get(9)?,
            effort_value: row.get(10)?,
            note: row.get(11)?,
        })
    }

    fn into_set(self, entry: SessionExerciseId) -> Result<LoggedSet, RepoError> {
        let id = uuid(&self.id)
            .map(SetId::from_uuid)
            .ok_or_else(|| corrupt(TABLE, &format_args!("in entry {entry}"), "id"))?;
        let bad = |field| corrupt(TABLE, &id, field);
        let mode = codes::mode(&self.mode).ok_or_else(|| bad("mode"))?;
        let measures = Measures {
            load: optional_load(
                self.load_hundredths,
                self.load_unit.as_deref(),
                self.load_ng,
            )
            .map_err(|Invalid| bad("load"))?,
            reps: nullable(self.reps, |n| Reps::new(u16::try_from(n).ok()?).ok())
                .map_err(|Invalid| bad("reps"))?,
            duration: nullable(self.duration_s, |s| {
                DurationSecs::new(u32::try_from(s).ok()?).ok()
            })
            .map_err(|Invalid| bad("duration_s"))?,
            distance: nullable(self.distance_mm, |mm| {
                Distance::from_millimetres(u32::try_from(mm).ok()?).ok()
            })
            .map_err(|Invalid| bad("distance_mm"))?,
        };
        Ok(LoggedSet {
            id,
            set_type: codes::set_type(&self.set_type).ok_or_else(|| bad("set_type"))?,
            metrics: measures
                .into_metrics(mode)
                .ok_or_else(|| bad("measurements for its mode"))?,
            effort: effort(self.effort_scale.as_deref(), self.effort_value)
                .map_err(|Invalid| bad("effort"))?,
            note: note(self.note.as_deref()).map_err(|Invalid| bad("note"))?,
        })
    }
}
