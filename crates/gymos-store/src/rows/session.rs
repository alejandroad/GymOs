/*!
 * The `sessions` table, and saving or loading a whole session aggregate
 * with its entries and sets (S-3).
 */

use std::collections::BTreeSet;

use gymos_domain::{
    ExerciseId, Name, Note, Session, SessionExercise, SessionId, TemplateId, Timestamp,
};
use gymos_engine::{RepoError, SessionHeader};
use rusqlite::{Connection, OptionalExtension, Row, params};

use super::entry::{load_entries, save_entry};
use super::{
    Invalid, SCHEMA_VERSION, blob, codes, flag, name, note, optional_timestamp, timestamp, uuid,
};
use crate::error::{IntoRepo, corrupt};

const TABLE: &str = "sessions";

/**
 * Writes a whole session in one pass (S-3): the session row, then every
 * entry and set by position. Children of this session that are not in
 * `session` end up tombstoned: everything is tombstoned first and each
 * row written is revived, which inside one transaction is the same as
 * tombstoning only what is missing.
 *
 * IDs are global, so the store refuses (as `Backend`, rolling the write
 * back) what it can't save without losing data: a set ID used twice in the
 * session, or an entry or set ID that belongs to another session. Neither
 * can come from the engine, whose IDs are fresh; this guards imports and
 * sync.
 */
pub(crate) fn save_session(conn: &Connection, session: &Session) -> Result<(), RepoError> {
    let mut set_ids = BTreeSet::new();
    for set in session.exercises().iter().flat_map(SessionExercise::sets) {
        if !set_ids.insert(set.id) {
            return Err(RepoError::Backend(format!(
                "session {}: set {} appears twice",
                session.id(),
                set.id
            )));
        }
    }
    save_row(conn, session)?;
    let id = blob(session.id().as_uuid());
    conn.prepare_cached(
        "UPDATE sets SET deleted = 1
         WHERE deleted = 0
           AND entry_id IN (SELECT id FROM session_entries WHERE session_id = ?1)",
    )
    .repo()?
    .execute([id])
    .repo()?;
    conn.prepare_cached(
        "UPDATE session_entries SET deleted = 1 WHERE session_id = ?1 AND deleted = 0",
    )
    .repo()?
    .execute([id])
    .repo()?;
    for (position, entry) in (0_i64..).zip(session.exercises()) {
        save_entry(conn, session.id(), position, entry)?;
    }
    Ok(())
}

fn save_row(conn: &Connection, session: &Session) -> Result<(), RepoError> {
    let (kind, format, version) = codes::source_code(session.source());
    let ended = session.ended_at();
    conn.prepare_cached(
        "INSERT INTO sessions (id, started_utc_ms, started_offset_min, ended_utc_ms,
             ended_offset_min, title, note, template_id, deload, source_kind,
             import_format, importer_version, deleted, schema_version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0, ?13)
         ON CONFLICT (id) DO UPDATE SET
             started_utc_ms = excluded.started_utc_ms,
             started_offset_min = excluded.started_offset_min,
             ended_utc_ms = excluded.ended_utc_ms,
             ended_offset_min = excluded.ended_offset_min,
             title = excluded.title,
             note = excluded.note,
             template_id = excluded.template_id,
             deload = excluded.deload,
             source_kind = excluded.source_kind,
             import_format = excluded.import_format,
             importer_version = excluded.importer_version,
             deleted = 0,
             schema_version = excluded.schema_version",
    )
    .repo()?
    .execute(params![
        blob(session.id().as_uuid()),
        session.started_at().unix_millis(),
        session.started_at().offset().minutes(),
        ended.map(Timestamp::unix_millis),
        ended.map(|t| t.offset().minutes()),
        session.title().map(Name::as_str),
        session.note().map(Note::as_str),
        session.template().map(|t| blob(t.as_uuid())),
        session.is_deload(),
        kind,
        format,
        version,
        SCHEMA_VERSION,
    ])
    .repo()?;
    Ok(())
}

/** Loads a live session with its live entries and sets. */
pub(crate) fn load_session(conn: &Connection, id: SessionId) -> Result<Option<Session>, RepoError> {
    let raw = conn
        .prepare_cached(
            "SELECT id, started_utc_ms, started_offset_min, ended_utc_ms, ended_offset_min,
                    template_id, deload, title, note, source_kind, import_format,
                    importer_version
             FROM sessions WHERE id = ?1 AND deleted = 0",
        )
        .repo()?
        .query_row([blob(id.as_uuid())], Raw::read)
        .optional()
        .repo()?;
    let Some(raw) = raw else {
        return Ok(None);
    };
    let header = raw.header.into_header()?;
    let bad = |field| corrupt(TABLE, &id, field);
    let mut session = Session::start(
        id,
        header.started_at,
        codes::source(
            &raw.source_kind,
            raw.import_format.as_deref(),
            raw.importer_version,
        )
        .ok_or_else(|| bad("source"))?,
    );
    session.set_template(header.template);
    session.set_deload(header.deload);
    session.set_title(match raw.title.as_deref() {
        None => None,
        Some(title) => Some(name(title).ok_or_else(|| bad("title"))?),
    });
    session.set_note(note(raw.note.as_deref()).map_err(|Invalid| bad("note"))?);
    for entry in load_entries(conn, id)? {
        session
            .add_exercise(entry)
            .map_err(|_| bad("entry, which appears twice"))?;
    }
    if let Some(ended_at) = header.ended_at {
        session
            .finish(ended_at)
            .map_err(|_| bad("ended_utc_ms, which is before the start"))?;
    }
    Ok(Some(session))
}

/**
 * Every live session with a live entry for `exercise`, oldest first. The
 * store applies no business rule here (PROMPT-3 §6.1): unfinished and
 * deload sessions are listed too.
 */
pub(crate) fn sessions_containing(
    conn: &Connection,
    exercise: ExerciseId,
) -> Result<Vec<SessionHeader>, RepoError> {
    let mut statement = conn
        .prepare_cached(
            "SELECT s.id, s.started_utc_ms, s.started_offset_min, s.ended_utc_ms,
                    s.ended_offset_min, s.template_id, s.deload
             FROM sessions AS s
             WHERE s.deleted = 0
               AND EXISTS (SELECT 1 FROM session_entries AS e
                           WHERE e.session_id = s.id AND e.exercise_id = ?1
                             AND e.deleted = 0)
             ORDER BY s.started_utc_ms, s.id",
        )
        .repo()?;
    let rows = statement
        .query_map([blob(exercise.as_uuid())], RawHeader::read)
        .repo()?;
    let mut headers = Vec::new();
    for raw in rows {
        headers.push(raw.repo()?.into_header()?);
    }
    Ok(headers)
}

/** The columns a [`SessionHeader`] needs, before validation. */
struct RawHeader {
    id: Vec<u8>,
    started_utc_ms: i64,
    started_offset_min: i64,
    ended_utc_ms: Option<i64>,
    ended_offset_min: Option<i64>,
    template: Option<Vec<u8>>,
    deload: i64,
}

impl RawHeader {
    /** Reads columns 0 to 6. */
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            started_utc_ms: row.get(1)?,
            started_offset_min: row.get(2)?,
            ended_utc_ms: row.get(3)?,
            ended_offset_min: row.get(4)?,
            template: row.get(5)?,
            deload: row.get(6)?,
        })
    }

    fn into_header(self) -> Result<SessionHeader, RepoError> {
        let id = uuid(&self.id)
            .map(SessionId::from_uuid)
            .ok_or_else(|| corrupt(TABLE, &"<invalid id>", "id"))?;
        let bad = |field| corrupt(TABLE, &id, field);
        Ok(SessionHeader {
            id,
            started_at: timestamp(self.started_utc_ms, self.started_offset_min)
                .ok_or_else(|| bad("started_offset_min"))?,
            ended_at: optional_timestamp(self.ended_utc_ms, self.ended_offset_min)
                .map_err(|Invalid| bad("ended_offset_min"))?,
            template: match self.template {
                None => None,
                Some(raw) => Some(
                    uuid(&raw)
                        .map(TemplateId::from_uuid)
                        .ok_or_else(|| bad("template_id"))?,
                ),
            },
            deload: flag(self.deload).ok_or_else(|| bad("deload"))?,
        })
    }
}

/** Every column of a session row, before validation. */
struct Raw {
    header: RawHeader,
    title: Option<String>,
    note: Option<String>,
    source_kind: String,
    import_format: Option<String>,
    importer_version: Option<i64>,
}

impl Raw {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            header: RawHeader::read(row)?,
            title: row.get(7)?,
            note: row.get(8)?,
            source_kind: row.get(9)?,
            import_format: row.get(10)?,
            importer_version: row.get(11)?,
        })
    }
}
