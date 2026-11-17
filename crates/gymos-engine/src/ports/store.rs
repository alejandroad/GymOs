/*!
 * The store port: a closure-scoped unit of work (kernel-architecture §5.3).
 *
 * The engine decides *what* belongs in one transaction by doing all of it
 * inside one [`Store::write`] closure. The store decides *how* to make that
 * atomic: a SQLite transaction in `gymos-store`, a cloned state in the
 * testkit fake.
 *
 * Writes work on whole aggregates. There are no per-field setters, so
 * business sequencing never leaks into an adapter, and each domain type
 * checks its own rules before it is saved.
 */

use gymos_domain::{Exercise, ExerciseId, Session, SessionId, TemplateId, Timestamp};

use crate::command::CommandId;
use crate::error::{EngineError, RepoError};

/**
 * Persistent storage for exercises, sessions and applied commands.
 *
 * Every command makes exactly one [`write`](Store::write) call. Queries use
 * [`read`](Store::read).
 */
pub trait Store: Send + Sync {
    /** A read-only view, valid for one [`read`](Store::read) call. */
    type Read<'t>: ReadTx;
    /** A read-write transaction, valid for one [`write`](Store::write) call. */
    type Write<'t>: WriteTx;

    /**
     * Runs `f` against a consistent, read-only view of the store.
     *
     * # Errors
     *
     * Whatever `f` returns, or [`EngineError::Repo`] if the view can't be
     * opened.
     */
    fn read<R>(
        &self,
        f: impl FnOnce(&Self::Read<'_>) -> Result<R, EngineError>,
    ) -> Result<R, EngineError>;

    /**
     * Runs `f` in one transaction. Everything `f` wrote is committed if it
     * returns `Ok`, and nothing is if it returns `Err`.
     *
     * # Errors
     *
     * Whatever `f` returns (after rolling back), or [`EngineError::Repo`] if
     * the transaction can't be opened or committed.
     */
    fn write<R>(
        &self,
        f: impl FnOnce(&mut Self::Write<'_>) -> Result<R, EngineError>,
    ) -> Result<R, EngineError>;
}

/**
 * What a "last time" search needs to know about a session, without loading
 * its sets. Adapters build it with [`SessionHeader::of`] or from an indexed
 * query.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionHeader {
    /** The session. */
    pub id: SessionId,
    /** When it started. */
    pub started_at: Timestamp,
    /** When it ended, or `None` while unfinished. */
    pub ended_at: Option<Timestamp>,
    /** The template it was started from. */
    pub template: Option<TemplateId>,
    /** Whether the user marked it as a deload. */
    pub deload: bool,
}

impl SessionHeader {
    /** The header of a loaded session. */
    pub const fn of(session: &Session) -> Self {
        Self {
            id: session.id(),
            started_at: session.started_at(),
            ended_at: session.ended_at(),
            template: session.template(),
            deload: session.is_deload(),
        }
    }
}

/** Reads, inside either kind of transaction. */
pub trait ReadTx {
    /**
     * Loads one exercise.
     *
     * # Errors
     *
     * [`RepoError`] if the store fails or the stored row is invalid.
     */
    fn exercise(&self, id: ExerciseId) -> Result<Option<Exercise>, RepoError>;

    /**
     * Loads one session with all its entries and sets.
     *
     * # Errors
     *
     * [`RepoError`] if the store fails or the stored rows are invalid.
     */
    fn session(&self, id: SessionId) -> Result<Option<Session>, RepoError>;

    /**
     * Lists every session with at least one entry for `exercise`, in no
     * particular order. Finished or not, deload or not: choosing among them
     * is a business rule, so the engine does it.
     *
     * # Errors
     *
     * [`RepoError`] if the store fails or a stored row is invalid.
     */
    fn sessions_containing(&self, exercise: ExerciseId) -> Result<Vec<SessionHeader>, RepoError>;
}

/** Writes, inside a [`Store::write`] transaction. */
pub trait WriteTx: ReadTx {
    /**
     * Records that `command` is being applied. Returns `false` if it was
     * already applied, in which case the engine changes nothing (DD-15).
     *
     * # Errors
     *
     * [`RepoError`] if the store fails.
     */
    fn mark_applied(&mut self, command: CommandId) -> Result<bool, RepoError>;

    /**
     * Inserts or replaces an exercise.
     *
     * # Errors
     *
     * [`RepoError`] if the store fails.
     */
    fn save_exercise(&mut self, exercise: &Exercise) -> Result<(), RepoError>;

    /**
     * Inserts or replaces a whole session: its fields, entries and sets.
     * Anything stored for the session but missing from `session` is
     * removed.
     *
     * # Errors
     *
     * [`RepoError`] if the store fails.
     */
    fn save_session(&mut self, session: &Session) -> Result<(), RepoError>;
}
