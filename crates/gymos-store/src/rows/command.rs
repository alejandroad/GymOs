/*!
 * The `applied_commands` table: each applied command's ID and outcome, so
 * a replay returns the same answer (DD-15, S-7).
 */

use gymos_engine::{CommandId, CommandOutcome, RepoError};
use rusqlite::{Connection, OptionalExtension, params};

use super::{blob, codes, uuid};
use crate::error::{IntoRepo, corrupt};

const TABLE: &str = "applied_commands";

/** Records `command` as applied with `outcome`. */
pub(crate) fn save_outcome(
    conn: &Connection,
    command: CommandId,
    outcome: CommandOutcome,
) -> Result<(), RepoError> {
    let (kind, id) = codes::outcome_code(outcome);
    conn.prepare_cached(
        "INSERT INTO applied_commands (id, outcome_kind, outcome_id) VALUES (?1, ?2, ?3)",
    )
    .repo()?
    .execute(params![blob(command.as_uuid()), kind, id.map(blob)])
    .repo()?;
    Ok(())
}

/** The outcome `command` was applied with, or `None` if it never was. */
pub(crate) fn load_outcome(
    conn: &Connection,
    command: CommandId,
) -> Result<Option<CommandOutcome>, RepoError> {
    let raw: Option<(String, Option<Vec<u8>>)> = conn
        .prepare_cached("SELECT outcome_kind, outcome_id FROM applied_commands WHERE id = ?1")
        .repo()?
        .query_row([blob(command.as_uuid())], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .optional()
        .repo()?;
    let Some((kind, raw_id)) = raw else {
        return Ok(None);
    };
    let bad = || corrupt(TABLE, &command, "outcome");
    let id = match raw_id {
        None => None,
        Some(raw) => Some(uuid(&raw).ok_or_else(bad)?),
    };
    codes::outcome(&kind, id).map(Some).ok_or_else(bad)
}
