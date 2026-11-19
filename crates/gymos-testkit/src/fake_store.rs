/*!
 * An in-memory [`Store`] with real rollback.
 *
 * [`FakeStore::write`](Store::write) runs the closure against a **clone**
 * of the state and swaps the clone in only if the closure returns `Ok`, so
 * a failed command leaves the store exactly as it was, the same guarantee a
 * SQLite transaction gives. The contract scenarios in
 * [`scenarios`](crate::scenarios) hold it to the same behaviour as the real
 * store.
 */

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use gymos_domain::{Exercise, ExerciseId, Session, SessionId};
use gymos_engine::{CommandId, EngineError, ReadTx, RepoError, SessionHeader, Store, WriteTx};

/** Everything a [`FakeStore`] holds. Compare two snapshots to prove nothing changed. */
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FakeState {
    exercises: BTreeMap<ExerciseId, Exercise>,
    sessions: BTreeMap<SessionId, Session>,
    applied: BTreeSet<CommandId>,
}

impl FakeState {
    /** Every stored exercise, by ID. */
    pub const fn exercises(&self) -> &BTreeMap<ExerciseId, Exercise> {
        &self.exercises
    }

    /** Every stored session, by ID. */
    pub const fn sessions(&self) -> &BTreeMap<SessionId, Session> {
        &self.sessions
    }

    /** Every command ID recorded as applied. */
    pub const fn applied(&self) -> &BTreeSet<CommandId> {
        &self.applied
    }

    fn exercise(&self, id: ExerciseId) -> Option<Exercise> {
        self.exercises.get(&id).cloned()
    }

    fn session(&self, id: SessionId) -> Option<Session> {
        self.sessions.get(&id).cloned()
    }

    fn sessions_containing(&self, exercise: ExerciseId) -> Vec<SessionHeader> {
        self.sessions
            .values()
            .filter(|s| s.exercises().iter().any(|e| e.exercise() == exercise))
            .map(SessionHeader::of)
            .collect()
    }
}

/**
 * The in-memory store. Cloning it gives a second handle on the **same**
 * state, so a test can hand one handle to `GymOs` and keep one to inspect.
 */
#[derive(Debug, Clone, Default)]
pub struct FakeStore {
    state: Arc<Mutex<FakeState>>,
}

impl FakeStore {
    /** An empty store. */
    pub fn new() -> Self {
        Self::default()
    }

    /** A copy of everything stored right now. */
    pub fn snapshot(&self) -> FakeState {
        self.lock().clone()
    }

    /**
     * Locks the state. A panic inside a closure can poison the lock, but
     * the state is only ever replaced after a closure succeeds, so the data
     * behind a poisoned lock is still consistent and safe to keep using.
     */
    fn lock(&self) -> MutexGuard<'_, FakeState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/** A read-only view of a [`FakeStore`]. */
#[derive(Debug)]
pub struct FakeRead<'t> {
    state: &'t FakeState,
}

/** A transaction on a [`FakeStore`]: a private copy of the state. */
#[derive(Debug)]
pub struct FakeWrite {
    state: FakeState,
}

impl Store for FakeStore {
    type Read<'t> = FakeRead<'t>;
    type Write<'t> = FakeWrite;

    fn read<R>(
        &self,
        f: impl FnOnce(&Self::Read<'_>) -> Result<R, EngineError>,
    ) -> Result<R, EngineError> {
        let guard = self.lock();
        f(&FakeRead { state: &guard })
    }

    fn write<R>(
        &self,
        f: impl FnOnce(&mut Self::Write<'_>) -> Result<R, EngineError>,
    ) -> Result<R, EngineError> {
        let mut guard = self.lock();
        let mut tx = FakeWrite {
            state: guard.clone(),
        };
        let result = f(&mut tx)?;
        *guard = tx.state;
        Ok(result)
    }
}

impl ReadTx for FakeRead<'_> {
    fn exercise(&self, id: ExerciseId) -> Result<Option<Exercise>, RepoError> {
        Ok(self.state.exercise(id))
    }

    fn session(&self, id: SessionId) -> Result<Option<Session>, RepoError> {
        Ok(self.state.session(id))
    }

    fn sessions_containing(&self, exercise: ExerciseId) -> Result<Vec<SessionHeader>, RepoError> {
        Ok(self.state.sessions_containing(exercise))
    }
}

impl ReadTx for FakeWrite {
    fn exercise(&self, id: ExerciseId) -> Result<Option<Exercise>, RepoError> {
        Ok(self.state.exercise(id))
    }

    fn session(&self, id: SessionId) -> Result<Option<Session>, RepoError> {
        Ok(self.state.session(id))
    }

    fn sessions_containing(&self, exercise: ExerciseId) -> Result<Vec<SessionHeader>, RepoError> {
        Ok(self.state.sessions_containing(exercise))
    }
}

impl WriteTx for FakeWrite {
    fn mark_applied(&mut self, command: CommandId) -> Result<bool, RepoError> {
        Ok(self.state.applied.insert(command))
    }

    fn save_exercise(&mut self, exercise: &Exercise) -> Result<(), RepoError> {
        self.state.exercises.insert(exercise.id, exercise.clone());
        Ok(())
    }

    fn save_session(&mut self, session: &Session) -> Result<(), RepoError> {
        self.state.sessions.insert(session.id(), session.clone());
        Ok(())
    }
}
