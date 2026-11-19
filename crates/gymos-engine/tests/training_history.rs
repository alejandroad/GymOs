/*!
 * Training histories: how verdicts read across weeks of real training.
 *
 * Each test logs a realistic sequence of sessions through the engine and
 * asserts the verdict of every session in order. Expected values are worked
 * out by hand from DD-13 and DD-15, never copied from the implementation.
 * These lock in what "getting better" means over time, so a rule change
 * that breaks one of these stories must be a deliberate decision.
 */

#![expect(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers build fixtures from known-valid literals; a panic is a failed test"
)]

use gymos_domain::progression::{
    Direction, ExerciseVerdict, ProgressionScheme, RepRange, Signal, Verdict,
};
use gymos_domain::{
    ALGORITHM_VERSION, Effort, ExerciseId, Load, Rir, SessionId, Source, TemplateId, Timestamp,
    TrackingMode, Uuid,
};
use gymos_engine::{
    BaselineRule, Command, CommandOutcome, EntryOutcome, FinishSession, StartSession,
};
use gymos_testkit::{SessionPlan, SetBuilder, TestGym, at, kg, lb, reps, rpe, working};

use Verdict::{Deload, InsufficientData, Matched, Progressed, Regressed};

/* ---- helpers ------------------------------------------------------------ */

const DAY: i64 = 86_400_000;

fn lift(gym: &TestGym, label: &str) -> ExerciseId {
    gym.create_exercise(label, TrackingMode::WeightReps, None)
}

fn template(n: u128) -> TemplateId {
    TemplateId::from_uuid(Uuid::from_u128(0x7E << 64 | n))
}

fn sets(load: Load, reps: &[u16]) -> Vec<SetBuilder> {
    reps.iter().map(|&n| working(load, n)).collect()
}

fn rir(n: u8) -> Effort {
    Effort::Rir(Rir::new(n).unwrap())
}

/** Logs a finished session with one entry and returns its ID. */
fn log(gym: &TestGym, exercise: ExerciseId, sets: Vec<SetBuilder>) -> SessionId {
    gym.log_session(&SessionPlan::new().exercise(exercise, sets))
        .session
}

/** Logs a finished session at an explicit time, as an importer would. */
fn log_at(
    gym: &TestGym,
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
        .unwrap();
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
    .unwrap();
    session
}

/** The verdict of a session's only entry. */
fn verdict_of(gym: &TestGym, session: SessionId) -> ExerciseVerdict {
    let verdicts = gym.os().session_verdicts(session).unwrap();
    assert_eq!(verdicts.entries.len(), 1);
    match &verdicts.entries[0].outcome {
        EntryOutcome::Verdict(verdict) => verdict.clone(),
        EntryOutcome::Unsupported(mode) => panic!("unexpected unsupported {mode:?}"),
    }
}

fn verdicts_of(gym: &TestGym, sessions: &[SessionId]) -> Vec<Verdict> {
    sessions
        .iter()
        .map(|&s| verdict_of(gym, s).verdict)
        .collect()
}

fn baseline_of(gym: &TestGym, session: SessionId) -> Option<SessionId> {
    let verdicts = gym.os().session_verdicts(session).unwrap();
    verdicts.entries[0].baseline.as_ref().map(|b| b.session)
}

fn has_signal(verdict: &ExerciseVerdict, matches: impl Fn(&Signal) -> bool) -> bool {
    verdict.signals.iter().any(matches)
}

/* ---- getting better ----------------------------------------------------- */

#[test]
fn a_linear_progression_block_progresses_every_week() {
    let gym = TestGym::new();
    let squat = lift(&gym, "Squat");
    let block: Vec<SessionId> = [100, 102, 105, 107, 110, 112]
        .iter()
        .zip([0, 50, 0, 50, 0, 50])
        .map(|(&whole, half)| log(&gym, squat, sets(kg(whole, half), &[5, 5, 5])))
        .collect();

    assert_eq!(
        verdicts_of(&gym, &block),
        [
            InsufficientData,
            Progressed,
            Progressed,
            Progressed,
            Progressed,
            Progressed
        ]
    );
    /* Week 2: every paired set lifted more at the same reps. */
    let week_two = verdict_of(&gym, block[1]);
    for set_index in 0..3 {
        assert!(week_two.signals.contains(&Signal::LoadAtReps {
            direction: Direction::Up,
            set_index,
            reps: reps(5),
            before: kg(100, 0),
            after: kg(102, 50),
        }));
    }
    assert!(has_signal(&week_two, |s| matches!(
        s,
        Signal::E1rm {
            direction: Direction::Up,
            ..
        }
    )));
}

#[test]
fn a_double_progression_cycle_reads_as_progress_until_it_stalls() {
    let gym = TestGym::new();
    let scheme =
        ProgressionScheme::new(RepRange::new(reps(6), reps(8)).unwrap(), kg(2, 50)).unwrap();
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
    .map(|(load, r)| log(&gym, bench, sets(*load, r)))
    .collect();

    assert_eq!(
        verdicts_of(&gym, &cycle),
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
    let load_step = verdict_of(&gym, cycle[4]);
    assert_eq!(
        load_step.signals.first(),
        Some(&Signal::E1rm {
            direction: Direction::Down,
            before: gymos_domain::progression::E1rm::of_set(kg(100, 0), reps(8), None).unwrap(),
            after: gymos_domain::progression::E1rm::of_set(kg(102, 50), reps(6), None).unwrap(),
        }),
        "the e1RM drop is still listed, so the explanation stays honest"
    );
    assert!(load_step.signals.contains(&Signal::LoadStep {
        before: kg(100, 0),
        after: kg(102, 50),
        reps: reps(6),
    }));
    let stall = verdict_of(&gym, cycle[6]);
    assert!(stall.signals.contains(&Signal::RepsAtLoad {
        direction: Direction::Down,
        set_index: 2,
        load: kg(102, 50),
        before: reps(6),
        after: reps(5),
    }));
}

#[test]
fn a_plateau_matches_until_a_breakthrough() {
    let gym = TestGym::new();
    let press = lift(&gym, "Overhead Press");
    let mut weeks: Vec<SessionId> = (0..4)
        .map(|_| log(&gym, press, sets(kg(60, 0), &[5, 5, 5])))
        .collect();
    weeks.push(log(&gym, press, sets(kg(60, 0), &[6, 5, 5])));
    assert_eq!(
        verdicts_of(&gym, &weeks),
        [InsufficientData, Matched, Matched, Matched, Progressed]
    );
    assert_eq!(verdict_of(&gym, weeks[1]).signals, []);
}

#[test]
fn micro_plates_are_real_progress() {
    let gym = TestGym::new();
    let press = lift(&gym, "Overhead Press");
    let before = log(&gym, press, sets(kg(100, 0), &[5, 5, 5]));
    let after = log(&gym, press, sets(kg(100, 25), &[5, 5, 5]));
    assert_eq!(
        verdicts_of(&gym, &[before, after]),
        [InsufficientData, Progressed]
    );
    assert!(
        verdict_of(&gym, after)
            .signals
            .contains(&Signal::LoadAtReps {
                direction: Direction::Up,
                set_index: 0,
                reps: reps(5),
                before: kg(100, 0),
                after: kg(100, 25),
            })
    );
}

#[test]
fn the_same_work_at_an_easier_effort_is_progress_on_either_scale() {
    let gym = TestGym::new();
    let bench = lift(&gym, "Bench Press");
    let at_effort = |effort: Effort| vec![working(kg(100, 0), 8).effort(effort)];
    let sessions = [
        log(&gym, bench, at_effort(rpe(18))),
        log(&gym, bench, at_effort(rpe(16))),
        /* RIR 2 is RPE 8: the same effort on the other scale. */
        log(&gym, bench, at_effort(rir(2))),
        log(&gym, bench, at_effort(rpe(19))),
        /* RIR 3 is RPE 7: easier than RPE 9.5. */
        log(&gym, bench, at_effort(rir(3))),
    ];
    assert_eq!(
        verdicts_of(&gym, &sessions),
        [InsufficientData, Progressed, Matched, Regressed, Progressed]
    );
    assert!(has_signal(&verdict_of(&gym, sessions[1]), |s| matches!(
        s,
        Signal::Effort {
            direction: Direction::Up,
            ..
        }
    )));
    assert!(has_signal(&verdict_of(&gym, sessions[3]), |s| matches!(
        s,
        Signal::Effort {
            direction: Direction::Down,
            ..
        }
    )));
}

#[test]
fn adding_a_set_progresses_and_dropping_one_regresses() {
    let gym = TestGym::new();
    let row = lift(&gym, "Barbell Row");
    let sessions = [
        log(&gym, row, sets(kg(80, 0), &[8, 8, 8])),
        log(&gym, row, sets(kg(80, 0), &[8, 8, 8, 8])),
        log(&gym, row, sets(kg(80, 0), &[8, 8, 8])),
        log(&gym, row, sets(kg(80, 0), &[8, 8, 8])),
    ];
    assert_eq!(
        verdicts_of(&gym, &sessions),
        [InsufficientData, Progressed, Regressed, Matched]
    );
    assert!(has_signal(&verdict_of(&gym, sessions[1]), |s| matches!(
        s,
        Signal::WorkingSets {
            direction: Direction::Up,
            ..
        }
    )));
    assert!(has_signal(&verdict_of(&gym, sessions[2]), |s| matches!(
        s,
        Signal::WorkingSets {
            direction: Direction::Down,
            ..
        }
    )));
}

#[test]
fn a_heavier_single_progresses_even_though_volume_falls() {
    let gym = TestGym::new();
    let deadlift = lift(&gym, "Deadlift");
    log(&gym, deadlift, sets(kg(100, 0), &[5, 5, 5]));
    /* 100 × 5 is an e1RM of 116.67 kg; a single at 120 kg beats it. */
    let single = verdict_of(&gym, log(&gym, deadlift, sets(kg(120, 0), &[1])));
    assert_eq!(single.verdict, Progressed);
    assert!(has_signal(&single, |s| matches!(
        s,
        Signal::E1rm {
            direction: Direction::Up,
            ..
        }
    )));
    assert!(has_signal(&single, |s| matches!(
        s,
        Signal::VolumeLoad {
            direction: Direction::Down,
            ..
        }
    )));
}

#[test]
fn a_heavier_single_below_the_old_e1rm_regresses() {
    let gym = TestGym::new();
    let deadlift = lift(&gym, "Deadlift");
    log(&gym, deadlift, sets(kg(100, 0), &[5, 5, 5]));
    /* 115 kg × 1 is heavier than 100 kg, but below the 116.67 kg e1RM. */
    let single = verdict_of(&gym, log(&gym, deadlift, sets(kg(115, 0), &[1])));
    assert_eq!(single.verdict, Regressed);
}

#[test]
fn switching_units_never_fakes_progress_or_regression() {
    let gym = TestGym::new();
    let squat = lift(&gym, "Squat");
    let sessions = [
        log(&gym, squat, sets(kg(100, 0), &[5])),
        /* 220.46 lb is 100 kg to within 1 g. */
        log(&gym, squat, sets(lb(220, 46), &[5])),
        log(&gym, squat, sets(kg(100, 0), &[5])),
        /* 225 lb is 102.06 kg: a real increase. */
        log(&gym, squat, sets(lb(225, 0), &[5])),
        /* 102.5 kg is within 0.5% of 225 lb, so it counts as the same load. */
        log(&gym, squat, sets(kg(102, 50), &[5])),
    ];
    assert_eq!(
        verdicts_of(&gym, &sessions),
        [InsufficientData, Matched, Matched, Progressed, Matched]
    );
}

#[test]
fn several_exercises_in_one_session_are_judged_independently() {
    let gym = TestGym::new();
    let (bench, squat, row, curl) = (
        lift(&gym, "Bench Press"),
        lift(&gym, "Squat"),
        lift(&gym, "Barbell Row"),
        lift(&gym, "Curl"),
    );
    gym.log_session(
        &SessionPlan::new()
            .exercise(bench, sets(kg(100, 0), &[5]))
            .exercise(squat, sets(kg(140, 0), &[5]))
            .exercise(row, sets(kg(80, 0), &[8])),
    );
    let today = gym.log_session(
        &SessionPlan::new()
            .exercise(bench, sets(kg(102, 50), &[5]))
            .exercise(squat, sets(kg(140, 0), &[5]))
            .exercise(row, sets(kg(80, 0), &[7]))
            .exercise(curl, sets(kg(20, 0), &[10])),
    );
    let verdicts = gym.os().session_verdicts(today.session).unwrap();
    let each: Vec<Verdict> = verdicts
        .entries
        .iter()
        .map(|e| match &e.outcome {
            EntryOutcome::Verdict(v) => v.verdict,
            EntryOutcome::Unsupported(mode) => panic!("unexpected unsupported {mode:?}"),
        })
        .collect();
    assert_eq!(each, [Progressed, Matched, Regressed, InsufficientData]);
    let tally = verdicts.tally;
    assert_eq!(
        (
            tally.progressed,
            tally.matched,
            tally.regressed,
            tally.insufficient,
            tally.deload
        ),
        (1, 1, 1, 1, 0)
    );
}

/* ---- deloads in a block ------------------------------------------------- */

#[test]
fn a_deload_week_is_skipped_and_the_block_resumes_from_before_it() {
    let gym = TestGym::new();
    let squat = lift(&gym, "Squat");
    let w1 = log(&gym, squat, sets(kg(100, 0), &[5, 5, 5]));
    let w2 = log(&gym, squat, sets(kg(102, 50), &[5, 5, 5]));
    let w3 = log(&gym, squat, sets(kg(105, 0), &[5, 5, 5]));
    let w4 = gym
        .log_session(
            &SessionPlan::new()
                .deload()
                .exercise(squat, sets(kg(80, 0), &[5, 5, 5])),
        )
        .session;
    let w5 = log(&gym, squat, sets(kg(105, 0), &[5, 5, 5]));
    let w6 = log(&gym, squat, sets(kg(107, 50), &[5, 5, 5]));

    assert_eq!(
        verdicts_of(&gym, &[w1, w2, w3, w4, w5, w6]),
        [
            InsufficientData,
            Progressed,
            Progressed,
            Deload,
            Matched,
            Progressed
        ]
    );
    assert_eq!(
        baseline_of(&gym, w4),
        Some(w3),
        "a deload is still compared, for its signals"
    );
    assert_eq!(baseline_of(&gym, w5), Some(w3));
    assert_eq!(baseline_of(&gym, w6), Some(w5));
}

#[test]
fn back_to_back_deloads_reach_back_to_the_last_real_session() {
    let gym = TestGym::new();
    let squat = lift(&gym, "Squat");
    let real = log(&gym, squat, sets(kg(100, 0), &[5, 5, 5]));
    for load in [kg(80, 0), kg(70, 0)] {
        gym.log_session(
            &SessionPlan::new()
                .deload()
                .exercise(squat, sets(load, &[5, 5, 5])),
        );
    }
    let back = log(&gym, squat, sets(kg(100, 0), &[5, 5, 5]));
    assert_eq!(baseline_of(&gym, back), Some(real));
    assert_eq!(verdict_of(&gym, back).verdict, Matched);
}

#[test]
fn a_first_ever_session_marked_deload_leaves_no_baseline() {
    let gym = TestGym::new();
    let squat = lift(&gym, "Squat");
    let deload = gym
        .log_session(
            &SessionPlan::new()
                .deload()
                .exercise(squat, sets(kg(60, 0), &[5])),
        )
        .session;
    let first_real = log(&gym, squat, sets(kg(100, 0), &[5]));
    let second = log(&gym, squat, sets(kg(100, 0), &[6]));
    assert_eq!(
        verdicts_of(&gym, &[deload, first_real, second]),
        [Deload, InsufficientData, Progressed]
    );
    assert_eq!(baseline_of(&gym, first_real), None);
}

#[test]
fn marking_todays_session_as_a_deload_changes_its_verdict_only() {
    let gym = TestGym::new();
    let squat = lift(&gym, "Squat");
    log(&gym, squat, sets(kg(100, 0), &[5]));
    let today = log(&gym, squat, sets(kg(90, 0), &[5]));
    assert_eq!(verdict_of(&gym, today).verdict, Regressed);
    gym.set_deload(today, true);
    let verdicts = gym.os().session_verdicts(today).unwrap();
    assert_eq!(verdict_of(&gym, today).verdict, Deload);
    assert_eq!((verdicts.tally.deload, verdicts.tally.regressed), (1, 0));
}

/* ---- templates over weeks ----------------------------------------------- */

#[test]
fn rotating_templates_each_progress_against_their_own_history() {
    let gym = TestGym::new();
    let bench = lift(&gym, "Bench Press");
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
        verdicts_of(&gym, &[a1, b1, a2, b2, a3, b3]),
        [
            InsufficientData,
            Regressed,
            Progressed,
            Progressed,
            Progressed,
            Progressed
        ]
    );
    let rule = |s| {
        gym.os().session_verdicts(s).unwrap().entries[0]
            .baseline
            .as_ref()
            .map(|b| (b.session, b.rule))
    };
    assert_eq!(rule(b1), Some((a1, BaselineRule::SameExercise)));
    assert_eq!(rule(a2), Some((a1, BaselineRule::SameTemplate(heavy))));
    assert_eq!(rule(b2), Some((b1, BaselineRule::SameTemplate(volume))));
    assert_eq!(rule(b3), Some((b2, BaselineRule::SameTemplate(volume))));
}

/* ---- the past changes, and verdicts follow ------------------------------ */

#[test]
fn importing_an_older_session_moves_the_baseline_of_later_ones() {
    let gym = TestGym::new();
    let squat = lift(&gym, "Squat");
    let jan_1 = log_at(&gym, at(1_767_225_600_000), squat, &sets(kg(100, 0), &[5]));
    let jan_15 = log_at(
        &gym,
        at(1_767_225_600_000 + 14 * DAY),
        squat,
        &sets(kg(105, 0), &[5]),
    );
    assert_eq!(
        (baseline_of(&gym, jan_15), verdict_of(&gym, jan_15).verdict),
        (Some(jan_1), Progressed)
    );

    /* A forgotten workout from Jan 8 is imported afterwards. */
    let jan_8 = log_at(
        &gym,
        at(1_767_225_600_000 + 7 * DAY),
        squat,
        &sets(kg(107, 50), &[5]),
    );
    assert_eq!(
        (baseline_of(&gym, jan_8), verdict_of(&gym, jan_8).verdict),
        (Some(jan_1), Progressed)
    );
    assert_eq!(
        (baseline_of(&gym, jan_15), verdict_of(&gym, jan_15).verdict),
        (Some(jan_8), Regressed)
    );
}

#[test]
fn logging_a_forgotten_set_into_a_past_session_rejudges_the_next_one() {
    let gym = TestGym::new();
    let row = lift(&gym, "Barbell Row");
    let past = gym.log_session(&SessionPlan::new().exercise(row, sets(kg(80, 0), &[8, 8])));
    let today = log(&gym, row, sets(kg(80, 0), &[8, 8, 8]));
    assert_eq!(verdict_of(&gym, today).verdict, Progressed);

    gym.log_set(past.session, past.entries[0], &working(kg(80, 0), 8));
    assert_eq!(verdict_of(&gym, today).verdict, Matched);
}

#[test]
fn finishing_an_older_session_later_makes_it_the_baseline() {
    let gym = TestGym::new();
    let squat = lift(&gym, "Squat");
    let first = log(&gym, squat, sets(kg(100, 0), &[5]));
    let left_open = gym
        .log_session(
            &SessionPlan::new()
                .unfinished()
                .exercise(squat, sets(kg(105, 0), &[5])),
        )
        .session;
    let today = log(&gym, squat, sets(kg(105, 0), &[5]));
    assert_eq!(
        (baseline_of(&gym, today), verdict_of(&gym, today).verdict),
        (Some(first), Progressed)
    );

    gym.finish(left_open);
    assert_eq!(
        (baseline_of(&gym, today), verdict_of(&gym, today).verdict),
        (Some(left_open), Matched)
    );
}

#[test]
fn a_replayed_command_never_changes_a_verdict() {
    let gym = TestGym::new();
    let squat = lift(&gym, "Squat");
    log(&gym, squat, sets(kg(100, 0), &[5]));
    let today = gym.log_session(&SessionPlan::new().exercise(squat, sets(kg(100, 0), &[5])));
    let before = gym.os().session_verdicts(today.session).unwrap();

    let extra = Command::new(
        gym.command_id(),
        working(kg(100, 0), 5).log_into(today.session, today.entries[0]),
    );
    gym.os().execute(extra.clone()).unwrap();
    let after_first = gym.os().session_verdicts(today.session).unwrap();
    assert_ne!(after_first, before, "the extra set is a new working set");
    assert_eq!(
        gym.os().execute(extra).unwrap(),
        CommandOutcome::AlreadyApplied
    );
    assert_eq!(
        gym.os().session_verdicts(today.session).unwrap(),
        after_first
    );
}

#[test]
fn verdicts_are_deterministic_and_carry_the_rules_version() {
    let gym = TestGym::new();
    let squat = lift(&gym, "Squat");
    log(&gym, squat, sets(kg(100, 0), &[5, 5, 5]));
    let today = log(&gym, squat, sets(kg(100, 0), &[6, 5, 4]));
    let first = gym.os().session_verdicts(today).unwrap();
    assert_eq!(gym.os().session_verdicts(today).unwrap(), first);
    assert_eq!(verdict_of(&gym, today).algorithm_version, ALGORITHM_VERSION);
}
