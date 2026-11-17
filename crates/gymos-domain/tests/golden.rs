/*!
 * Golden overload examples A–M: the test oracle for DD-13 (DD-09).
 *
 * Every expected value is written by hand from the spec table, never
 * accepted from the implementation's own output. e1RM values are checked
 * to two decimals as the table shows them; volumes are checked exactly.
 */

#![expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers build fixtures from known-valid literals; a panic is a failed test"
)]

use gymos_domain::progression::{
    Direction, E1rm, ExerciseVerdict, ProgressionScheme, RepRange, SessionVerdict, Signal, Skipped,
    Verdict, VerdictError, VerdictInput, exercise_verdict,
};
use gymos_domain::{
    ALGORITHM_VERSION, DurationSecs, Effort, Exercise, ExerciseId, Load, LoggedSet, Name, Reps,
    Rpe, SessionExercise, SetId, SetMetrics, SetType, TrackingMode, WeightUnit,
};
use uuid::Uuid;

use Direction::{Down, Up};
use SetType::{Backoff, Drop, TopSet, Warmup, Working};

/* ---- builders ---------------------------------------------------------- */

/** `kg(102, 50)` is 102.50 kg. */
fn kg(whole: u32, hundredths: u32) -> Load {
    Load::new(whole * 100 + hundredths, WeightUnit::Kilograms).unwrap()
}

/** `lb(220, 46)` is 220.46 lb. */
fn lb(whole: u32, hundredths: u32) -> Load {
    Load::new(whole * 100 + hundredths, WeightUnit::Pounds).unwrap()
}

fn reps(n: u16) -> Reps {
    Reps::new(n).unwrap()
}

fn rpe(half_points: u8) -> Effort {
    Effort::Rpe(Rpe::from_half_points(half_points).unwrap())
}

fn bench() -> Exercise {
    Exercise {
        id: ExerciseId::from_uuid(Uuid::from_u128(1)),
        name: Name::new("Bench Press").unwrap(),
        mode: TrackingMode::WeightReps,
    }
}

/** One logged set: type, load, reps and an optional effort. */
type Spec = (SetType, Load, u16, Option<Effort>);

fn w(load: Load, n: u16) -> Spec {
    (Working, load, n, None)
}

fn entry(sets: &[Spec]) -> SessionExercise {
    let mut entry = SessionExercise::new(&bench());
    for (i, &(set_type, load, n, effort)) in (0_u128..).zip(sets) {
        entry
            .push_set(LoggedSet {
                id: SetId::from_uuid(Uuid::from_u128(100 + i)),
                set_type,
                metrics: SetMetrics::WeightReps {
                    load,
                    reps: reps(n),
                },
                effort,
                note: None,
            })
            .unwrap();
    }
    entry
}

fn verdict(
    baseline: Option<&[Spec]>,
    current: &[Spec],
    deload: bool,
    scheme: Option<&ProgressionScheme>,
) -> ExerciseVerdict {
    let baseline = baseline.map(entry);
    exercise_verdict(&VerdictInput {
        baseline: baseline.as_ref(),
        current: &entry(current),
        current_is_deload: deload,
        scheme,
    })
    .unwrap()
}

fn compare(baseline: &[Spec], current: &[Spec]) -> ExerciseVerdict {
    verdict(Some(baseline), current, false, None)
}

/* ---- expected values --------------------------------------------------- */

fn e1rm(load: Load, n: u16, effort: Option<Effort>) -> E1rm {
    E1rm::of_set(load, reps(n), effort).unwrap()
}

fn e1rm_signal(direction: Direction, before: E1rm, after: E1rm, kg: (f64, f64)) -> Signal {
    let rounded = |e: E1rm| (e.kilograms() * 100.0).round() / 100.0;
    assert!(
        (rounded(before) - kg.0).abs() < 1e-9,
        "{} ≠ {}",
        before.kilograms(),
        kg.0
    );
    assert!(
        (rounded(after) - kg.1).abs() < 1e-9,
        "{} ≠ {}",
        after.kilograms(),
        kg.1
    );
    Signal::E1rm {
        direction,
        before,
        after,
    }
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

/**
 * Checks every signal against `expected`, in order. A `None` slot stands for
 * the volume signal, checked exactly against `volume`: (direction, before,
 * after) in nanogram-reps, built with [`kg_reps`].
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
                volume,
            );
        }
    }
}

const VOLUME: Option<Signal> = None;

/**
 * Checks everything in the verdict except the signals: the outcome, the
 * skipped rules and the algorithm version.
 */
fn assert_outcome(v: &ExerciseVerdict, verdict: Verdict, skipped: &[Skipped]) {
    assert_eq!(
        (v.verdict, v.skipped.as_slice(), v.algorithm_version),
        (verdict, skipped, ALGORITHM_VERSION),
    );
}

/** `kg_reps(1232, 50)` is 1232.50 kg·reps, exactly, in nanogram-reps. */
fn kg_reps(whole: i128, hundredths: i128) -> i128 {
    (whole * 100 + hundredths) * 10_000_000_000
}

/* ---- examples A–M ------------------------------------------------------ */

fn example_a() -> (Vec<Spec>, Vec<Spec>) {
    let k = kg(100, 0);
    (
        vec![w(k, 8), w(k, 8), w(k, 7)],
        vec![w(k, 8), w(k, 9), w(k, 7)],
    )
}

#[test]
fn a_one_more_rep_progresses() {
    let (baseline, current) = example_a();
    let v = compare(&baseline, &current);
    let k = kg(100, 0);
    assert_outcome(&v, Verdict::Progressed, &[]);
    assert_signals(
        &v.signals,
        &[
            Some(e1rm_signal(
                Up,
                e1rm(k, 8, None),
                e1rm(k, 9, None),
                (126.67, 130.00),
            )),
            Some(reps_at_load(Up, 1, k, 8, 9)),
            VOLUME,
        ],
        Some((Up, kg_reps(2300, 0), kg_reps(2400, 0))),
    );
}

fn example_b() -> (Vec<Spec>, Vec<Spec>) {
    (vec![w(kg(100, 0), 8); 3], vec![w(kg(102, 50), 6); 3])
}

#[test]
fn b_double_progression_load_step_progresses() {
    let (baseline, current) = example_b();
    let range = RepRange::new(reps(6), reps(8)).unwrap();
    let scheme = ProgressionScheme::new(range, kg(2, 50)).unwrap();
    let v = verdict(Some(&baseline), &current, false, Some(&scheme));
    assert_outcome(&v, Verdict::Progressed, &[]);
    assert_signals(
        &v.signals,
        &[
            Some(e1rm_signal(
                Down,
                e1rm(kg(100, 0), 8, None),
                e1rm(kg(102, 50), 6, None),
                (126.67, 123.00),
            )),
            Some(Signal::LoadStep {
                before: kg(100, 0),
                after: kg(102, 50),
                reps: reps(6),
            }),
            VOLUME,
        ],
        Some((Down, kg_reps(2400, 0), kg_reps(1845, 0))),
    );
}

#[test]
fn b_prime_the_same_session_without_a_scheme_regresses() {
    let (baseline, current) = example_b();
    let v = compare(&baseline, &current);
    assert_outcome(&v, Verdict::Regressed, &[]);
    assert_signals(
        &v.signals,
        &[
            Some(e1rm_signal(
                Down,
                e1rm(kg(100, 0), 8, None),
                e1rm(kg(102, 50), 6, None),
                (126.67, 123.00),
            )),
            VOLUME,
        ],
        Some((Down, kg_reps(2400, 0), kg_reps(1845, 0))),
    );
}

#[test]
fn c_the_same_work_at_a_lower_rpe_progresses() {
    let k = kg(100, 0);
    let v = compare(
        &[(Working, k, 8, Some(rpe(18)))],
        &[(Working, k, 8, Some(rpe(16)))],
    );
    assert_outcome(&v, Verdict::Progressed, &[]);
    assert_signals(
        &v.signals,
        &[
            Some(e1rm_signal(
                Up,
                e1rm(k, 8, Some(rpe(18))),
                e1rm(k, 8, Some(rpe(16))),
                (130.00, 133.33),
            )),
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
fn d_the_same_session_matches() {
    let sets = vec![w(kg(100, 0), 8); 3];
    let v = compare(&sets, &sets);
    assert_outcome(&v, Verdict::Matched, &[]);
    assert_eq!(v.signals, []);
}

#[test]
fn e_fewer_reps_regresses() {
    let k = kg(100, 0);
    let v = compare(&[w(k, 8); 3], &[w(k, 8), w(k, 7), w(k, 6)]);
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
}

#[test]
fn f_a_first_session_is_insufficient_data() {
    let v = verdict(None, &[w(kg(100, 0), 8)], false, None);
    assert_outcome(&v, Verdict::InsufficientData, &[]);
    assert_eq!(v.signals, []);
}

#[test]
fn g_a_deload_is_never_a_regression_but_still_explains_itself() {
    let (before, after) = (kg(100, 0), kg(80, 0));
    let v = verdict(Some(&[w(before, 8); 3]), &[w(after, 8); 3], true, None);
    assert_outcome(&v, Verdict::Deload, &[]);
    assert_signals(
        &v.signals,
        &[
            Some(e1rm_signal(
                Down,
                e1rm(before, 8, None),
                e1rm(after, 8, None),
                (126.67, 101.33),
            )),
            Some(load_at_reps(Down, 0, 8, before, after)),
            Some(load_at_reps(Down, 1, 8, before, after)),
            Some(load_at_reps(Down, 2, 8, before, after)),
            VOLUME,
        ],
        Some((Down, kg_reps(2400, 0), kg_reps(1920, 0))),
    );
}

#[test]
fn h_a_mixed_session_progresses_and_lists_the_drop() {
    let k = kg(100, 0);
    let v = compare(&[w(k, 8); 3], &[w(k, 9), w(k, 8), w(k, 6)]);
    assert_outcome(&v, Verdict::Progressed, &[]);
    assert_signals(
        &v.signals,
        &[
            Some(e1rm_signal(
                Up,
                e1rm(k, 8, None),
                e1rm(k, 9, None),
                (126.67, 130.00),
            )),
            Some(reps_at_load(Up, 0, k, 8, 9)),
            Some(reps_at_load(Down, 2, k, 8, 6)),
            VOLUME,
        ],
        Some((Down, kg_reps(2400, 0), kg_reps(2300, 0))),
    );
}

#[test]
fn i_an_extra_identical_set_progresses() {
    let k = kg(100, 0);
    let v = compare(&[w(k, 8); 3], &[w(k, 8); 4]);
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
fn j_switching_units_at_the_same_load_matches() {
    let v = compare(&[w(kg(100, 0), 8)], &[w(lb(220, 46), 8)]);
    assert_outcome(&v, Verdict::Matched, &[]);
    assert_eq!(v.signals, []);
}

#[test]
fn k_high_rep_sets_skip_e1rm_but_still_progress() {
    let k = kg(60, 0);
    let v = compare(&[w(k, 15), w(k, 15)], &[w(k, 16), w(k, 15)]);
    assert_outcome(&v, Verdict::Progressed, &[Skipped::E1rmAboveRepCap]);
    assert_signals(
        &v.signals,
        &[Some(reps_at_load(Up, 0, k, 15, 16)), VOLUME],
        Some((Up, kg_reps(1800, 0), kg_reps(1860, 0))),
    );
}

#[test]
fn l_top_sets_pair_with_top_sets_and_backoffs_only_add_volume() {
    let backoff = (Backoff, kg(90, 0), 8, None);
    let v = compare(
        &[(TopSet, kg(100, 0), 5, None), backoff],
        &[(TopSet, kg(102, 50), 5, None), backoff],
    );
    assert_outcome(&v, Verdict::Progressed, &[]);
    assert_signals(
        &v.signals,
        &[
            Some(e1rm_signal(
                Up,
                e1rm(kg(100, 0), 5, None),
                e1rm(kg(102, 50), 5, None),
                (116.67, 119.58),
            )),
            Some(load_at_reps(Up, 0, 5, kg(100, 0), kg(102, 50))),
            VOLUME,
        ],
        Some((Up, kg_reps(1220, 0), kg_reps(1232, 50))),
    );
}

#[test]
fn m_other_tracking_modes_are_unsupported() {
    let plank = Exercise {
        id: ExerciseId::from_uuid(Uuid::from_u128(2)),
        name: Name::new("Plank").unwrap(),
        mode: TrackingMode::Duration,
    };
    let mut current = SessionExercise::new(&plank);
    current
        .push_set(LoggedSet {
            id: SetId::from_uuid(Uuid::from_u128(200)),
            set_type: Working,
            metrics: SetMetrics::Duration {
                duration: DurationSecs::new(60).unwrap(),
                load: None,
            },
            effort: None,
            note: None,
        })
        .unwrap();
    let result = exercise_verdict(&VerdictInput {
        baseline: Some(&entry(&[w(kg(100, 0), 8)])),
        current: &current,
        current_is_deload: false,
        scheme: None,
    });
    assert_eq!(
        result,
        Err(VerdictError::UnsupportedMode(TrackingMode::Duration))
    );
}

/* ---- extra cases ------------------------------------------------------- */

#[test]
fn warmups_change_nothing() {
    let (mut baseline, mut current) = example_a();
    let warmup = (Warmup, kg(60, 0), 10, None);
    baseline.insert(0, warmup);
    current.insert(0, warmup);
    let (plain_baseline, plain_current) = example_a();
    assert_eq!(
        compare(&baseline, &current),
        compare(&plain_baseline, &plain_current)
    );
}

#[test]
fn drop_sets_add_volume_but_are_never_paired() {
    let k = kg(100, 0);
    let v = compare(
        &[w(k, 8), (Drop, kg(70, 0), 10, None)],
        &[w(k, 8), (Drop, kg(70, 0), 12, None)],
    );
    /* No RepsAtLoad for the drop set: only the volume moved. */
    assert_outcome(&v, Verdict::Progressed, &[]);
    assert_signals(
        &v.signals,
        &[VOLUME],
        Some((Up, kg_reps(1500, 0), kg_reps(1640, 0))),
    );
}

#[test]
fn an_extra_set_at_another_load_is_not_another_working_set() {
    let k = kg(100, 0);
    let mut current = vec![w(k, 8); 3];
    current.push(w(kg(0, 0), 1));
    let v = compare(&[w(k, 8); 3], &current);
    /* Same e1RM, same volume, and the extra set repeats no work. */
    assert_outcome(&v, Verdict::Matched, &[]);
    assert_eq!(v.signals, []);
}

fn all_verdicts() -> Vec<ExerciseVerdict> {
    let (a_base, a_cur) = example_a();
    let (b_base, b_cur) = example_b();
    let k = kg(100, 0);
    vec![
        compare(&a_base, &a_cur),                                  // Progressed
        compare(&b_base, &b_cur),                                  // Regressed (B′)
        compare(&[w(k, 8)], &[w(k, 8)]),                           // Matched
        verdict(None, &[w(k, 8)], false, None),                    // InsufficientData
        verdict(Some(&[w(k, 8)]), &[w(kg(80, 0), 8)], true, None), // Deload
    ]
}

#[test]
fn only_progressed_is_progressive_overload() {
    let flags: Vec<(Verdict, bool)> = all_verdicts()
        .iter()
        .map(|v| (v.verdict, v.is_progressive_overload()))
        .collect();
    assert_eq!(
        flags,
        [
            (Verdict::Progressed, true),
            (Verdict::Regressed, false),
            (Verdict::Matched, false),
            (Verdict::InsufficientData, false),
            (Verdict::Deload, false),
        ]
    );
}

#[test]
fn session_verdict_counts_each_outcome() {
    let verdicts = all_verdicts();
    let mut session = SessionVerdict::tally(&verdicts);
    assert_eq!(
        session,
        SessionVerdict {
            progressed: 1,
            matched: 1,
            regressed: 1,
            insufficient: 1,
            deload: 1,
        }
    );
    assert_eq!(session.total(), 5);
    session = SessionVerdict::tally(verdicts.iter().take(1).chain(&verdicts));
    assert_eq!((session.progressed, session.total()), (2, 6));
}
