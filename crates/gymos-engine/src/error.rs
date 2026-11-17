/*!
 * Engine errors (DD-07).
 *
 * [`EngineError`] is the one error type the engine returns. It wraps domain
 * errors unchanged and storage failures as [`RepoError`], which the engine
 * defines so that no adapter's error type (such as `rusqlite::Error`) ever
 * appears here. The shell maps `EngineError` once into stable IPC codes.
 */

use gymos_domain::progression::VerdictError;
use gymos_domain::{ExerciseId, SessionError, SessionExerciseId, SessionId};
use thiserror::Error;

/**
 * A storage failure, reported by a store adapter.
 *
 * The messages are for local logs only; the shell never forwards them to the
 * UI (DD-07: no information leakage).
 */
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RepoError {
    /** The store could not be reached or the operation failed in it. */
    #[error("storage failed: {0}")]
    Backend(String),
    /**
     * Stored data could not be turned back into domain values, so the
     * database holds something the current rules reject.
     */
    #[error("stored data is corrupt: {0}")]
    Corrupt(String),
}

/** A rule from `gymos-domain` that refused a change or a computation. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DomainError {
    /** A session or one of its entries rejected the change. */
    #[error(transparent)]
    Session(#[from] SessionError),
    /**
     * A verdict could not be computed. Because the engine never pairs
     * different exercises and a tracking mode can't change once sets exist
     * (D-4), this means stored data is inconsistent.
     */
    #[error(transparent)]
    Verdict(#[from] VerdictError),
}

/** Something a command or query named that doesn't exist. */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    /** No exercise with this ID. */
    Exercise(ExerciseId),
    /** No session with this ID. */
    Session(SessionId),
    /** The session exists but has no entry with this ID. */
    Entry {
        /** The session that was searched. */
        session: SessionId,
        /** The entry that wasn't in it. */
        entry: SessionExerciseId,
    },
}

impl core::fmt::Display for Missing {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Exercise(id) => write!(f, "exercise {id}"),
            Self::Session(id) => write!(f, "session {id}"),
            Self::Entry { session, entry } => write!(f, "entry {entry} in session {session}"),
        }
    }
}

/**
 * Why a command or query failed.
 *
 * A `Conflict` variant (a rule that needs history, such as refusing a
 * tracking-mode change once sets exist, D-4) arrives with the first command
 * that can produce it.
 */
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EngineError {
    /** A domain rule rejected the input. */
    #[error(transparent)]
    Domain(#[from] DomainError),
    /** The store failed. */
    #[error(transparent)]
    Repo(#[from] RepoError),
    /** A referenced entity doesn't exist. */
    #[error("{0} was not found")]
    NotFound(Missing),
}

impl From<SessionError> for EngineError {
    fn from(error: SessionError) -> Self {
        Self::Domain(DomainError::Session(error))
    }
}

impl From<VerdictError> for EngineError {
    fn from(error: VerdictError) -> Self {
        Self::Domain(DomainError::Verdict(error))
    }
}
