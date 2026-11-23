/*! The `exercises` table. */

use gymos_domain::progression::{ProgressionScheme, RepRange};
use gymos_domain::{Exercise, ExerciseId, Reps};
use gymos_engine::RepoError;
use rusqlite::{Connection, OptionalExtension, Row, params};

use super::{Invalid, LoadColumns, SCHEMA_VERSION, blob, codes, load, name, uuid};
use crate::error::{IntoRepo, corrupt};

const TABLE: &str = "exercises";

/** Inserts or replaces an exercise, reviving it if it was tombstoned. */
pub(crate) fn save_exercise(conn: &Connection, exercise: &Exercise) -> Result<(), RepoError> {
    let scheme = exercise.scheme.map(|s| {
        let increment = LoadColumns::of(s.increment());
        (s.reps().min().get(), s.reps().max().get(), increment)
    });
    let (min, max, increment) = match scheme {
        Some((min, max, inc)) => (Some(min), Some(max), Some(inc)),
        None => (None, None, None),
    };
    conn.prepare_cached(
        "INSERT INTO exercises (id, name, mode, scheme_min_reps, scheme_max_reps,
             scheme_increment_hundredths, scheme_increment_unit, scheme_increment_ng,
             deleted, schema_version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9)
         ON CONFLICT (id) DO UPDATE SET
             name = excluded.name,
             mode = excluded.mode,
             scheme_min_reps = excluded.scheme_min_reps,
             scheme_max_reps = excluded.scheme_max_reps,
             scheme_increment_hundredths = excluded.scheme_increment_hundredths,
             scheme_increment_unit = excluded.scheme_increment_unit,
             scheme_increment_ng = excluded.scheme_increment_ng,
             deleted = 0,
             schema_version = excluded.schema_version",
    )
    .repo()?
    .execute(params![
        blob(exercise.id.as_uuid()),
        exercise.name.as_str(),
        codes::mode_code(exercise.mode),
        min,
        max,
        increment.as_ref().map(|i| i.hundredths),
        increment.as_ref().map(|i| i.unit),
        increment.as_ref().map(|i| i.nanograms),
        SCHEMA_VERSION,
    ])
    .repo()?;
    Ok(())
}

/** Loads a live exercise. */
pub(crate) fn load_exercise(
    conn: &Connection,
    id: ExerciseId,
) -> Result<Option<Exercise>, RepoError> {
    let raw = conn
        .prepare_cached(
            "SELECT name, mode, scheme_min_reps, scheme_max_reps,
                    scheme_increment_hundredths, scheme_increment_unit, scheme_increment_ng
             FROM exercises WHERE id = ?1 AND deleted = 0",
        )
        .repo()?
        .query_row([blob(id.as_uuid())], Raw::read)
        .optional()
        .repo()?;
    raw.map(|raw| raw.into_exercise(id)).transpose()
}

/** The columns of one row, before validation. */
struct Raw {
    name: String,
    mode: String,
    min_reps: Option<i64>,
    max_reps: Option<i64>,
    increment_hundredths: Option<i64>,
    increment_unit: Option<String>,
    increment_ng: Option<i64>,
}

impl Raw {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            name: row.get(0)?,
            mode: row.get(1)?,
            min_reps: row.get(2)?,
            max_reps: row.get(3)?,
            increment_hundredths: row.get(4)?,
            increment_unit: row.get(5)?,
            increment_ng: row.get(6)?,
        })
    }

    fn into_exercise(self, id: ExerciseId) -> Result<Exercise, RepoError> {
        let bad = |field| corrupt(TABLE, &id, field);
        Ok(Exercise {
            id,
            name: name(&self.name).ok_or_else(|| bad("name"))?,
            mode: codes::mode(&self.mode).ok_or_else(|| bad("mode"))?,
            scheme: self.scheme().map_err(|Invalid| bad("scheme"))?,
        })
    }

    /** The scheme: all five columns null, or all five a valid scheme. */
    fn scheme(&self) -> Result<Option<ProgressionScheme>, Invalid> {
        let (Some(min), Some(max), Some(h), Some(unit), Some(ng)) = (
            self.min_reps,
            self.max_reps,
            self.increment_hundredths,
            self.increment_unit.as_deref(),
            self.increment_ng,
        ) else {
            let all_null = self.min_reps.is_none()
                && self.max_reps.is_none()
                && self.increment_hundredths.is_none()
                && self.increment_unit.is_none()
                && self.increment_ng.is_none();
            return if all_null { Ok(None) } else { Err(Invalid) };
        };
        let reps = |n: i64| Reps::new(u16::try_from(n).ok()?).ok();
        let scheme = (|| {
            let range = RepRange::new(reps(min)?, reps(max)?).ok()?;
            ProgressionScheme::new(range, load(h, unit, ng)?).ok()
        })();
        scheme.map(Some).ok_or(Invalid)
    }
}

/** The reading side needs an ID back from a stored blob. */
pub(super) fn exercise_id(raw: &[u8]) -> Option<ExerciseId> {
    uuid(raw).map(ExerciseId::from_uuid)
}
