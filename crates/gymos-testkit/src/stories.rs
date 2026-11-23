/*!
 * Store-generic engine stories (S-9).
 *
 * Each story drives a [`TestGym`] through real commands and asserts the
 * verdicts and baselines a lifter would see. The engine's own tests run
 * them over [`FakeStore`](crate::FakeStore), and `gymos-store` runs the
 * same functions over SQLite, which proves orchestration, transactions and
 * ordering behave the same on the real store.
 *
 * Expected values are worked out by hand from DD-13 and DD-15, never copied
 * from the implementation. Each story takes a fresh, empty gym.
 */

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "stories are tests: an unexpected error or outcome is a failed test"
)]

use gymos_domain::progression::{
    Direction, E1rm, ExerciseVerdict, ProgressionScheme, RepRange, Signal, Verdict,
};
use gymos_domain::{
    ALGORITHM_VERSION, ExerciseId, Load, SessionId, Source, TemplateId, Timestamp, TrackingMode,
    Uuid,
};
use gymos_engine::{
    BaselineRule, Command, CommandOutcome, EntryOutcome, FinishSession, SessionVerdicts,
    StartSession, Store,
};

use crate::builders::{SetBuilder, at, kg, reps, working};
use crate::gym::{SessionPlan, TestGym};

use Verdict::{Deload, InsufficientData, Matched, Progressed, Regressed};

/* ---- helpers ------------------------------------------------------------ */

const DAY: i64 = 86_400_000;

fn lift<S: Store + Clone>(gym: &TestGym<S>, label: &str) -> ExerciseId {
    gym.create_exercise(label, TrackingMode::WeightReps, None)
}

fn template(n: u128) -> TemplateId {
    TemplateId::from_uuid(Uuid::from_u128(0x7E << 64 | n))
}

fn sets(load: Load, reps: &[u16]) -> Vec<SetBuilder> {
    reps.iter().map(|&n| working(load, n)).collect()
}

/** Logs a finished session with one entry and returns its ID. */
fn log<S: Store + Clone>(
    gym: &TestGym<S>,
    exercise: ExerciseId,
    sets: Vec<SetBuilder>,
) -> SessionId {
    gym.log_session(&SessionPlan::new().exercise(exercise, sets))
        .session
}

/** Logs a finished, one-hour session at an explicit time, as an importer would. */
fn log_at<S: Store + Clone>(
    gym: &TestGym<S>,
    started_at: Timestamp,
    exercise: ExerciseId,
    sets: &[SetBuilder],
) -> SessionId {
    let outcome = gym
        .run(StartSession {
            started_at: Some(started_at),
            template: None,
            title: None,
            source: Source::Manual,
        })
        .expect("start succeeds");
    let CommandOutcome::SessionStarted(session) = outcome else {
        panic!("expected SessionStarted, got {outcome:?}");
    };
    let entry = gym.add_exercise(session, exercise);
    for set in sets {
        gym.log_set(session, entry, set);
    }
    gym.run(FinishSession {
        session,
        ended_at: Some(Timestamp::new(
            started_at.unix_millis() + 3_600_000,
            started_at.offset(),
        )),
    })
    .expect("finish succeeds");
    session
}

fn verdicts<S: Store + Clone>(gym: &TestGym<S>, session: SessionId) -> SessionVerdicts {
    gym.os()
        .session_verdicts(session)
        .expect("verdicts are computed")
}

/** The verdict of a session's only entry. */
fn verdict_of<S: Store + Clone>(gym: &TestGym<S>, session: SessionId) -> ExerciseVerdict {
    let verdicts = verdicts(gym, session);
    let [entry] = verdicts.entries.as_slice() else {
        panic!("expected one entry, got {}", verdicts.entries.len());
    };
    match &entry.outcome {
        EntryOutcome::Verdict(verdict) => verdict.clone(),
        EntryOutcome::Unsupported(mode) => panic!("unexpected unsupported {mode:?}"),
    }
}

fn verdicts_of<S: Store + Clone>(gym: &TestGym<S>, sessions: &[SessionId]) -> Vec<Verdict> {
    sessions
        .iter()
        .map(|&s| verdict_of(gym, s).verdict)
        .collect()
}

/** The baseline session and rule of a session's first entry. */
fn baseline_of<S: Store + Clone>(
    gym: &TestGym<S>,
    session: SessionId,
) -> Option<(SessionId, BaselineRule)> {
    verdicts(gym, session)
        .entries
        .first()
        .expect("the session has an entry")
        .baseline
        .as_ref()
        .map(|b| (b.session, b.rule))
}

fn e1rm(load: Load, n: u16) -> E1rm {
    E1rm::of_set(load, reps(n), None).expect("e1RM is defined at these reps")
}

/** `kg_reps(2300)` is 2300 kg·reps in nanogram-reps. */
const fn kg_reps(whole: i128) -> i128 {
    whole * 1_000_000_000_000
}

/* ---- stories ------------------------------------------------------------ */

/**
 * The walking skeleton: two sessions of bench, then golden example A
 * exactly: reps ↑ on set 2 (8→9), e1RM ↑ 126.67→130.00 kg, volume ↑
 * 2300→2400 kg·reps.
 *
 * # Panics
 *
 * If the engine or store misbehaves.
 */
pub fn walking_skeleton_matches_golden_example_a<S: Store + Clone>(gym: &TestGym<S>) {
    let bench = lift(gym, "Bench Press");
    let k = kg(100, 0);
    let first = gym.log_session(&SessionPlan::new().exercise(bench, sets(k, &[8, 8, 7])));
    let second = gym.log_session(&SessionPlan::new().exercise(bench, sets(k, &[8, 9, 7])));

    let verdicts = verdicts(gym, second.session);
    assert_eq!(verdicts.session, second.session);
    let [entry] = verdicts.entries.as_slice() else {
        panic!("expected one entry");
    };
    assert_eq!(
        (Some(&entry.entry), entry.exercise),
        (second.entries.first(), bench)
    );
    let baseline = entry
        .baseline
        .as_ref()
        .expect("the first session is the baseline");
    assert_eq!(
        (baseline.session, baseline.rule),
        (first.session, BaselineRule::SameExercise)
    );
    assert_eq!(Some(&baseline.entry.id()), first.entries.first());

    let EntryOutcome::Verdict(v) = &entry.outcome else {
        panic!("expected a verdict");
    };
    assert_eq!(v.verdict, Progressed);
    assert!(v.is_progressive_overload());
    assert_eq!(
        (v.skipped.as_slice(), v.algorithm_version),
        (&[][..], ALGORITHM_VERSION)
    );
    let [e1rm_signal, reps_signal, volume_signal] = v.signals.as_slice() else {
        panic!("expected three signals, got {:#?}", v.signals);
    };
    assert_eq!(
        *e1rm_signal,
        Signal::E1rm {
            direction: Direction::Up,
            before: e1rm(k, 8),
            after: e1rm(k, 9),
        }
    );
    assert!((e1rm(k, 8).kilograms() - 126.666_666).abs() < 1e-3);
    assert!((e1rm(k, 9).kilograms() - 130.0).abs() < 1e-9);
    assert_eq!(
        *reps_signal,
        Signal::RepsAtLoad {
            direction: Direction::Up,
            set_index: 1,
            load: k,
            before: reps(8),
            after: reps(9),
        }
    );
    let Signal::VolumeLoad {
        direction,
        before,
        after,
    } = *volume_signal
    else {
        panic!("expected VolumeLoad, got {volume_signal:?}");
    };
    assert_eq!(
        (direction, before.nanogram_reps(), after.nanogram_reps()),
        (Direction::Up, kg_reps(2300), kg_reps(2400))
    );
    assert_eq!((verdicts.tally.progressed, verdicts.tally.total()), (1, 1));
}

/**
 * Double progression on a 6–8 scheme: reps climb to the top of the range,
 * the load steps up and reps fall back (still progress, DD-13 gap 1), then
 * a matched week and a stall.
 *
 * # Panics
 *
 * If the engine or store misbehaves.
 */
pub fn a_double_progression_cycle_reads_as_progress_until_it_stalls<S: Store + Clone>(
    gym: &TestGym<S>,
) {
    let scheme = ProgressionScheme::new(
        RepRange::new(reps(6), reps(8)).expect("valid range"),
        kg(2, 50),
    )
    .expect("valid scheme");
    let bench = gym.create_exercise("Bench Press", TrackingMode::WeightReps, Some(scheme));
    let cycle: Vec<SessionId> = [
        (kg(100, 0), [6, 6, 6]),
        (kg(100, 0), [7, 6, 6]),
        (kg(100, 0), [8, 7, 7]),
        (kg(100, 0), [8, 8, 8]),
        /* Top of the range reached: add the increment, reps fall back. */
        (kg(102, 50), [6, 6, 6]),
        (kg(102, 50), [6, 6, 6]),
        (kg(102, 50), [6, 6, 5]),
    ]
    .iter()
    .map(|(load, r)| log(gym, bench, sets(*load, r)))
    .collect();

    assert_eq!(
        verdicts_of(gym, &cycle),
        [
            InsufficientData,
            Progressed,
            Progressed,
            Progressed,
            Progressed,
            Matched,
            Regressed
        ]
    );
    let [.., load_step_week, _, stall_week] = cycle.as_slice() else {
        panic!("seven sessions were logged");
    };
    let load_step = verdict_of(gym, *load_step_week);
    assert_eq!(
        load_step.signals.first(),
        Some(&Signal::E1rm {
            direction: Direction::Down,
            before: e1rm(kg(100, 0), 8),
            after: e1rm(kg(102, 50), 6),
        }),
        "the e1RM drop is still listed, so the explanation stays honest"
    );
    assert!(load_step.signals.contains(&Signal::LoadStep {
        before: kg(100, 0),
        after: kg(102, 50),
        reps: reps(6),
    }));
    let stall = verdict_of(gym, *stall_week);
    assert!(stall.signals.contains(&Signal::RepsAtLoad {
        direction: Direction::Down,
        set_index: 2,
        load: kg(102, 50),
        before: reps(6),
        after: reps(5),
    }));
}

/**
 * A deload week reads as `Deload`, is never a baseline, and the week after
 * it compares with the last real week (DD-13 gap 2).
 *
 * # Panics
 *
 * If the engine or store misbehaves.
 */
pub fn a_deload_week_is_skipped_and_the_block_resumes_from_before_it<S: Store + Clone>(
    gym: &TestGym<S>,
) {
    let squat = lift(gym, "Squat");
    let w1 = log(gym, squat, sets(kg(100, 0), &[5, 5, 5]));
    let w2 = log(gym, squat, sets(kg(102, 50), &[5, 5, 5]));
    let w3 = log(gym, squat, sets(kg(105, 0), &[5, 5, 5]));
    let w4 = gym
        .log_session(
            &SessionPlan::new()
                .deload()
                .exercise(squat, sets(kg(80, 0), &[5, 5, 5])),
        )
        .session;
    let w5 = log(gym, squat, sets(kg(105, 0), &[5, 5, 5]));
    let w6 = log(gym, squat, sets(kg(107, 50), &[5, 5, 5]));

    assert_eq!(
        verdicts_of(gym, &[w1, w2, w3, w4, w5, w6]),
        [
            InsufficientData,
            Progressed,
            Progressed,
            Deload,
            Matched,
            Progressed
        ]
    );
    let baseline = |s| baseline_of(gym, s).map(|(session, _)| session);
    assert_eq!(
        baseline(w4),
        Some(w3),
        "a deload is still compared, for its signals"
    );
    assert_eq!(baseline(w5), Some(w3));
    assert_eq!(baseline(w6), Some(w5));
}

/**
 * Two templates alternate; once each has history, each compares with its
 * own previous session (DD-15 rule 1).
 *
 * # Panics
 *
 * If the engine or store misbehaves.
 */
pub fn rotating_templates_each_progress_against_their_own_history<S: Store + Clone>(
    gym: &TestGym<S>,
) {
    let bench = lift(gym, "Bench Press");
    let (heavy, volume) = (template(1), template(2));
    let session = |template, load, r: &[u16]| {
        gym.log_session(
            &SessionPlan::new()
                .template(template)
                .exercise(bench, sets(load, r)),
        )
        .session
    };
    let a1 = session(heavy, kg(100, 0), &[5, 5, 5]);
    let b1 = session(volume, kg(70, 0), &[10, 10, 10]);
    let a2 = session(heavy, kg(102, 50), &[5, 5, 5]);
    let b2 = session(volume, kg(70, 0), &[11, 10, 10]);
    let a3 = session(heavy, kg(105, 0), &[5, 5, 5]);
    let b3 = session(volume, kg(70, 0), &[12, 11, 10]);

    /* B1 has no earlier volume day, so it falls back to the heavy day and
     * reads as a regression (e1RM 93.33 vs 116.67 kg). From then on each
     * template compares with itself. */
    assert_eq!(
        verdicts_of(gym, &[a1, b1, a2, b2, a3, b3]),
        [
            InsufficientData,
            Regressed,
            Progressed,
            Progressed,
            Progressed,
            Progressed
        ]
    );
    assert_eq!(baseline_of(gym, b1), Some((a1, BaselineRule::SameExercise)));
    assert_eq!(
        baseline_of(gym, a2),
        Some((a1, BaselineRule::SameTemplate(heavy)))
    );
    assert_eq!(
        baseline_of(gym, b2),
        Some((b1, BaselineRule::SameTemplate(volume)))
    );
    assert_eq!(
        baseline_of(gym, b3),
        Some((b2, BaselineRule::SameTemplate(volume)))
    );
}

/**
 * A forgotten session imported later, dated between two others, becomes
 * the later one's baseline and changes its verdict (DD-15 rule 3).
 *
 * # Panics
 *
 * If the engine or store misbehaves.
 */
pub fn importing_an_older_session_moves_the_baseline_of_later_ones<S: Store + Clone>(
    gym: &TestGym<S>,
) {
    let squat = lift(gym, "Squat");
    let baseline = |s| baseline_of(gym, s).map(|(session, _)| session);
    let jan_1 = log_at(gym, at(1_767_225_600_000), squat, &sets(kg(100, 0), &[5]));
    let jan_15 = log_at(
        gym,
        at(1_767_225_600_000 + 14 * DAY),
        squat,
        &sets(kg(105, 0), &[5]),
    );
    assert_eq!(
        (baseline(jan_15), verdict_of(gym, jan_15).verdict),
        (Some(jan_1), Progressed)
    );

    /* A forgotten workout from Jan 8 is imported afterwards. */
    let jan_8 = log_at(
        gym,
        at(1_767_225_600_000 + 7 * DAY),
        squat,
        &sets(kg(107, 50), &[5]),
    );
    assert_eq!(
        (baseline(jan_8), verdict_of(gym, jan_8).verdict),
        (Some(jan_1), Progressed)
    );
    assert_eq!(
        (baseline(jan_15), verdict_of(gym, jan_15).verdict),
        (Some(jan_8), Regressed)
    );
}

/**
 * Replaying a command changes nothing and returns the first outcome again
 * (S-7), so verdicts stay as they were after the first application.
 *
 * # Panics
 *
 * If the engine or store misbehaves.
 */
pub fn a_replayed_command_never_changes_a_verdict<S: Store + Clone>(gym: &TestGym<S>) {
    let squat = lift(gym, "Squat");
    log(gym, squat, sets(kg(100, 0), &[5]));
    let today = gym.log_session(&SessionPlan::new().exercise(squat, sets(kg(100, 0), &[5])));
    let before = verdicts(gym, today.session);

    let entry = *today.entries.first().expect("one entry was logged");
    let extra = Command::new(
        gym.command_id(),
        working(kg(100, 0), 5).log_into(today.session, entry),
    );
    let first = gym.os().execute(extra.clone()).expect("the set is logged");
    assert!(matches!(first, CommandOutcome::SetLogged(_)), "{first:?}");
    let after_first = verdicts(gym, today.session);
    assert_ne!(after_first, before, "the extra set is a new working set");
    assert_eq!(gym.os().execute(extra).expect("the replay succeeds"), first);
    assert_eq!(verdicts(gym, today.session), after_first);
}
