/*!
 * Personal-record (PR) rules: what beats what, for every record kind
 * (DD-14).
 *
 * The record detectors arrive with the PR engine. These tests lock in the
 * rules they will be built on: each kind's "better" direction, that a tie
 * is never a new record, that values of different kinds never compare, and
 * how a history of attempts turns into a series of records.
 */

#![expect(
    clippy::unwrap_used,
    reason = "test helpers build fixtures from known-valid literals; a panic is a failed test"
)]

use gymos_domain::{
    ALGORITHM_VERSION, ActivityKind, Better, CustomValue, Distance, DurationSecs, ElapsedTime,
    ExerciseId, HeartRate, Load, MeasurementKind, MetricId, Quantity, Record, RecordError,
    RecordKind, RecordSource, Reps, SessionId, StandardDistance, Subject, Timestamp, UtcOffset,
    Uuid, Vo2Max, WeightUnit,
};

/* ---- values ------------------------------------------------------------- */

fn kg(whole: u32, hundredths: u32) -> Quantity {
    Quantity::Mass(
        Load::new(whole * 100 + hundredths, WeightUnit::Kilograms)
            .unwrap()
            .mass(),
    )
}

fn reps(n: u16) -> Quantity {
    Quantity::Reps(Reps::new(n).unwrap())
}

fn hold(seconds: u32) -> Quantity {
    Quantity::Duration(DurationSecs::new(seconds).unwrap())
}

fn time(seconds: u32) -> Quantity {
    Quantity::Elapsed(ElapsedTime::new(seconds).unwrap())
}

fn km(metres: u32) -> Quantity {
    Quantity::Distance(Distance::from_millimetres(metres * 1_000).unwrap())
}

fn vo2(tenths: u16) -> Quantity {
    Quantity::Vo2Max(Vo2Max::from_tenths(tenths).unwrap())
}

fn bpm(n: u8) -> Quantity {
    Quantity::HeartRate(HeartRate::from_bpm(n).unwrap())
}

fn custom(thousandths: i64) -> Quantity {
    Quantity::Custom(CustomValue::from_thousandths(thousandths).unwrap())
}

fn bench() -> Subject {
    Subject::Exercise(ExerciseId::from_uuid(Uuid::from_u128(1)))
}

/* ---- every kind's direction -------------------------------------------- */

/**
 * The oracle, written from DD-14 and product §3.5 rather than from
 * `fixed_better`. The match is exhaustive, so a new record kind fails to
 * compile here until someone decides its direction.
 */
const fn expected_direction(kind: RecordKind) -> Option<Better> {
    match kind {
        /* Faster times and less assistance are better. */
        RecordKind::FastestDistance(_) | RecordKind::LeastAssistance(_) => Some(Better::Lower),
        /* Decided by the measurement series or custom metric. */
        RecordKind::BestValue => None,
        RecordKind::RepMax(_)
        | RecordKind::EstimatedOneRepMax
        | RecordKind::MaxRepsAtLoad(_)
        | RecordKind::BestSetVolume
        | RecordKind::BestSessionVolume
        | RecordKind::HeaviestLoad
        | RecordKind::MaxRepsInSet
        | RecordKind::MaxRepsInSession
        | RecordKind::LongestHold
        | RecordKind::LongestDistance
        | RecordKind::LongestDuration
        | RecordKind::BiggestWeek
        | RecordKind::BiggestMonth
        | RecordKind::LongestStreakWeeks
        | RecordKind::MostSessionsInMonth => Some(Better::Higher),
    }
}

/**
 * One scenario per record kind: the subject it belongs to, the direction
 * that wins, and a worse, an equal and a better attempt against a previous
 * best of `equal`.
 */
struct Case {
    subject: Subject,
    kind: RecordKind,
    better: Better,
    worse: Quantity,
    equal: Quantity,
    improved: Quantity,
}

#[expect(
    clippy::too_many_lines,
    reason = "one row per record kind; the whole table reads best in one place"
)]
fn every_kind() -> Vec<Case> {
    let run = Subject::Activity(ActivityKind::Run);
    let ride = Subject::Activity(ActivityKind::Ride);
    let pull_up = Subject::Exercise(ExerciseId::from_uuid(Uuid::from_u128(2)));
    let plank = Subject::Exercise(ExerciseId::from_uuid(Uuid::from_u128(3)));
    let mut cases = vec![
        /* Strength. */
        Case {
            subject: bench(),
            kind: RecordKind::RepMax(Reps::new(5).unwrap()),
            better: Better::Higher,
            worse: kg(100, 0),
            equal: kg(102, 50),
            improved: kg(105, 0),
        },
        Case {
            subject: bench(),
            kind: RecordKind::EstimatedOneRepMax,
            better: Better::Higher,
            worse: kg(120, 0),
            equal: kg(126, 67),
            improved: kg(126, 68),
        },
        Case {
            subject: bench(),
            kind: RecordKind::MaxRepsAtLoad(
                Load::new(10_000, WeightUnit::Kilograms).unwrap().mass(),
            ),
            better: Better::Higher,
            worse: reps(7),
            equal: reps(8),
            improved: reps(9),
        },
        Case {
            subject: bench(),
            kind: RecordKind::HeaviestLoad,
            better: Better::Higher,
            worse: kg(140, 0),
            equal: kg(140, 25),
            improved: kg(140, 50),
        },
        /* Bodyweight and holds. */
        Case {
            subject: pull_up,
            kind: RecordKind::MaxRepsInSet,
            better: Better::Higher,
            worse: reps(11),
            equal: reps(12),
            improved: reps(13),
        },
        Case {
            subject: pull_up,
            kind: RecordKind::MaxRepsInSession,
            better: Better::Higher,
            worse: reps(40),
            equal: reps(45),
            improved: reps(46),
        },
        Case {
            subject: pull_up,
            kind: RecordKind::LeastAssistance(Reps::new(5).unwrap()),
            better: Better::Lower,
            worse: kg(25, 0),
            equal: kg(20, 0),
            improved: kg(15, 0),
        },
        Case {
            subject: plank,
            kind: RecordKind::LongestHold,
            better: Better::Higher,
            worse: hold(119),
            equal: hold(120),
            improved: hold(121),
        },
        /* Running and other activities. */
        Case {
            subject: run,
            kind: RecordKind::LongestDistance,
            better: Better::Higher,
            worse: km(20_000),
            equal: km(21_098),
            improved: km(21_100),
        },
        Case {
            subject: ride,
            kind: RecordKind::LongestDuration,
            better: Better::Higher,
            worse: time(10_000),
            equal: time(10_800),
            improved: time(10_801),
        },
        Case {
            subject: run,
            kind: RecordKind::BiggestWeek,
            better: Better::Higher,
            worse: km(40_000),
            equal: km(52_000),
            improved: km(52_001),
        },
        Case {
            subject: run,
            kind: RecordKind::BiggestMonth,
            better: Better::Higher,
            worse: km(150_000),
            equal: km(200_000),
            improved: km(210_000),
        },
        /* Measurements: the direction comes from the series. */
        Case {
            subject: Subject::Measurement(MeasurementKind::Vo2Max),
            kind: RecordKind::BestValue,
            better: Better::Higher,
            worse: vo2(500),
            equal: vo2(520),
            improved: vo2(521),
        },
        Case {
            subject: Subject::Measurement(MeasurementKind::RestingHeartRate),
            kind: RecordKind::BestValue,
            better: Better::Lower,
            worse: bpm(55),
            equal: bpm(52),
            improved: bpm(51),
        },
        Case {
            subject: Subject::Measurement(MeasurementKind::Custom(MetricId::from_uuid(
                Uuid::from_u128(9),
            ))),
            kind: RecordKind::BestValue,
            better: Better::Lower,
            worse: custom(-500),
            equal: custom(-750),
            improved: custom(-751),
        },
        /* Training as a whole. */
        Case {
            subject: Subject::Overall,
            kind: RecordKind::LongestStreakWeeks,
            better: Better::Higher,
            worse: Quantity::Count(11),
            equal: Quantity::Count(12),
            improved: Quantity::Count(13),
        },
        Case {
            subject: Subject::Overall,
            kind: RecordKind::MostSessionsInMonth,
            better: Better::Higher,
            worse: Quantity::Count(15),
            equal: Quantity::Count(16),
            improved: Quantity::Count(17),
        },
    ];
    /* Fastest time over every standard race distance. */
    for (distance, pr) in [
        (StandardDistance::Mile, 360),
        (StandardDistance::FiveK, 1_320),
        (StandardDistance::TenK, 2_820),
        (StandardDistance::HalfMarathon, 6_300),
        (StandardDistance::Marathon, 13_500),
    ] {
        cases.push(Case {
            subject: Subject::Activity(ActivityKind::Run),
            kind: RecordKind::FastestDistance(distance),
            better: Better::Lower,
            worse: time(pr + 1),
            equal: time(pr),
            improved: time(pr - 1),
        });
    }
    cases
}

fn record(
    case: &Case,
    value: Quantity,
    better: Better,
    previous: Option<Quantity>,
) -> Result<Record, RecordError> {
    Record::new(
        case.subject,
        case.kind,
        value,
        better,
        Timestamp::new(1_000, UtcOffset::UTC),
        RecordSource::Session(SessionId::from_uuid(Uuid::from_u128(5))),
        previous,
        ALGORITHM_VERSION,
    )
}

#[test]
fn every_kind_has_the_direction_the_spec_gives_it() {
    for case in every_kind() {
        assert_eq!(
            case.kind.fixed_better(),
            expected_direction(case.kind),
            "{:?}",
            case.kind
        );
        if let Some(fixed) = expected_direction(case.kind) {
            assert_eq!(case.better, fixed, "{:?}", case.kind);
        }
    }
}

/*
 * Volume records (load × reps) have no value cases above: `Quantity` has no
 * variant that holds a volume yet. `Mass` is a weight capped at 1000 kg,
 * while one session's volume is often several thousand kg·reps, and the
 * verdict rules already use a separate `Volume` type. Their direction is
 * still checked here; their values are an open question for the PR engine.
 */
#[test]
fn volume_records_are_higher_is_better() {
    for kind in [RecordKind::BestSetVolume, RecordKind::BestSessionVolume] {
        assert_eq!(kind.fixed_better(), expected_direction(kind));
        assert_eq!(kind.fixed_better(), Some(Better::Higher));
    }
}

#[test]
fn only_a_strictly_better_attempt_beats_the_previous_best() {
    for case in every_kind() {
        let kind = case.kind;
        assert_eq!(
            case.improved.beats(&case.equal, case.better),
            Some(true),
            "{kind:?}: improvement"
        );
        assert_eq!(
            case.equal.beats(&case.equal, case.better),
            Some(false),
            "{kind:?}: a tie is not a record"
        );
        assert_eq!(
            case.worse.beats(&case.equal, case.better),
            Some(false),
            "{kind:?}: worse"
        );
    }
}

#[test]
fn every_kind_builds_a_record_that_remembers_what_it_beat() {
    for case in every_kind() {
        let pr = record(&case, case.improved, case.better, Some(case.equal)).unwrap();
        assert_eq!(
            (
                pr.subject(),
                pr.kind(),
                pr.value(),
                pr.better(),
                pr.previous()
            ),
            (
                case.subject,
                case.kind,
                case.improved,
                case.better,
                Some(case.equal)
            ),
            "{:?}",
            case.kind
        );
        /* The first attempt ever is a record with nothing before it. */
        assert!(record(&case, case.equal, case.better, None).is_ok());
    }
}

#[test]
fn a_kind_with_a_fixed_direction_refuses_the_opposite_one() {
    for case in every_kind() {
        let opposite = match case.better {
            Better::Higher => Better::Lower,
            Better::Lower => Better::Higher,
        };
        let result = record(&case, case.improved, opposite, None);
        match case.kind.fixed_better() {
            Some(expected) => assert_eq!(
                result,
                Err(RecordError::DirectionMismatch {
                    kind: case.kind,
                    expected,
                    found: opposite
                }),
                "{:?}",
                case.kind
            ),
            /* BestValue takes either direction: it belongs to the series. */
            None => assert!(result.is_ok()),
        }
    }
}

#[test]
fn a_previous_value_of_another_quantity_is_refused() {
    for case in every_kind() {
        let other = match case.improved {
            Quantity::Count(_) => reps(1),
            Quantity::Mass(_)
            | Quantity::Reps(_)
            | Quantity::Duration(_)
            | Quantity::Elapsed(_)
            | Quantity::Distance(_)
            | Quantity::Vo2Max(_)
            | Quantity::HeartRate(_)
            | Quantity::Custom(_) => Quantity::Count(1),
        };
        assert_eq!(
            case.improved.beats(&other, case.better),
            None,
            "{:?}",
            case.kind
        );
        assert_eq!(
            record(&case, case.improved, case.better, Some(other)),
            Err(RecordError::Incomparable),
            "{:?}",
            case.kind
        );
    }
}

#[test]
fn quantities_of_different_kinds_never_compare() {
    let samples = [
        kg(100, 0),
        reps(8),
        hold(60),
        time(60),
        km(5_000),
        Quantity::Count(3),
        vo2(500),
        bpm(60),
        custom(1),
    ];
    for (i, a) in samples.iter().enumerate() {
        for (j, b) in samples.iter().enumerate() {
            assert_eq!(a.compare(b).is_some(), i == j, "{a:?} vs {b:?}");
        }
    }
}

#[test]
fn body_weight_has_no_best_direction() {
    assert_eq!(MeasurementKind::BodyWeight.built_in_better(), None);
    assert_eq!(
        MeasurementKind::Vo2Max.built_in_better(),
        Some(Better::Higher)
    );
    assert_eq!(
        MeasurementKind::RestingHeartRate.built_in_better(),
        Some(Better::Lower)
    );
}

/* ---- histories: which attempts set a record ----------------------------- */

/**
 * Walks attempts in order and returns the positions that set a new record:
 * the first attempt, then every one that beats the best so far.
 */
fn record_positions(attempts: &[Quantity], better: Better) -> Vec<usize> {
    let mut best: Option<Quantity> = None;
    let mut positions = Vec::new();
    for (i, attempt) in attempts.iter().enumerate() {
        let is_record = best.is_none_or(|b| attempt.beats(&b, better) == Some(true));
        if is_record {
            best = Some(*attempt);
            positions.push(i);
        }
    }
    positions
}

#[test]
fn a_heaviest_lift_history_counts_only_strict_improvements() {
    let attempts = [
        kg(100, 0),
        kg(102, 50),
        kg(102, 50),
        kg(100, 0),
        kg(102, 75),
        kg(105, 0),
    ];
    assert_eq!(record_positions(&attempts, Better::Higher), [0, 1, 4, 5]);
}

#[test]
fn a_5k_history_counts_only_faster_times() {
    /* 22:00, 21:40, 21:40 again, a slow 23:10, then 21:15. */
    let attempts = [
        time(1_320),
        time(1_300),
        time(1_300),
        time(1_390),
        time(1_275),
    ];
    assert_eq!(record_positions(&attempts, Better::Lower), [0, 1, 4]);
}

#[test]
fn an_assisted_pull_up_history_counts_less_assistance() {
    let attempts = [kg(30, 0), kg(25, 0), kg(27, 50), kg(25, 0), kg(20, 0)];
    assert_eq!(record_positions(&attempts, Better::Lower), [0, 1, 4]);
}

#[test]
fn a_resting_heart_rate_history_counts_lower_readings() {
    let attempts = [bpm(62), bpm(60), bpm(61), bpm(58), bpm(58)];
    assert_eq!(record_positions(&attempts, Better::Lower), [0, 1, 3]);
}

#[test]
fn a_run_distance_history_counts_longer_runs() {
    let attempts = [km(5_000), km(10_000), km(8_000), km(10_000), km(21_098)];
    assert_eq!(record_positions(&attempts, Better::Higher), [0, 1, 4]);
}

#[test]
fn standard_race_distances_are_exact_in_millimetres() {
    let expected = [
        (StandardDistance::Mile, 1_609_344),
        (StandardDistance::FiveK, 5_000_000),
        (StandardDistance::TenK, 10_000_000),
        (StandardDistance::HalfMarathon, 21_097_500),
        (StandardDistance::Marathon, 42_195_000),
    ];
    for (distance, mm) in expected {
        assert_eq!(distance.millimetres(), mm, "{distance:?}");
    }
}
