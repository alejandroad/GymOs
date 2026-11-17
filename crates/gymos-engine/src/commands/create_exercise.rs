/*! `CreateExercise`: adds an exercise to the catalog. */

use gymos_domain::progression::ProgressionScheme;
use gymos_domain::{Exercise, ExerciseId, Name, TrackingMode};

use crate::command::{CommandId, CommandOutcome};
use crate::error::EngineError;
use crate::gymos::GymOs;
use crate::ports::{IdGen, Ports, WriteTx};

/**
 * Adds an exercise. The engine generates its ID and returns it in
 * [`CommandOutcome::ExerciseCreated`].
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateExercise {
    /** Display name, such as "Bench Press". */
    pub name: Name,
    /**
     * How its sets are measured. It can't change once sets exist (D-4), so
     * there is no command to change it yet.
     */
    pub mode: TrackingMode,
    /** How it progresses, if the user chose a scheme (D-2). */
    pub scheme: Option<ProgressionScheme>,
}

pub(super) fn run<P: Ports>(
    os: &GymOs<P>,
    id: CommandId,
    command: CreateExercise,
) -> Result<CommandOutcome, EngineError> {
    super::write(os, id, |tx| {
        let exercise = Exercise {
            id: ExerciseId::from_uuid(os.ids.next_id()),
            name: command.name,
            mode: command.mode,
            scheme: command.scheme,
        };
        tx.save_exercise(&exercise)?;
        Ok(CommandOutcome::ExerciseCreated(exercise.id))
    })
}
