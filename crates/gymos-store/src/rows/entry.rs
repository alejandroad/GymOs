/*! The `session_entries` table: one row per exercise entry in a session. */

use gymos_domain::{
    Exercise, ExerciseId, Name, Note, SessionExercise, SessionExerciseId, SessionId, TrackingMode,
};
use gymos_engine::RepoError;
use rusqlite::{Connection, Row, params};

use super::exercise::exercise_id;
use super::set::{load_sets, save_set};
use super::{Invalid, SCHEMA_VERSION, blob, codes, note, uuid};
use crate::error::{IntoRepo, corrupt};

const TABLE: &str = "session_entries";

/**
 * Writes one entry at `position` in `session`, then its sets in order,
 * reviving any that were tombstoned. An entry ID that belongs to another
 * session is refused rather than moved, which would silently take it out
 * of that session.
 */
pub(super) fn save_entry(
    conn: &Connection,
    session: SessionId,
    position: i64,
    entry: &SessionExercise,
) -> Result<(), RepoError> {
    let written = conn
        .prepare_cached(
            "INSERT INTO session_entries (id, session_id, exercise_id, position, mode, note,
                 deleted, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7)
             ON CONFLICT (id) DO UPDATE SET
                 exercise_id = excluded.exercise_id,
                 position = excluded.position,
                 mode = excluded.mode,
                 note = excluded.note,
                 deleted = 0,
                 schema_version = excluded.schema_version
             WHERE session_entries.session_id = excluded.session_id",
        )
        .repo()?
        .execute(params![
            blob(entry.id().as_uuid()),
            blob(session.as_uuid()),
            blob(entry.exercise().as_uuid()),
            position,
            codes::mode_code(entry.mode()),
            entry.note().map(Note::as_str),
            SCHEMA_VERSION,
        ])
        .repo()?;
    if written != 1 {
        return Err(RepoError::Backend(format!(
            "entry {} belongs to another session than {session}",
            entry.id()
        )));
    }
    for (position, set) in (0_i64..).zip(entry.sets()) {
        save_set(conn, entry.id(), position, set)?;
    }
    Ok(())
}

/** The live entries of `session`, in order, each with its live sets. */
pub(super) fn load_entries(
    conn: &Connection,
    session: SessionId,
) -> Result<Vec<SessionExercise>, RepoError> {
    let raws = {
        let mut statement = conn
            .prepare_cached(
                "SELECT id, exercise_id, mode, note FROM session_entries
                 WHERE session_id = ?1 AND deleted = 0
                 ORDER BY position",
            )
            .repo()?;
        let rows = statement
            .query_map([blob(session.as_uuid())], Raw::read)
            .repo()?;
        rows.collect::<rusqlite::Result<Vec<_>>>().repo()?
    };
    raws.into_iter()
        .map(|raw| raw.into_entry(conn, session))
        .collect()
}

/** The columns of one row, before validation. */
struct Raw {
    id: Vec<u8>,
    exercise: Vec<u8>,
    mode: String,
    note: Option<String>,
}

impl Raw {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            exercise: row.get(1)?,
            mode: row.get(2)?,
            note: row.get(3)?,
        })
    }

    fn into_entry(
        self,
        conn: &Connection,
        session: SessionId,
    ) -> Result<SessionExercise, RepoError> {
        let id = uuid(&self.id)
            .map(SessionExerciseId::from_uuid)
            .ok_or_else(|| corrupt(TABLE, &format_args!("in session {session}"), "id"))?;
        let bad = |field| corrupt(TABLE, &id, field);
        let exercise = exercise_id(&self.exercise).ok_or_else(|| bad("exercise_id"))?;
        let mode = codes::mode(&self.mode).ok_or_else(|| bad("mode"))?;
        let mut entry =
            SessionExercise::new(id, &carrier(exercise, mode).ok_or_else(|| bad("mode"))?);
        entry.set_note(note(self.note.as_deref()).map_err(|Invalid| bad("note"))?);
        for set in load_sets(conn, id)? {
            entry
                .push_set(set)
                .map_err(|_| bad("set whose mode differs from the entry's"))?;
        }
        Ok(entry)
    }
}

/**
 * `SessionExercise::new` takes an exercise to capture its ID and tracking
 * mode. The entry keeps the mode it was created with, which is what the
 * row stores and may differ from the exercise's current mode, so this
 * builds a value carrying exactly those two. Its name and scheme are never
 * read.
 */
fn carrier(id: ExerciseId, mode: TrackingMode) -> Option<Exercise> {
    Some(Exercise {
        id,
        name: Name::new("entry").ok()?,
        mode,
        scheme: None,
    })
}
