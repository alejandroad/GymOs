/*!
 * `proptest` strategies for valid domain values (S-11, kernel-architecture
 * §6.5).
 *
 * Every strategy builds its values through the domain's own constructors,
 * so it can only produce what the domain accepts. Use them to state
 * properties over "any valid session", such as the store's round trip.
 */

#![expect(
    clippy::expect_used,
    reason = "assembling a session from generated parts can't fail; if it does, the strategy is wrong"
)]

use gymos_domain::progression::{ProgressionScheme, RepRange};
use gymos_domain::{
    Distance, DurationSecs, Effort, Exercise, ExerciseId, ImportFormat, Load, LoggedSet, Name,
    Note, Reps, Rir, Rpe, Session, SessionExercise, SessionExerciseId, SessionId, SetId,
    SetMetrics, SetType, Source, TemplateId, Timestamp, TrackingMode, UtcOffset, Uuid, WeightUnit,
};
use proptest::prelude::*;
use proptest::sample::select;

/** Any tracking mode. */
pub fn tracking_mode() -> impl Strategy<Value = TrackingMode> {
    select(vec![
        TrackingMode::WeightReps,
        TrackingMode::BodyweightReps,
        TrackingMode::WeightedBodyweight,
        TrackingMode::AssistedBodyweight,
        TrackingMode::Duration,
        TrackingMode::DistanceLoad,
        TrackingMode::RepsOnly,
        TrackingMode::TimeReps,
    ])
}

/** Any set type. */
pub fn set_type() -> impl Strategy<Value = SetType> {
    select(vec![
        SetType::Warmup,
        SetType::Working,
        SetType::Drop,
        SetType::Failure,
        SetType::TopSet,
        SetType::Backoff,
    ])
}

/** Any valid load, from 0 up to [`Load::MAX`], in either unit. */
pub fn load() -> impl Strategy<Value = Load> {
    /* The largest entries under 1000 kg: 100 000 hundredths of a kg, and
     * 220 462 hundredths of a pound (2204.62 lb = 999.998 kg). */
    prop_oneof![
        (0_u32..=100_000).prop_map(|h| (h, WeightUnit::Kilograms)),
        (0_u32..=220_462).prop_map(|h| (h, WeightUnit::Pounds)),
    ]
    .prop_filter_map("within Load::MAX", |(h, unit)| Load::new(h, unit).ok())
}

/** Any valid rep count. */
pub fn reps() -> impl Strategy<Value = Reps> {
    (Reps::MIN..=Reps::MAX).prop_filter_map("valid reps", |n| Reps::new(n).ok())
}

/** Any valid set duration. */
pub fn duration() -> impl Strategy<Value = DurationSecs> {
    (DurationSecs::MIN..=DurationSecs::MAX)
        .prop_filter_map("valid duration", |s| DurationSecs::new(s).ok())
}

/** Any valid distance. */
pub fn distance() -> impl Strategy<Value = Distance> {
    (Distance::MIN_MM..=Distance::MAX_MM)
        .prop_filter_map("valid distance", |mm| Distance::from_millimetres(mm).ok())
}

/** An RPE (1.0–10.0 in half points) or an RIR (0–10). */
pub fn effort() -> impl Strategy<Value = Effort> {
    prop_oneof![
        (Rpe::MIN_HALF_POINTS..=Rpe::MAX_HALF_POINTS).prop_filter_map("valid RPE", |h| {
            Rpe::from_half_points(h).ok().map(Effort::Rpe)
        }),
        (0..=Rir::MAX).prop_filter_map("valid RIR", |r| Rir::new(r).ok().map(Effort::Rir)),
    ]
}

/** Any measurements valid for `mode`. */
pub fn set_metrics(mode: TrackingMode) -> BoxedStrategy<SetMetrics> {
    match mode {
        TrackingMode::WeightReps => (load(), reps())
            .prop_map(|(load, reps)| SetMetrics::WeightReps { load, reps })
            .boxed(),
        TrackingMode::BodyweightReps => reps()
            .prop_map(|reps| SetMetrics::BodyweightReps { reps })
            .boxed(),
        TrackingMode::WeightedBodyweight => (load(), reps())
            .prop_map(|(added, reps)| SetMetrics::WeightedBodyweight { added, reps })
            .boxed(),
        TrackingMode::AssistedBodyweight => (load(), reps())
            .prop_map(|(assistance, reps)| SetMetrics::AssistedBodyweight { assistance, reps })
            .boxed(),
        TrackingMode::Duration => (duration(), proptest::option::of(load()))
            .prop_map(|(duration, load)| SetMetrics::Duration { duration, load })
            .boxed(),
        TrackingMode::DistanceLoad => (distance(), load(), proptest::option::of(duration()))
            .prop_map(|(distance, load, duration)| SetMetrics::DistanceLoad {
                distance,
                load,
                duration,
            })
            .boxed(),
        TrackingMode::RepsOnly => reps()
            .prop_map(|reps| SetMetrics::RepsOnly { reps })
            .boxed(),
        TrackingMode::TimeReps => (reps(), duration())
            .prop_map(|(reps, duration)| SetMetrics::TimeReps { reps, duration })
            .boxed(),
    }
}

/**
 * A short name: plain, accented, emoji and padded text, as `Name::new`
 * stores it (trimmed).
 */
pub fn name() -> impl Strategy<Value = Name> {
    select(vec![
        "Bench Press",
        "Día de pierna",
        "Überkopfdrücken",
        "深蹲",
        "PR day 💪",
        "  padded  ",
        "x",
    ])
    .prop_filter_map("valid name", |s| Name::new(s).ok())
}

/** A note, including multi-line, tabbed and Windows line endings. */
pub fn note() -> impl Strategy<Value = Note> {
    select(vec![
        "felt strong",
        "felt strong\nslept well",
        "cue:\tbrace",
        "from Windows\r\nline two",
        "ünïcödé — notes",
        " trailing space ",
    ])
    .prop_filter_map("valid note", |s| Note::new(s).ok())
}

/** Any provenance. */
pub fn source() -> impl Strategy<Value = Source> {
    prop_oneof![
        Just(Source::Manual),
        (
            select(vec![
                ImportFormat::Strong,
                ImportFormat::Hevy,
                ImportFormat::GarminActivities,
                ImportFormat::GarminVo2Max,
            ]),
            any::<u16>(),
        )
            .prop_map(|(format, importer_version)| Source::Import {
                format,
                importer_version
            }),
    ]
}

/** A timestamp within ±100 years of 1970, at any valid offset. */
pub fn timestamp() -> impl Strategy<Value = Timestamp> {
    const CENTURY_MS: i64 = 100 * 366 * 86_400_000;
    (
        -CENTURY_MS..=CENTURY_MS,
        UtcOffset::MIN_MINUTES..=UtcOffset::MAX_MINUTES,
    )
        .prop_filter_map("valid offset", |(ms, offset)| {
            UtcOffset::from_minutes(offset)
                .ok()
                .map(|o| Timestamp::new(ms, o))
        })
}

/** Any valid progression scheme. */
pub fn scheme() -> impl Strategy<Value = ProgressionScheme> {
    (reps(), reps(), load()).prop_filter_map("valid scheme", |(a, b, increment)| {
        let (min, max) = if a <= b { (a, b) } else { (b, a) };
        ProgressionScheme::new(RepRange::new(min, max).ok()?, increment).ok()
    })
}

/** One set's parts, before it gets an ID. */
type SetParts = (SetType, SetMetrics, Option<Effort>, Option<Note>);

fn set_parts(mode: TrackingMode) -> impl Strategy<Value = SetParts> {
    (
        set_type(),
        set_metrics(mode),
        proptest::option::of(effort()),
        proptest::option::of(note()),
    )
}

/** An entry's parts: which exercise of the pool, a note, and 0–6 sets. */
type EntryParts = (usize, Option<Note>, Vec<SetParts>);

fn entry_parts(pool: Vec<TrackingMode>) -> impl Strategy<Value = EntryParts> {
    (0..pool.len()).prop_flat_map(move |index| {
        let mode = *pool.get(index).expect("index drawn from the pool");
        (
            Just(index),
            proptest::option::of(note()),
            proptest::collection::vec(set_parts(mode), 0..=6),
        )
    })
}

/** A session's own fields, before entries. */
#[derive(Debug, Clone)]
struct Header {
    started_at: Timestamp,
    ended_after_ms: Option<(i64, Timestamp)>,
    title: Option<Name>,
    note: Option<Note>,
    template: Option<u128>,
    deload: bool,
    source: Source,
}

fn header() -> impl Strategy<Value = Header> {
    (
        timestamp(),
        proptest::option::of((0_i64..=86_400_000, timestamp())),
        proptest::option::of(name()),
        proptest::option::of(note()),
        proptest::option::of(1_u128..=3),
        any::<bool>(),
        source(),
    )
        .prop_map(
            |(started_at, ended_after_ms, title, note, template, deload, source)| Header {
                started_at,
                ended_after_ms,
                title,
                note,
                template,
                deload,
                source,
            },
        )
}

/**
 * A valid session with 0–5 entries of 0–6 sets each, together with the
 * exercises its entries name (a pool of one to three, each in its own
 * mode, with or without a scheme).
 *
 * IDs are derived from positions: the session is
 * `00000000-0000-0000-0000-000000000001`, so two values from this strategy
 * describe the same session and saving the second replaces the first.
 */
pub fn session_with_exercises() -> impl Strategy<Value = (Vec<Exercise>, Session)> {
    proptest::collection::vec(
        (tracking_mode(), proptest::option::of(scheme()), name()),
        1..=3,
    )
    .prop_flat_map(|pool| {
        let modes: Vec<TrackingMode> = pool.iter().map(|(mode, _, _)| *mode).collect();
        (
            Just(pool),
            header(),
            proptest::collection::vec(entry_parts(modes), 0..=5),
        )
    })
    .prop_map(|(pool, header, entries)| build(&pool, header, entries))
}

fn id(prefix: u128, n: usize) -> Uuid {
    Uuid::from_u128(prefix << 64 | n as u128)
}

fn build(
    pool: &[(TrackingMode, Option<ProgressionScheme>, Name)],
    header: Header,
    entries: Vec<EntryParts>,
) -> (Vec<Exercise>, Session) {
    let exercises: Vec<Exercise> = pool
        .iter()
        .enumerate()
        .map(|(k, (mode, scheme, name))| Exercise {
            id: ExerciseId::from_uuid(id(0xEE, k)),
            name: name.clone(),
            mode: *mode,
            scheme: *scheme,
        })
        .collect();
    let mut session = Session::start(
        SessionId::from_uuid(Uuid::from_u128(1)),
        header.started_at,
        header.source,
    );
    session.set_title(header.title);
    session.set_note(header.note);
    session.set_template(
        header
            .template
            .map(|t| TemplateId::from_uuid(Uuid::from_u128(t))),
    );
    session.set_deload(header.deload);
    for (i, (index, note, sets)) in entries.into_iter().enumerate() {
        let exercise = exercises.get(index).expect("index drawn from the pool");
        let mut entry = SessionExercise::new(SessionExerciseId::from_uuid(id(0xE, i)), exercise);
        entry.set_note(note);
        for (j, (set_type, metrics, effort, note)) in sets.into_iter().enumerate() {
            let set = LoggedSet {
                id: SetId::from_uuid(id(0x5E7, i * 16 + j)),
                set_type,
                metrics,
                effort,
                note,
            };
            entry
                .push_set(set)
                .expect("metrics were generated for this entry's mode");
        }
        session
            .add_exercise(entry)
            .expect("entry IDs are distinct positions");
    }
    if let Some((after_ms, at)) = header.ended_after_ms {
        let ended = Timestamp::new(header.started_at.unix_millis() + after_ms, at.offset());
        session
            .finish(ended)
            .expect("the end is at or after the start");
    }
    (exercises, session)
}
