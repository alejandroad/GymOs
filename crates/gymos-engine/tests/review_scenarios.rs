/*!
 * Review scenarios: verdicts driven end to end through `GymOs<TestPorts>`.
 *
 * Every expected value is computed by hand from the spec (DD-13, DD-15 and
 * the golden table in `docs/research/progressive-overload.md`), never taken
 * from the implementation's output. Epley: `e1RM = w × (1 + r/30)`, RPE
 * adjusted through RIR = 10 − RPE. Volume is Σ load × reps over counted
 * (non-warm-up) sets.
 */

#![expect(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "scenario fixtures from known-valid literals; a panic is a failed test"
)]

use gymos_domain::progression::{
    Direction, E1rm, ExerciseVerdict, ProgressionScheme, RepRange, Signal, Skipped, Verdict,
};
use gymos_domain::{
    ALGORITHM_VERSION, Distance, DurationSecs, Effort, Exercise, ExerciseId, Load,
    SessionExerciseId, SessionId, SetMetrics, SetType, Source, TemplateId, Timestamp, TrackingMode,
    UtcOffset, Uuid,
};
use gymos_engine::{
    BaselineRule, CommandOutcome, EntryOutcome, FinishSession, SessionVerdicts, StartSession,
    Store, WriteTx,
};
use gymos_testkit::{LoggedSession, SessionPlan, SetBuilder, TestGym, kg, lb, reps, rpe, working};

use Direction::{Down, Up};

/* ---- helpers ------------------------------------------------------------ */

fn bench(gym: &TestGym) -> ExerciseId {
    gym.create_exercise("Bench Press", TrackingMode::WeightReps, None)
}

fn lift(gym: &TestGym, label: &str) -> ExerciseId {
    gym.create_exercise(label, TrackingMode::WeightReps, None)
}

fn template(n: u128) -> TemplateId {
    TemplateId::from_uuid(Uuid::from_u128(0x7E << 64 | n))
}

fn ws(load: Load, reps: &[u16]) -> Vec<SetBuilder> {
    reps.iter().map(|&n| working(load, n)).collect()
}

fn typed(set_type: SetType, load: Load, n: u16) -> SetBuilder {
    working(load, n).set_type(set_type)
}

fn single(gym: &TestGym, exercise: ExerciseId, sets: Vec<SetBuilder>) -> LoggedSession {
    gym.log_session(&SessionPlan::new().exercise(exercise, sets))
}

fn verdicts(gym: &TestGym, session: SessionId) -> SessionVerdicts {
    gym.os().session_verdicts(session).unwrap()
}

/** The verdict of entry `i`, which must have one. */
fn verdict_at(v: &SessionVerdicts, i: usize) -> &ExerciseVerdict {
    match &v.entries[i].outcome {
        EntryOutcome::Verdict(verdict) => verdict,
        EntryOutcome::Unsupported(mode) => panic!("entry {i}: unexpected unsupported {mode:?}"),
    }
}

/** The verdict of a single-entry session, computed through the engine. */
fn only(gym: &TestGym, session: SessionId) -> ExerciseVerdict {
    let v = verdicts(gym, session);
    assert_eq!(v.entries.len(), 1);
    verdict_at(&v, 0).clone()
}

fn baseline_of(gym: &TestGym, session: SessionId, entry: SessionExerciseId) -> Option<SessionId> {
    gym.os()
        .last_time(session, entry)
        .unwrap()
        .map(|b| b.session)
}

fn e1rm(load: Load, n: u16, effort: Option<Effort>) -> E1rm {
    E1rm::of_set(load, reps(n), effort).unwrap()
}

/** Checks an e1RM against a hand-computed value, to two decimals. */
fn assert_kg(e: E1rm, kg: f64) {
    let rounded = (e.kilograms() * 100.0).round() / 100.0;
    assert!(
        (rounded - kg).abs() < 1e-9,
        "{} kg is not {kg}",
        e.kilograms()
    );
}

/** `kg_reps(1232, 50)` is 1232.50 kg·reps in nanogram-reps. */
const fn kg_reps(whole: i128, hundredths: i128) -> i128 {
    (whole * 100 + hundredths) * 10_000_000_000
}

/**
 * Compares signals in order. A `None` slot is the volume signal, checked
 * against `volume` as (direction, before, after) in nanogram-reps.
 */
fn assert_signals(
    actual: &[Signal],
    expected: &[Option<Signal>],
    volume: Option<(Direction, i128, i128)>,
) {
    assert_eq!(actual.len(), expected.len(), "signals: {actual:#?}");
    for (got, want) in actual.iter().zip(expected) {
        if let Some(want) = want {
            assert_eq!(got, want);
        } else {
            let Signal::VolumeLoad {
                direction,
                before,
                after,
            } = *got
            else {
                panic!("expected VolumeLoad, got {got:?}");
            };
            assert_eq!(
                Some((direction, before.nanogram_reps(), after.nanogram_reps())),
                volume
            );
        }
    }
}

const VOLUME: Option<Signal> = None;

fn assert_outcome(v: &ExerciseVerdict, verdict: Verdict, skipped: &[Skipped]) {
    assert_eq!(
        (v.verdict, v.skipped.as_slice(), v.algorithm_version),
        (verdict, skipped, ALGORITHM_VERSION)
    );
}

fn reps_at_load(
    direction: Direction,
    set_index: usize,
    load: Load,
    before: u16,
    after: u16,
) -> Signal {
    Signal::RepsAtLoad {
        direction,
        set_index,
        load,
        before: reps(before),
        after: reps(after),
    }
}

fn load_at_reps(
    direction: Direction,
    set_index: usize,
    n: u16,
    before: Load,
    after: Load,
) -> Signal {
    Signal::LoadAtReps {
        direction,
        set_index,
        reps: reps(n),
        before,
        after,
    }
}

/** One set in `mode`, whatever its measurements. */
fn sample(mode: TrackingMode) -> SetMetrics {
    let r = reps(10);
    let secs = DurationSecs::new(45).unwrap();
    match mode {
        TrackingMode::WeightReps => SetMetrics::WeightReps {
            load: kg(50, 0),
            reps: r,
        },
        TrackingMode::BodyweightReps => SetMetrics::BodyweightReps { reps: r },
        TrackingMode::WeightedBodyweight => SetMetrics::WeightedBodyweight {
            added: kg(10, 0),
            reps: r,
        },
        TrackingMode::AssistedBodyweight => SetMetrics::AssistedBodyweight {
            assistance: kg(20, 0),
            reps: r,
        },
        TrackingMode::Duration => SetMetrics::Duration {
            duration: secs,
            load: None,
        },
        TrackingMode::DistanceLoad => SetMetrics::DistanceLoad {
            distance: Distance::from_millimetres(20_000).unwrap(),
            load: kg(40, 0),
            duration: None,
        },
        TrackingMode::RepsOnly => SetMetrics::RepsOnly { reps: r },
        TrackingMode::TimeReps => SetMetrics::TimeReps {
            reps: r,
            duration: secs,
        },
    }
}

/**
 * Logs a finished one-exercise session at an explicit start (UTC millis
 * after `TestGym::START`, in the given offset), ending one minute later.
 * Never reads the stepping clock, so the order is fully controlled.
 */
fn logged_at(
    gym: &TestGym,
    hours_after_start: i64,
    offset_minutes: i16,
    exercise: ExerciseId,
    sets: &[SetBuilder],
) -> (SessionId, SessionExerciseId) {
    let offset = UtcOffset::from_minutes(offset_minutes).unwrap();
    let millis = TestGym::START.unix_millis() + hours_after_start * 3_600_000;
    let outcome = gym
        .run(StartSession {
            started_at: Some(Timestamp::new(millis, offset)),
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
        ended_at: Some(Timestamp::new(millis + 60_000, offset)),
    })
    .unwrap();
    (session, entry)
}

/* ---- every verdict kind, golden examples through the engine ------------ */

#[test]
fn golden_c_a_lower_rpe_at_the_same_work_progresses() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let k = kg(100, 0);
    single(&gym, bench, vec![working(k, 8).effort(rpe(18))]);
    let today = single(&gym, bench, vec![working(k, 8).effort(rpe(16))]);

    /* RPE 9 → 1 RIR → 9 effective reps: 100 × 39/30 = 130.00.
     * RPE 8 → 2 RIR → 10 effective reps: 100 × 40/30 = 133.33.
     * Same load × reps, so volume is unchanged. */
    let v = only(&gym, today.session);
    assert_outcome(&v, Verdict::Progressed, &[]);
    let before = e1rm(k, 8, Some(rpe(18)));
    let after = e1rm(k, 8, Some(rpe(16)));
    assert_kg(before, 130.00);
    assert_kg(after, 133.33);
    assert_signals(
        &v.signals,
        &[
            Some(Signal::E1rm {
                direction: Up,
                before,
                after,
            }),
            Some(Signal::Effort {
                direction: Up,
                set_index: 0,
                before: rpe(18),
                after: rpe(16),
            }),
        ],
        None,
    );
}

#[test]
fn golden_d_the_same_session_matches() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    single(&gym, bench, ws(kg(100, 0), &[8, 8, 8]));
    let today = single(&gym, bench, ws(kg(100, 0), &[8, 8, 8]));
    let v = verdicts(&gym, today.session);
    assert_outcome(verdict_at(&v, 0), Verdict::Matched, &[]);
    assert_eq!(verdict_at(&v, 0).signals, []);
    assert_eq!((v.tally.matched, v.tally.total()), (1, 1));
}

#[test]
fn golden_e_fewer_reps_regresses() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let k = kg(100, 0);
    single(&gym, bench, ws(k, &[8, 8, 8]));
    let today = single(&gym, bench, ws(k, &[8, 7, 6]));
    /* Best set is 100 × 8 on both sides: no e1RM signal.
     * Volume 2400 → 800 + 700 + 600 = 2100. */
    let v = only(&gym, today.session);
    assert_outcome(&v, Verdict::Regressed, &[]);
    assert_signals(
        &v.signals,
        &[
            Some(reps_at_load(Down, 1, k, 8, 7)),
            Some(reps_at_load(Down, 2, k, 8, 6)),
            VOLUME,
        ],
        Some((Down, kg_reps(2400, 0), kg_reps(2100, 0))),
    );
    assert!(!v.is_progressive_overload());
}

#[test]
fn golden_g_a_deload_session_is_deload_and_still_explains_itself() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let normal = single(&gym, bench, ws(kg(100, 0), &[8, 8, 8]));
    let deload = gym.log_session(
        &SessionPlan::new()
            .deload()
            .exercise(bench, ws(kg(80, 0), &[8, 8, 8])),
    );
    /* A deload still resolves its own baseline, so it can explain itself. */
    assert_eq!(
        baseline_of(&gym, deload.session, deload.entries[0]),
        Some(normal.session)
    );
    /* 100 × 38/30 = 126.67 → 80 × 38/30 = 101.33; volume 2400 → 1920. */
    let v = verdicts(&gym, deload.session);
    let only = verdict_at(&v, 0);
    assert_outcome(only, Verdict::Deload, &[]);
    let (before, after) = (e1rm(kg(100, 0), 8, None), e1rm(kg(80, 0), 8, None));
    assert_kg(before, 126.67);
    assert_kg(after, 101.33);
    assert_signals(
        &only.signals,
        &[
            Some(Signal::E1rm {
                direction: Down,
                before,
                after,
            }),
            Some(load_at_reps(Down, 0, 8, kg(100, 0), kg(80, 0))),
            Some(load_at_reps(Down, 1, 8, kg(100, 0), kg(80, 0))),
            Some(load_at_reps(Down, 2, 8, kg(100, 0), kg(80, 0))),
            VOLUME,
        ],
        Some((Down, kg_reps(2400, 0), kg_reps(1920, 0))),
    );
    assert_eq!((v.tally.deload, v.tally.total()), (1, 1));
}

#[test]
fn golden_h_a_mixed_session_progresses_and_lists_the_drop() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let k = kg(100, 0);
    single(&gym, bench, ws(k, &[8, 8, 8]));
    let today = single(&gym, bench, ws(k, &[9, 8, 6]));
    /* e1RM 126.67 → 100 × 39/30 = 130.00; volume 2400 → 2300 (always listed
     * when it falls). */
    let v = only(&gym, today.session);
    assert_outcome(&v, Verdict::Progressed, &[]);
    assert_signals(
        &v.signals,
        &[
            Some(Signal::E1rm {
                direction: Up,
                before: e1rm(k, 8, None),
                after: e1rm(k, 9, None),
            }),
            Some(reps_at_load(Up, 0, k, 8, 9)),
            Some(reps_at_load(Down, 2, k, 8, 6)),
            VOLUME,
        ],
        Some((Down, kg_reps(2400, 0), kg_reps(2300, 0))),
    );
}

#[test]
fn golden_i_an_extra_identical_set_progresses() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let k = kg(100, 0);
    single(&gym, bench, ws(k, &[8, 8, 8]));
    let today = single(&gym, bench, ws(k, &[8, 8, 8, 8]));
    let v = only(&gym, today.session);
    assert_outcome(&v, Verdict::Progressed, &[]);
    assert_signals(
        &v.signals,
        &[
            Some(Signal::WorkingSets {
                direction: Up,
                before: 3,
                after: 4,
            }),
            VOLUME,
        ],
        Some((Up, kg_reps(2400, 0), kg_reps(3200, 0))),
    );
}

#[test]
fn golden_j_switching_from_kg_to_lb_at_the_same_load_matches() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    single(&gym, bench, ws(kg(100, 0), &[8]));
    /* 220.46 lb is 100 kg within the 0.5% cross-unit tolerance. */
    let today = single(&gym, bench, ws(lb(220, 46), &[8]));
    let v = only(&gym, today.session);
    assert_outcome(&v, Verdict::Matched, &[]);
    assert_eq!(v.signals, []);
}

#[test]
fn golden_k_high_rep_sets_skip_e1rm_but_still_progress() {
    let gym = TestGym::new();
    let curl = lift(&gym, "Cable Row");
    let k = kg(60, 0);
    single(&gym, curl, ws(k, &[15, 15]));
    let today = single(&gym, curl, ws(k, &[16, 15]));
    /* Volume 60 × 30 = 1800 → 60 × 31 = 1860. */
    let v = only(&gym, today.session);
    assert_outcome(&v, Verdict::Progressed, &[Skipped::E1rmAboveRepCap]);
    assert_signals(
        &v.signals,
        &[Some(reps_at_load(Up, 0, k, 15, 16)), VOLUME],
        Some((Up, kg_reps(1800, 0), kg_reps(1860, 0))),
    );
}

#[test]
fn golden_l_top_sets_pair_with_top_sets_and_backoffs_only_add_volume() {
    let gym = TestGym::new();
    let squat = lift(&gym, "Squat");
    single(
        &gym,
        squat,
        vec![
            typed(SetType::TopSet, kg(100, 0), 5),
            typed(SetType::Backoff, kg(90, 0), 8),
        ],
    );
    let today = single(
        &gym,
        squat,
        vec![
            typed(SetType::TopSet, kg(102, 50), 5),
            typed(SetType::Backoff, kg(90, 0), 8),
        ],
    );
    /* 100 × 35/30 = 116.67 → 102.5 × 35/30 = 119.58.
     * Volume 500 + 720 = 1220 → 512.5 + 720 = 1232.5. */
    let v = only(&gym, today.session);
    assert_outcome(&v, Verdict::Progressed, &[]);
    let (before, after) = (e1rm(kg(100, 0), 5, None), e1rm(kg(102, 50), 5, None));
    assert_kg(before, 116.67);
    assert_kg(after, 119.58);
    assert_signals(
        &v.signals,
        &[
            Some(Signal::E1rm {
                direction: Up,
                before,
                after,
            }),
            Some(load_at_reps(Up, 0, 5, kg(100, 0), kg(102, 50))),
            VOLUME,
        ],
        Some((Up, kg_reps(1220, 0), kg_reps(1232, 50))),
    );
}

#[test]
fn one_session_reaches_four_verdict_kinds_then_all_become_deload() {
    let gym = TestGym::new();
    let (bench, squat, row, curl) = (
        bench(&gym),
        lift(&gym, "Squat"),
        lift(&gym, "Row"),
        lift(&gym, "Curl"),
    );
    gym.log_session(
        &SessionPlan::new()
            .exercise(bench, ws(kg(100, 0), &[8]))
            .exercise(squat, ws(kg(140, 0), &[5]))
            .exercise(row, ws(kg(80, 0), &[10])),
    );
    let today = gym.log_session(
        &SessionPlan::new()
            .exercise(bench, ws(kg(100, 0), &[9]))
            .exercise(squat, ws(kg(140, 0), &[5]))
            .exercise(row, ws(kg(80, 0), &[9]))
            .exercise(curl, ws(kg(15, 0), &[12])),
    );
    let v = verdicts(&gym, today.session);
    let kinds: Vec<Verdict> = (0..4).map(|i| verdict_at(&v, i).verdict).collect();
    assert_eq!(
        kinds,
        [
            Verdict::Progressed,
            Verdict::Matched,
            Verdict::Regressed,
            Verdict::InsufficientData
        ]
    );
    assert_eq!(v.entries[3].baseline, None);
    assert_eq!(
        (
            v.tally.progressed,
            v.tally.matched,
            v.tally.regressed,
            v.tally.insufficient,
            v.tally.deload
        ),
        (1, 1, 1, 1, 0)
    );

    /* Marking the session a deload turns every entry, even a first one,
     * into Deload (rule 2 comes before rule 3). */
    gym.set_deload(today.session, true);
    let v = verdicts(&gym, today.session);
    assert!((0..4).all(|i| verdict_at(&v, i).verdict == Verdict::Deload));
    assert_eq!((v.tally.deload, v.tally.total()), (4, 4));
}

/* ---- set types ---------------------------------------------------------- */

#[test]
fn a_warmup_only_session_is_insufficient_data_on_either_side() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let s1 = single(&gym, bench, ws(kg(100, 0), &[8]));
    let warm = single(&gym, bench, vec![typed(SetType::Warmup, kg(60, 0), 10)]);
    /* It has a baseline, but no counted sets of its own. */
    assert_eq!(
        baseline_of(&gym, warm.session, warm.entries[0]),
        Some(s1.session)
    );
    assert_eq!(only(&gym, warm.session).verdict, Verdict::InsufficientData);

    /* DD-15 rule 5: a warm-up-only entry is never a baseline, so the next
     * real session skips it and compares with S1. */
    let next = single(&gym, bench, ws(kg(100, 0), &[8]));
    assert_eq!(
        baseline_of(&gym, next.session, next.entries[0]),
        Some(s1.session)
    );
    assert_eq!(only(&gym, next.session).verdict, Verdict::Matched);
}

#[test]
fn warmups_in_the_baseline_change_nothing() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    single(
        &gym,
        bench,
        vec![
            typed(SetType::Warmup, kg(60, 0), 10),
            typed(SetType::Warmup, kg(80, 0), 5),
            working(kg(100, 0), 8),
        ],
    );
    let today = single(&gym, bench, ws(kg(100, 0), &[8]));
    let v = only(&gym, today.session);
    assert_outcome(&v, Verdict::Matched, &[]);
    assert_eq!(v.signals, []);
}

#[test]
fn drop_sets_add_volume_but_are_never_paired() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let k = kg(100, 0);
    single(
        &gym,
        bench,
        vec![working(k, 8), typed(SetType::Drop, kg(70, 0), 10)],
    );
    let today = single(
        &gym,
        bench,
        vec![working(k, 8), typed(SetType::Drop, kg(70, 0), 12)],
    );
    /* No RepsAtLoad for the drop set. Volume 800 + 700 = 1500 → 800 + 840
     * = 1640, and the best e1RM (100 × 8) did not drop. */
    let v = only(&gym, today.session);
    assert_outcome(&v, Verdict::Progressed, &[]);
    assert_signals(
        &v.signals,
        &[VOLUME],
        Some((Up, kg_reps(1500, 0), kg_reps(1640, 0))),
    );
}

#[test]
fn backoff_sets_that_lose_reps_only_lower_volume() {
    let gym = TestGym::new();
    let squat = lift(&gym, "Squat");
    single(
        &gym,
        squat,
        vec![
            typed(SetType::TopSet, kg(100, 0), 5),
            typed(SetType::Backoff, kg(90, 0), 8),
        ],
    );
    let today = single(
        &gym,
        squat,
        vec![
            typed(SetType::TopSet, kg(100, 0), 5),
            typed(SetType::Backoff, kg(90, 0), 6),
        ],
    );
    /* Top sets identical; backoffs are unpaired. Volume 1220 → 500 + 540. */
    let v = only(&gym, today.session);
    assert_outcome(&v, Verdict::Regressed, &[]);
    assert_signals(
        &v.signals,
        &[VOLUME],
        Some((Down, kg_reps(1220, 0), kg_reps(1040, 0))),
    );
}

#[test]
fn failure_sets_pair_by_order_like_working_sets() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let k = kg(100, 0);
    single(
        &gym,
        bench,
        vec![working(k, 8), typed(SetType::Failure, k, 6)],
    );
    let today = single(
        &gym,
        bench,
        vec![working(k, 8), typed(SetType::Failure, k, 7)],
    );
    /* Best e1RM (100 × 8) unchanged; volume 1400 → 1500. */
    let v = only(&gym, today.session);
    assert_outcome(&v, Verdict::Progressed, &[]);
    assert_signals(
        &v.signals,
        &[Some(reps_at_load(Up, 1, k, 6, 7)), VOLUME],
        Some((Up, kg_reps(1400, 0), kg_reps(1500, 0))),
    );
}

#[test]
fn a_top_set_pairs_with_the_top_set_when_working_sets_are_added_before_it() {
    let gym = TestGym::new();
    let squat = lift(&gym, "Squat");
    single(
        &gym,
        squat,
        vec![working(kg(80, 0), 8), typed(SetType::TopSet, kg(100, 0), 5)],
    );
    let today = single(
        &gym,
        squat,
        vec![
            working(kg(80, 0), 8),
            working(kg(80, 0), 8),
            typed(SetType::TopSet, kg(100, 0), 6),
        ],
    );
    /* e1RM 100 × 35/30 = 116.67 → 100 × 36/30 = 120.00.
     * Volume 640 + 500 = 1140 → 1280 + 600 = 1880. */
    let v = only(&gym, today.session);
    assert_outcome(&v, Verdict::Progressed, &[]);
    assert!(
        v.signals.iter().any(|s| matches!(
            s,
            Signal::RepsAtLoad { direction: Up, load, before, after, .. }
                if *load == kg(100, 0) && *before == reps(5) && *after == reps(6)
        )),
        "the top set must pair with the top set: {:#?}",
        v.signals
    );
    assert!(
        !v.signals
            .iter()
            .any(|s| matches!(s, Signal::WorkingSets { .. })),
        "a changed pair rules out WorkingSets"
    );
    assert!(v.signals.contains(&Signal::E1rm {
        direction: Up,
        before: e1rm(kg(100, 0), 5, None),
        after: e1rm(kg(100, 0), 6, None),
    }));
}

/* ---- tracking modes ----------------------------------------------------- */

#[test]
fn every_non_weight_reps_mode_is_unsupported_without_failing_the_session() {
    let gym = TestGym::new();
    let modes = [
        TrackingMode::WeightReps,
        TrackingMode::BodyweightReps,
        TrackingMode::WeightedBodyweight,
        TrackingMode::AssistedBodyweight,
        TrackingMode::Duration,
        TrackingMode::DistanceLoad,
        TrackingMode::RepsOnly,
        TrackingMode::TimeReps,
    ];
    let exercises: Vec<ExerciseId> = modes
        .iter()
        .enumerate()
        .map(|(i, &mode)| gym.create_exercise(&format!("Lift {i}"), mode, None))
        .collect();
    let plan = |bench_reps: u16| {
        modes
            .iter()
            .zip(&exercises)
            .fold(SessionPlan::new(), |plan, (&mode, &id)| {
                let set = if mode == TrackingMode::WeightReps {
                    working(kg(50, 0), bench_reps)
                } else {
                    SetBuilder::new(sample(mode))
                };
                plan.exercise(id, [set])
            })
    };

    /* First session: unsupported entries and an InsufficientData one. */
    let first = gym.log_session(&plan(10));
    let v = verdicts(&gym, first.session);
    assert_eq!(verdict_at(&v, 0).verdict, Verdict::InsufficientData);
    assert_eq!((v.tally.insufficient, v.tally.total()), (1, 1));

    let today = gym.log_session(&plan(11));
    let v = verdicts(&gym, today.session);
    assert_eq!(v.entries.len(), modes.len());
    assert_eq!(verdict_at(&v, 0).verdict, Verdict::Progressed);
    for (i, &mode) in modes.iter().enumerate().skip(1) {
        assert_eq!(v.entries[i].outcome, EntryOutcome::Unsupported(mode));
        assert_eq!(v.entries[i].entry, today.entries[i]);
        /* The baseline is still resolved for unsupported modes. */
        assert_eq!(
            v.entries[i].baseline.as_ref().map(|b| b.session),
            Some(first.session)
        );
    }
    /* Only the WeightReps entry is counted. */
    assert_eq!((v.tally.progressed, v.tally.total()), (1, 1));
}

/* ---- progression schemes ------------------------------------------------ */

fn six_to_eight() -> ProgressionScheme {
    ProgressionScheme::new(RepRange::new(reps(6), reps(8)).unwrap(), kg(2, 50)).unwrap()
}

#[test]
fn reps_below_the_scheme_minimum_at_the_new_load_are_no_load_step() {
    let gym = TestGym::new();
    let bench = gym.create_exercise(
        "Bench Press",
        TrackingMode::WeightReps,
        Some(six_to_eight()),
    );
    single(&gym, bench, ws(kg(100, 0), &[8, 8, 8]));
    let today = single(&gym, bench, ws(kg(102, 50), &[6, 6, 5]));
    /* Fewest reps at 102.5 kg is 5 < 6, so no LoadStep.
     * e1RM 126.67 → 102.5 × 36/30 = 123.00; volume 2400 → 102.5 × 17 = 1742.5. */
    let v = only(&gym, today.session);
    assert_outcome(&v, Verdict::Regressed, &[]);
    let after = e1rm(kg(102, 50), 6, None);
    assert_kg(after, 123.00);
    assert_signals(
        &v.signals,
        &[
            Some(Signal::E1rm {
                direction: Down,
                before: e1rm(kg(100, 0), 8, None),
                after,
            }),
            VOLUME,
        ],
        Some((Down, kg_reps(2400, 0), kg_reps(1742, 50))),
    );
}

#[test]
fn any_load_increase_is_a_load_step_and_only_with_a_scheme() {
    let gym = TestGym::new();
    let with = gym.create_exercise(
        "Bench Press",
        TrackingMode::WeightReps,
        Some(six_to_eight()),
    );
    let without = lift(&gym, "Incline Press");
    gym.log_session(
        &SessionPlan::new()
            .exercise(with, ws(kg(100, 0), &[8]))
            .exercise(without, ws(kg(100, 0), &[8])),
    );
    /* +1 kg is less than the 2.5 kg increment: still a step (DD-13 gap 1). */
    let today = gym.log_session(
        &SessionPlan::new()
            .exercise(with, ws(kg(101, 0), &[6]))
            .exercise(without, ws(kg(101, 0), &[6])),
    );
    /* e1RM 126.67 → 101 × 36/30 = 121.20; volume 800 → 606. */
    let v = verdicts(&gym, today.session);
    let e1rm_down = Signal::E1rm {
        direction: Down,
        before: e1rm(kg(100, 0), 8, None),
        after: e1rm(kg(101, 0), 6, None),
    };
    assert_kg(e1rm(kg(101, 0), 6, None), 121.20);
    assert_outcome(verdict_at(&v, 0), Verdict::Progressed, &[]);
    assert_signals(
        &verdict_at(&v, 0).signals,
        &[
            Some(e1rm_down),
            Some(Signal::LoadStep {
                before: kg(100, 0),
                after: kg(101, 0),
                reps: reps(6),
            }),
            VOLUME,
        ],
        Some((Down, kg_reps(800, 0), kg_reps(606, 0))),
    );
    assert_outcome(verdict_at(&v, 1), Verdict::Regressed, &[]);
    assert_signals(
        &verdict_at(&v, 1).signals,
        &[Some(e1rm_down), VOLUME],
        Some((Down, kg_reps(800, 0), kg_reps(606, 0))),
    );
}

#[test]
fn setting_a_scheme_later_changes_past_verdicts() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    single(&gym, bench, ws(kg(100, 0), &[8, 8, 8]));
    let today = single(&gym, bench, ws(kg(102, 50), &[6, 6, 6]));
    assert_eq!(only(&gym, today.session).verdict, Verdict::Regressed);

    /* No command changes a scheme yet, so seed it directly. Verdicts read
     * the current scheme (D-2, D-6). */
    let mut stored: Exercise = gym.store().snapshot().exercises()[&bench].clone();
    stored.scheme = Some(six_to_eight());
    gym.store()
        .write(|tx| tx.save_exercise(&stored).map_err(Into::into))
        .unwrap();
    assert_eq!(only(&gym, today.session).verdict, Verdict::Progressed);
}

/* ---- baselines (D-1) ---------------------------------------------------- */

#[test]
fn template_rule_skips_deloads_and_unfinished_then_falls_back() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let push_a = template(1);
    let s1 = gym.log_session(
        &SessionPlan::new()
            .template(push_a)
            .exercise(bench, ws(kg(100, 0), &[8])),
    );
    gym.log_session(
        &SessionPlan::new()
            .template(push_a)
            .deload()
            .exercise(bench, ws(kg(80, 0), &[8])),
    );
    gym.log_session(
        &SessionPlan::new()
            .template(push_a)
            .unfinished()
            .exercise(bench, ws(kg(120, 0), &[8])),
    );
    let s4 = single(&gym, bench, ws(kg(90, 0), &[8]));
    let s5 = gym.log_session(
        &SessionPlan::new()
            .template(push_a)
            .exercise(bench, ws(kg(100, 0), &[8])),
    );

    /* The template wins over the more recent S4. */
    let b = gym
        .os()
        .last_time(s5.session, s5.entries[0])
        .unwrap()
        .unwrap();
    assert_eq!(
        (b.session, b.rule),
        (s1.session, BaselineRule::SameTemplate(push_a))
    );
    assert_eq!(only(&gym, s5.session).verdict, Verdict::Matched);

    /* With S1 a deload too, no template session qualifies: fall back to
     * the most recent finished, non-deload session, S4 (90 → 100 kg). */
    gym.set_deload(s1.session, true);
    let b = gym
        .os()
        .last_time(s5.session, s5.entries[0])
        .unwrap()
        .unwrap();
    assert_eq!(
        (b.session, b.rule),
        (s4.session, BaselineRule::SameExercise)
    );
    let v = only(&gym, s5.session);
    assert_eq!(v.verdict, Verdict::Progressed);
    assert!(
        v.signals
            .contains(&load_at_reps(Up, 0, 8, kg(90, 0), kg(100, 0)))
    );
}

#[test]
fn exercises_in_one_session_resolve_their_own_baselines() {
    let gym = TestGym::new();
    let (bench, squat) = (bench(&gym), lift(&gym, "Squat"));
    let legs = template(2);
    /* The template session has squat but no bench. */
    let s1 = gym.log_session(
        &SessionPlan::new()
            .template(legs)
            .exercise(squat, ws(kg(140, 0), &[5])),
    );
    let s2 = single(&gym, bench, ws(kg(100, 0), &[8]));
    let s3 = single(&gym, squat, ws(kg(150, 0), &[5]));
    let today = gym.log_session(
        &SessionPlan::new()
            .template(legs)
            .exercise(squat, ws(kg(140, 0), &[6]))
            .exercise(bench, ws(kg(100, 0), &[7])),
    );
    let v = verdicts(&gym, today.session);
    let squat_base = v.entries[0].baseline.as_ref().unwrap();
    let bench_base = v.entries[1].baseline.as_ref().unwrap();
    /* Squat: S1 by template, not the more recent S3. */
    assert_eq!(
        (squat_base.session, squat_base.rule),
        (s1.session, BaselineRule::SameTemplate(legs))
    );
    assert_ne!(squat_base.session, s3.session);
    /* Bench: no template session has it, so the same-exercise fallback. */
    assert_eq!(
        (bench_base.session, bench_base.rule),
        (s2.session, BaselineRule::SameExercise)
    );
    assert_eq!(
        (verdict_at(&v, 0).verdict, verdict_at(&v, 1).verdict),
        (Verdict::Progressed, Verdict::Regressed)
    );
    assert_eq!((v.tally.progressed, v.tally.regressed), (1, 1));
}

#[test]
fn repeated_entries_match_by_occurrence_around_other_exercises() {
    let gym = TestGym::new();
    let (bench, squat) = (bench(&gym), lift(&gym, "Squat"));
    let s1 = gym.log_session(
        &SessionPlan::new()
            .exercise(bench, ws(kg(100, 0), &[5]))
            .exercise(squat, ws(kg(140, 0), &[5]))
            .exercise(bench, ws(kg(60, 0), &[12])),
    );
    let s2 = gym.log_session(
        &SessionPlan::new()
            .exercise(squat, ws(kg(140, 0), &[5]))
            .exercise(bench, ws(kg(100, 0), &[6]))
            .exercise(bench, ws(kg(60, 0), &[12])),
    );
    let base = |i: usize| {
        gym.os()
            .last_time(s2.session, s2.entries[i])
            .unwrap()
            .unwrap()
            .entry
            .id()
    };
    assert_eq!(base(0), s1.entries[1]);
    assert_eq!(base(1), s1.entries[0]);
    assert_eq!(base(2), s1.entries[2]);
    let v = verdicts(&gym, s2.session);
    assert_eq!(
        (0..3)
            .map(|i| verdict_at(&v, i).verdict)
            .collect::<Vec<_>>(),
        [Verdict::Matched, Verdict::Progressed, Verdict::Matched]
    );
}

#[test]
fn an_extra_repeated_entry_compares_with_the_baselines_last_entry() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let s1 = single(&gym, bench, ws(kg(100, 0), &[5]));
    let s2 = gym.log_session(
        &SessionPlan::new()
            .exercise(bench, ws(kg(100, 0), &[6]))
            .exercise(bench, ws(kg(60, 0), &[12])),
    );
    let v = verdicts(&gym, s2.session);
    for entry in &v.entries {
        assert_eq!(entry.baseline.as_ref().unwrap().entry.id(), s1.entries[0]);
    }
    assert_eq!(verdict_at(&v, 0).verdict, Verdict::Progressed);
    /* 100 × 5 vs 60 × 12: no pair shares load or reps. e1RM 116.67 →
     * 60 × 42/30 = 84.00 falls; volume 500 → 720 rises, but an e1RM drop
     * suppresses VolumeLoad Up. Spec-literal Regressed for the back-off
     * style second entry. */
    let second = verdict_at(&v, 1);
    assert_outcome(second, Verdict::Regressed, &[]);
    assert_signals(
        &second.signals,
        &[Some(Signal::E1rm {
            direction: Down,
            before: e1rm(kg(100, 0), 5, None),
            after: e1rm(kg(60, 0), 12, None),
        })],
        None,
    );
}

#[test]
fn an_entry_with_zero_sets_is_insufficient_and_never_a_baseline() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let s1 = single(&gym, bench, ws(kg(100, 0), &[8]));
    let empty = single(&gym, bench, Vec::new());
    assert_eq!(
        baseline_of(&gym, empty.session, empty.entries[0]),
        Some(s1.session)
    );
    let v = only(&gym, empty.session);
    assert_outcome(&v, Verdict::InsufficientData, &[]);

    /* DD-15 rule 5: the empty entry doesn't hide S1 from the next session. */
    let next = single(&gym, bench, ws(kg(100, 0), &[8]));
    assert_eq!(
        baseline_of(&gym, next.session, next.entries[0]),
        Some(s1.session)
    );
    assert_eq!(only(&gym, next.session).verdict, Verdict::Matched);
}

#[test]
fn marking_a_baseline_as_deload_later_moves_the_comparison() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let k = kg(100, 0);
    let s1 = single(&gym, bench, ws(k, &[10]));
    let s2 = single(&gym, bench, ws(k, &[8]));
    let s3 = single(&gym, bench, ws(k, &[9]));

    /* Against S2: 8 → 9 reps. */
    assert_eq!(
        baseline_of(&gym, s3.session, s3.entries[0]),
        Some(s2.session)
    );
    assert_eq!(only(&gym, s3.session).verdict, Verdict::Progressed);

    /* S2 becomes a deload: S3 now compares with S1, 10 → 9 reps. */
    gym.set_deload(s2.session, true);
    assert_eq!(
        baseline_of(&gym, s3.session, s3.entries[0]),
        Some(s1.session)
    );
    let v = only(&gym, s3.session);
    assert_outcome(&v, Verdict::Regressed, &[]);
    assert!(v.signals.contains(&reps_at_load(Down, 0, k, 10, 9)));
    assert_eq!(only(&gym, s2.session).verdict, Verdict::Deload);

    /* S1 too: nothing qualifies, so S3 has no baseline at all. */
    gym.set_deload(s1.session, true);
    assert_eq!(baseline_of(&gym, s3.session, s3.entries[0]), None);
    assert_eq!(only(&gym, s3.session).verdict, Verdict::InsufficientData);
}

#[test]
fn most_recent_is_by_utc_start_not_by_logging_order_or_local_time() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    /* Logged first, but starts later in UTC (10:00 UTC, 05:00 local at
     * UTC−5). */
    let (later_utc, _) = logged_at(&gym, 10, -300, bench, &ws(kg(90, 0), &[8]));
    /* Logged second (an import), starts earlier in UTC (09:00 UTC) though
     * its local wall clock reads 19:00 at UTC+10. */
    let (earlier_utc, _) = logged_at(&gym, 9, 600, bench, &ws(kg(80, 0), &[8]));
    let (today, entry) = logged_at(&gym, 20, 0, bench, &ws(kg(100, 0), &[8]));

    assert_eq!(baseline_of(&gym, today, entry), Some(later_utc));
    /* The import is now the baseline of the session after it. */
    let later_entry = gym.store().snapshot().sessions()[&later_utc].exercises()[0].id();
    assert_eq!(baseline_of(&gym, later_utc, later_entry), Some(earlier_utc));
}
