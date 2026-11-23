/*!
 * The `last_time` and `session_verdicts` queries on `GymOs<TestPorts>`:
 * the walking skeleton, baseline selection (D-1) and verdicts computed on
 * read (D-6).
 */

#![expect(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers build fixtures from known-valid literals; a panic is a failed test"
)]

use gymos_domain::progression::{
    Direction, ExerciseVerdict, ProgressionScheme, RepRange, Signal, Verdict, VerdictError,
};
use gymos_domain::{
    DurationSecs, ExerciseId, Load, SessionExerciseId, SessionId, SetMetrics, Source, TemplateId,
    TrackingMode, Uuid,
};
use gymos_engine::{
    BaselineRule, DomainError, EngineError, EntryOutcome, Missing, SessionVerdicts, StartSession,
    Store, WriteTx,
};
use gymos_testkit::{
    SessionBuilder, SessionPlan, SetBuilder, TestGym, at, exercise, kg, reps, stories, working,
};

/* ---- helpers ------------------------------------------------------------ */

fn bench(gym: &TestGym) -> ExerciseId {
    gym.create_exercise("Bench Press", TrackingMode::WeightReps, None)
}

fn template(n: u128) -> TemplateId {
    TemplateId::from_uuid(Uuid::from_u128(0x7E << 64 | n))
}

fn sets(load: Load, reps: &[u16]) -> Vec<SetBuilder> {
    reps.iter().map(|&n| working(load, n)).collect()
}

fn one(gym: &TestGym, plan: &SessionPlan) -> (SessionId, SessionExerciseId) {
    let logged = gym.log_session(plan);
    (logged.session, logged.entries[0])
}

/** The verdict of the session's only entry. */
fn only_verdict(verdicts: &SessionVerdicts) -> &ExerciseVerdict {
    assert_eq!(verdicts.entries.len(), 1);
    match &verdicts.entries[0].outcome {
        EntryOutcome::Verdict(verdict) => verdict,
        EntryOutcome::Unsupported(mode) => panic!("unexpected unsupported {mode:?}"),
    }
}

fn baseline_session(
    gym: &TestGym,
    session: SessionId,
    entry: SessionExerciseId,
) -> Option<SessionId> {
    gym.os()
        .last_time(session, entry)
        .unwrap()
        .map(|b| b.session)
}

/* ---- walking skeleton --------------------------------------------------- */

/* Golden A, written from the spec table. The story is shared with the
 * SQLite store's tests (S-9), so it lives in `gymos_testkit::stories`. */
#[test]
fn walking_skeleton_matches_golden_example_a() {
    stories::walking_skeleton_matches_golden_example_a(&TestGym::new());
}

#[test]
fn a_first_session_is_insufficient_data() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let (s1, entry) = one(
        &gym,
        &SessionPlan::new().exercise(bench, sets(kg(100, 0), &[8])),
    );
    assert_eq!(gym.os().last_time(s1, entry).unwrap(), None);
    let verdicts = gym.os().session_verdicts(s1).unwrap();
    assert_eq!(verdicts.entries[0].baseline, None);
    assert_eq!(only_verdict(&verdicts).verdict, Verdict::InsufficientData);
    assert_eq!(verdicts.tally.insufficient, 1);
}

/* ---- baseline selection (D-1) ------------------------------------------ */

#[test]
fn deloads_are_skipped_as_baselines_and_get_a_deload_verdict() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let (s1, _) = one(
        &gym,
        &SessionPlan::new().exercise(bench, sets(kg(100, 0), &[8, 8, 8])),
    );
    let (s2, _) = one(
        &gym,
        &SessionPlan::new()
            .deload()
            .exercise(bench, sets(kg(80, 0), &[8, 8, 8])),
    );
    let (s3, s3_entry) = one(
        &gym,
        &SessionPlan::new().exercise(bench, sets(kg(100, 0), &[8, 8, 8])),
    );

    assert_eq!(baseline_session(&gym, s3, s3_entry), Some(s1));
    /* Against the deload it would have "progressed"; against S1 it matches. */
    let s3_verdicts = gym.os().session_verdicts(s3).unwrap();
    assert_eq!(only_verdict(&s3_verdicts).verdict, Verdict::Matched);

    let s2_verdicts = gym.os().session_verdicts(s2).unwrap();
    let deload = only_verdict(&s2_verdicts);
    assert_eq!(deload.verdict, Verdict::Deload);
    assert!(
        deload
            .signals
            .iter()
            .all(|s| s.direction() == Direction::Down)
    );
    assert!(!deload.signals.is_empty(), "a deload still explains itself");
    assert_eq!(s2_verdicts.tally.deload, 1);
}

#[test]
fn template_sessions_compare_with_the_same_template_first() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let (push_a, push_b) = (template(1), template(2));
    let (s1, _) = one(
        &gym,
        &SessionPlan::new()
            .template(push_a)
            .exercise(bench, sets(kg(100, 0), &[8, 8, 8])),
    );
    let (s2, _) = one(
        &gym,
        &SessionPlan::new()
            .template(push_b)
            .exercise(bench, sets(kg(80, 0), &[12, 12, 12])),
    );
    let (s3, s3_entry) = one(
        &gym,
        &SessionPlan::new()
            .template(push_a)
            .exercise(bench, sets(kg(100, 0), &[8, 8, 8])),
    );

    let baseline = gym.os().last_time(s3, s3_entry).unwrap().unwrap();
    assert_eq!(
        (baseline.session, baseline.rule),
        (s1, BaselineRule::SameTemplate(push_a))
    );
    let verdicts = gym.os().session_verdicts(s3).unwrap();
    assert_eq!(only_verdict(&verdicts).verdict, Verdict::Matched);

    /* A template with no earlier session falls back to the same exercise. */
    let (s4, s4_entry) = one(
        &gym,
        &SessionPlan::new()
            .template(template(3))
            .exercise(bench, sets(kg(100, 0), &[8])),
    );
    let fallback = gym.os().last_time(s4, s4_entry).unwrap().unwrap();
    assert_eq!(
        (fallback.session, fallback.rule),
        (s3, BaselineRule::SameExercise)
    );
    assert_ne!(fallback.session, s2);
}

#[test]
fn unfinished_sessions_are_never_a_baseline() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let (s1, _) = one(
        &gym,
        &SessionPlan::new().exercise(bench, sets(kg(100, 0), &[8])),
    );
    let (s2, s2_entry) = one(
        &gym,
        &SessionPlan::new()
            .unfinished()
            .exercise(bench, sets(kg(120, 0), &[8])),
    );
    let (s3, s3_entry) = one(
        &gym,
        &SessionPlan::new().exercise(bench, sets(kg(100, 0), &[8])),
    );

    assert_eq!(baseline_session(&gym, s3, s3_entry), Some(s1));
    /* The unfinished session itself still gets a verdict against S1. */
    assert_eq!(baseline_session(&gym, s2, s2_entry), Some(s1));
}

#[test]
fn entries_without_counted_sets_are_skipped_not_compared() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let push_a = template(1);
    let warmup = working(kg(60, 0), 10).set_type(gymos_domain::SetType::Warmup);
    let (s1, _) = one(
        &gym,
        &SessionPlan::new()
            .template(push_a)
            .exercise(bench, sets(kg(100, 0), &[8, 8, 8])),
    );
    /* Newer Push A sessions with bench but nothing counted, around a
     * non-template session with real work. */
    one(
        &gym,
        &SessionPlan::new()
            .template(push_a)
            .exercise(bench, [warmup]),
    );
    let (s3, _) = one(
        &gym,
        &SessionPlan::new().exercise(bench, sets(kg(90, 0), &[8])),
    );
    one(
        &gym,
        &SessionPlan::new().template(push_a).exercise(bench, []),
    );

    /* Without a template: the newest (empty Push A) session is skipped and
     * the most recent session with counted sets, S3, is used. */
    let (plain, plain_entry) = one(
        &gym,
        &SessionPlan::new().exercise(bench, sets(kg(90, 0), &[8])),
    );
    let fallback = gym.os().last_time(plain, plain_entry).unwrap().unwrap();
    assert_eq!(
        (fallback.session, fallback.rule),
        (s3, BaselineRule::SameExercise)
    );

    /* With Push A, template first still wins: both empty Push A sessions are
     * skipped and the older Push A with counted sets is used, not the newer
     * non-template sessions. */
    let (today, entry) = one(
        &gym,
        &SessionPlan::new()
            .template(push_a)
            .exercise(bench, sets(kg(100, 0), &[8, 8, 8])),
    );
    let baseline = gym.os().last_time(today, entry).unwrap().unwrap();
    assert_eq!(
        (baseline.session, baseline.rule),
        (s1, BaselineRule::SameTemplate(push_a))
    );
    assert_eq!(
        only_verdict(&gym.os().session_verdicts(today).unwrap()).verdict,
        Verdict::Matched
    );
}

#[test]
fn an_empty_matched_occurrence_moves_the_search_to_an_older_session() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let s1 = gym.log_session(
        &SessionPlan::new()
            .exercise(bench, sets(kg(100, 0), &[5]))
            .exercise(bench, sets(kg(60, 0), &[12])),
    );
    /* The second bench entry was added but never logged. */
    let s2 = gym.log_session(
        &SessionPlan::new()
            .exercise(bench, sets(kg(100, 0), &[5]))
            .exercise(bench, []),
    );
    let today = gym.log_session(
        &SessionPlan::new()
            .exercise(bench, sets(kg(100, 0), &[5]))
            .exercise(bench, sets(kg(60, 0), &[12])),
    );
    let baseline = |entry| gym.os().last_time(today.session, entry).unwrap().unwrap();
    assert_eq!(baseline(today.entries[0]).session, s2.session);
    let second = baseline(today.entries[1]);
    assert_eq!(
        (second.session, second.entry.id()),
        (s1.session, s1.entries[1])
    );
}

#[test]
fn a_past_sessions_baseline_does_not_move_when_later_sessions_are_logged() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let (s1, _) = one(
        &gym,
        &SessionPlan::new().exercise(bench, sets(kg(100, 0), &[8, 8, 8])),
    );
    let (s2, s2_entry) = one(
        &gym,
        &SessionPlan::new().exercise(bench, sets(kg(100, 0), &[8, 8, 9])),
    );
    let before = gym.os().session_verdicts(s2).unwrap();
    one(
        &gym,
        &SessionPlan::new().exercise(bench, sets(kg(120, 0), &[8, 8, 8])),
    );
    assert_eq!(baseline_session(&gym, s2, s2_entry), Some(s1));
    assert_eq!(gym.os().session_verdicts(s2).unwrap(), before);
}

#[test]
fn equal_start_times_break_ties_by_session_id() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let started_at = at(1_000_000);
    let mut ids = Vec::new();
    for load in [kg(100, 0), kg(90, 0)] {
        let outcome = gym
            .run(StartSession {
                started_at: Some(started_at),
                template: None,
                title: None,
                source: Source::Manual,
            })
            .unwrap();
        let gymos_engine::CommandOutcome::SessionStarted(session) = outcome else {
            panic!("expected SessionStarted, got {outcome:?}");
        };
        let entry = gym.add_exercise(session, bench);
        gym.log_set(session, entry, &working(load, 8));
        gym.finish(session);
        ids.push(session);
    }
    let later_id = ids[0].max(ids[1]);
    let (current, entry) = one(
        &gym,
        &SessionPlan::new().exercise(bench, sets(kg(100, 0), &[8])),
    );
    assert_eq!(baseline_session(&gym, current, entry), Some(later_id));
}

#[test]
fn repeated_exercises_match_by_occurrence_and_extras_use_the_last() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let s1 = gym.log_session(
        &SessionPlan::new()
            .exercise(bench, sets(kg(100, 0), &[5]))
            .exercise(bench, sets(kg(60, 0), &[12])),
    );
    let s2 = gym.log_session(
        &SessionPlan::new()
            .exercise(bench, sets(kg(100, 0), &[5]))
            .exercise(bench, sets(kg(60, 0), &[12]))
            .exercise(bench, sets(kg(60, 0), &[12])),
    );
    let baseline_entry = |entry| {
        gym.os()
            .last_time(s2.session, entry)
            .unwrap()
            .unwrap()
            .entry
            .id()
    };
    assert_eq!(baseline_entry(s2.entries[0]), s1.entries[0]);
    assert_eq!(baseline_entry(s2.entries[1]), s1.entries[1]);
    assert_eq!(baseline_entry(s2.entries[2]), s1.entries[1]);

    let verdicts = gym.os().session_verdicts(s2.session).unwrap();
    assert_eq!(verdicts.tally.matched, 3);
}

/* ---- verdicts computed on read (D-2, D-6) ------------------------------ */

#[test]
fn a_scheme_turns_a_double_progression_load_step_into_progress() {
    let gym = TestGym::new();
    let scheme =
        ProgressionScheme::new(RepRange::new(reps(6), reps(8)).unwrap(), kg(2, 50)).unwrap();
    let with_scheme = gym.create_exercise("Bench Press", TrackingMode::WeightReps, Some(scheme));
    let without = gym.create_exercise("Incline Press", TrackingMode::WeightReps, None);
    for lift in [with_scheme, without] {
        one(
            &gym,
            &SessionPlan::new().exercise(lift, sets(kg(100, 0), &[8, 8, 8])),
        );
    }
    let today = gym.log_session(
        &SessionPlan::new()
            .exercise(with_scheme, sets(kg(102, 50), &[6, 6, 6]))
            .exercise(without, sets(kg(102, 50), &[6, 6, 6])),
    );
    let verdicts = gym.os().session_verdicts(today.session).unwrap();
    let [stepped, plain] = verdicts.entries.as_slice() else {
        panic!("expected two entries");
    };
    let (EntryOutcome::Verdict(stepped), EntryOutcome::Verdict(plain)) =
        (&stepped.outcome, &plain.outcome)
    else {
        panic!("expected two verdicts");
    };
    /* Golden B and B′: the same lifts, progressed with a scheme and
     * regressed without one. */
    assert_eq!(stepped.verdict, Verdict::Progressed);
    assert!(stepped.signals.contains(&Signal::LoadStep {
        before: kg(100, 0),
        after: kg(102, 50),
        reps: reps(6),
    }));
    assert_eq!(plain.verdict, Verdict::Regressed);
    assert_eq!(
        (verdicts.tally.progressed, verdicts.tally.regressed),
        (1, 1)
    );
}

#[test]
fn toggling_a_deload_changes_verdicts_at_once() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let (s1, _) = one(
        &gym,
        &SessionPlan::new().exercise(bench, sets(kg(100, 0), &[8, 8, 8])),
    );
    let (s2, _) = one(
        &gym,
        &SessionPlan::new().exercise(bench, sets(kg(80, 0), &[8, 8, 8])),
    );
    let (s3, s3_entry) = one(
        &gym,
        &SessionPlan::new().exercise(bench, sets(kg(90, 0), &[8, 8, 8])),
    );
    let verdict = |s| only_verdict(&gym.os().session_verdicts(s).unwrap()).verdict;

    assert_eq!(
        (verdict(s2), verdict(s3)),
        (Verdict::Regressed, Verdict::Progressed)
    );
    gym.set_deload(s2, true);
    assert_eq!(
        (verdict(s2), verdict(s3)),
        (Verdict::Deload, Verdict::Regressed)
    );
    assert_eq!(baseline_session(&gym, s3, s3_entry), Some(s1));
    gym.set_deload(s2, false);
    assert_eq!(
        (verdict(s2), verdict(s3)),
        (Verdict::Regressed, Verdict::Progressed)
    );
}

#[test]
fn unsupported_modes_get_their_own_outcome_without_failing_the_session() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let plank = gym.create_exercise("Plank", TrackingMode::Duration, None);
    let hold = SetBuilder::new(SetMetrics::Duration {
        duration: DurationSecs::new(60).unwrap(),
        load: None,
    });
    for n in [8, 9] {
        gym.log_session(
            &SessionPlan::new()
                .exercise(plank, [hold.clone()])
                .exercise(bench, sets(kg(100, 0), &[n])),
        );
    }
    let last = gym.log_session(
        &SessionPlan::new()
            .exercise(plank, [hold])
            .exercise(bench, sets(kg(100, 0), &[10])),
    );
    let verdicts = gym.os().session_verdicts(last.session).unwrap();
    assert_eq!(
        verdicts.entries[0].outcome,
        EntryOutcome::Unsupported(TrackingMode::Duration)
    );
    let EntryOutcome::Verdict(bench_verdict) = &verdicts.entries[1].outcome else {
        panic!("bench should have a verdict");
    };
    assert_eq!(bench_verdict.verdict, Verdict::Progressed);
    assert_eq!((verdicts.tally.progressed, verdicts.tally.total()), (1, 1));
}

/* ---- errors ------------------------------------------------------------- */

#[test]
fn an_inconsistent_stored_mode_is_an_error_not_a_verdict() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    /* Seed a past session whose bench entry was captured as a timed hold,
     * which the engine itself would never write (D-4). */
    let as_hold = exercise(bench, "Bench Press", TrackingMode::Duration);
    let corrupt = SessionBuilder::new(SessionId::from_uuid(Uuid::from_u128(0xBAD)), at(0))
        .entry(
            SessionExerciseId::from_uuid(Uuid::from_u128(0xBAD)),
            &as_hold,
            [SetBuilder::new(SetMetrics::Duration {
                duration: DurationSecs::new(60).unwrap(),
                load: None,
            })],
        )
        .finished_at(at(1_000))
        .build(&gymos_testkit::SeqIds::with_prefix(0xBAD << 64));
    gym.store()
        .write(|tx| tx.save_session(&corrupt).map_err(EngineError::from))
        .unwrap();

    let (today, _) = one(
        &gym,
        &SessionPlan::new().exercise(bench, sets(kg(100, 0), &[8])),
    );
    assert_eq!(
        gym.os().session_verdicts(today),
        Err(EngineError::Domain(DomainError::Verdict(
            VerdictError::ModeMismatch {
                baseline: TrackingMode::Duration,
                current: TrackingMode::WeightReps,
            }
        )))
    );
}

#[test]
fn queries_report_missing_sessions_and_entries() {
    let gym = TestGym::new();
    let bench = bench(&gym);
    let (session, entry) = one(
        &gym,
        &SessionPlan::new().exercise(bench, sets(kg(100, 0), &[8])),
    );
    let nowhere = SessionId::from_uuid(Uuid::from_u128(404));
    let no_entry = SessionExerciseId::from_uuid(Uuid::from_u128(404));
    assert_eq!(
        gym.os().session_verdicts(nowhere),
        Err(EngineError::NotFound(Missing::Session(nowhere)))
    );
    assert_eq!(
        gym.os().last_time(nowhere, entry),
        Err(EngineError::NotFound(Missing::Session(nowhere)))
    );
    assert_eq!(
        gym.os().last_time(session, no_entry),
        Err(EngineError::NotFound(Missing::Entry {
            session,
            entry: no_entry
        }))
    );
}
