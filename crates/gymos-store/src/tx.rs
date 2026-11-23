/*!
 * Transactions: the [`Store`] implementation and the read and write views
 * the engine's closures receive (PROMPT-3 §6.2).
 *
 * - `write` opens `BEGIN IMMEDIATE`, so it takes the write lock up front
 *   and can't fail half way on a lock upgrade. It commits when the closure
 *   returns `Ok`, and rolls back on `Err` and on panic: the driver's
 *   transaction guard rolls back when dropped.
 * - `read` opens a deferred transaction, so every query in one closure sees
 *   one snapshot, and always rolls it back, so a read can never write.
 */

use core::fmt;

use gymos_domain::{Exercise, ExerciseId, Session, SessionId};
use gymos_engine::{
    CommandId, CommandOutcome, EngineError, ReadTx, RepoError, SessionHeader, Store, WriteTx,
};
use rusqlite::{Connection, TransactionBehavior};

use crate::error::IntoRepo;
use crate::rows;
use crate::sqlite_store::SqliteStore;

/** A read-only view of a [`SqliteStore`], valid for one `read` call. */
pub struct SqliteRead<'t> {
    conn: &'t Connection,
}

/** A transaction on a [`SqliteStore`], valid for one `write` call. */
pub struct SqliteWrite<'t> {
    conn: &'t Connection,
}

impl fmt::Debug for SqliteRead<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SqliteRead").finish_non_exhaustive()
    }
}

impl fmt::Debug for SqliteWrite<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SqliteWrite").finish_non_exhaustive()
    }
}

impl Store for SqliteStore {
    type Read<'t> = SqliteRead<'t>;
    type Write<'t> = SqliteWrite<'t>;

    fn read<R>(
        &self,
        f: impl FnOnce(&Self::Read<'_>) -> Result<R, EngineError>,
    ) -> Result<R, EngineError> {
        let mut conn = self.lock()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .repo()?;
        let result = f(&SqliteRead { conn: &tx });
        let ended = tx.rollback().repo();
        let value = result?;
        ended?;
        Ok(value)
    }

    fn write<R>(
        &self,
        f: impl FnOnce(&mut Self::Write<'_>) -> Result<R, EngineError>,
    ) -> Result<R, EngineError> {
        let mut conn = self.lock()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .repo()?;
        let value = f(&mut SqliteWrite { conn: &tx })?;
        tx.commit().repo()?;
        Ok(value)
    }
}

impl ReadTx for SqliteRead<'_> {
    fn exercise(&self, id: ExerciseId) -> Result<Option<Exercise>, RepoError> {
        rows::load_exercise(self.conn, id)
    }

    fn session(&self, id: SessionId) -> Result<Option<Session>, RepoError> {
        rows::load_session(self.conn, id)
    }

    fn sessions_containing(&self, exercise: ExerciseId) -> Result<Vec<SessionHeader>, RepoError> {
        rows::sessions_containing(self.conn, exercise)
    }
}

impl ReadTx for SqliteWrite<'_> {
    fn exercise(&self, id: ExerciseId) -> Result<Option<Exercise>, RepoError> {
        rows::load_exercise(self.conn, id)
    }

    fn session(&self, id: SessionId) -> Result<Option<Session>, RepoError> {
        rows::load_session(self.conn, id)
    }

    fn sessions_containing(&self, exercise: ExerciseId) -> Result<Vec<SessionHeader>, RepoError> {
        rows::sessions_containing(self.conn, exercise)
    }
}

impl WriteTx for SqliteWrite<'_> {
    fn begin_command(&mut self, command: CommandId) -> Result<Option<CommandOutcome>, RepoError> {
        rows::load_outcome(self.conn, command)
    }

    fn finish_command(
        &mut self,
        command: CommandId,
        outcome: CommandOutcome,
    ) -> Result<(), RepoError> {
        rows::save_outcome(self.conn, command, outcome)
    }

    fn save_exercise(&mut self, exercise: &Exercise) -> Result<(), RepoError> {
        rows::save_exercise(self.conn, exercise)
    }

    fn save_session(&mut self, session: &Session) -> Result<(), RepoError> {
        rows::save_session(self.conn, session)
    }
}
