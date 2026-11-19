/*!
 * Running and other activities: what a valid activity is (DD-14).
 *
 * Activities carry no overload verdict; their progress shows up as records
 * (fastest 5 km, longest run, biggest week), covered in `records.rs`. These
 * tests lock in the rules that keep a logged run honest, so every record
 * built from runs later starts from valid data.
 */

#![expect(
    clippy::unwrap_used,
    reason = "test helpers build fixtures from known-valid literals; a panic is a failed test"
)]

use gymos_domain::{
    Activity, ActivityDetails, ActivityError, ActivityId, ActivityKind, Distance, ElapsedTime,
    ElevationGain, Energy, Environment, HeartRate, ImportFormat, Name, PerceivedEffort, SessionId,
    Source, Timestamp, UtcOffset, Uuid,
};

fn run_for(seconds: u32, details: ActivityDetails) -> Result<Activity, ActivityError> {
    activity(ActivityKind::Run, seconds, details)
}

fn activity(
    kind: ActivityKind,
    seconds: u32,
    details: ActivityDetails,
) -> Result<Activity, ActivityError> {
    Activity::new(
        ActivityId::from_uuid(Uuid::from_u128(1)),
        kind,
        Timestamp::new(1_767_600_000_000, UtcOffset::from_minutes(-360).unwrap()),
        ElapsedTime::new(seconds).unwrap(),
        details,
        Source::Manual,
    )
}

fn metres(m: u32) -> Distance {
    Distance::from_millimetres(m * 1_000).unwrap()
}

fn secs(s: u32) -> ElapsedTime {
    ElapsedTime::new(s).unwrap()
}

fn bpm(n: u8) -> HeartRate {
    HeartRate::from_bpm(n).unwrap()
}

#[test]
fn a_fully_detailed_outdoor_run_is_accepted() {
    let details = ActivityDetails {
        title: Some(Name::new("Sunday long run").unwrap()),
        environment: Some(Environment::Outdoor),
        moving: Some(secs(5_880)),
        distance: Some(metres(21_098)),
        elevation_gain: Some(ElevationGain::from_metres(184).unwrap()),
        average_heart_rate: Some(bpm(151)),
        max_heart_rate: Some(bpm(176)),
        energy: Some(Energy::from_kcal(1_420).unwrap()),
        effort: Some(PerceivedEffort::new(7).unwrap()),
        ..ActivityDetails::default()
    };
    let run = run_for(6_000, details.clone()).unwrap();
    assert_eq!(run.kind(), ActivityKind::Run);
    assert_eq!(run.elapsed(), ElapsedTime::new(6_000).unwrap());
    assert_eq!(run.details(), &details);
}

#[test]
fn a_treadmill_run_is_an_indoor_run_that_still_has_a_distance() {
    let details = ActivityDetails {
        environment: Some(Environment::Indoor),
        distance: Some(metres(5_000)),
        ..ActivityDetails::default()
    };
    let run = run_for(1_500, details).unwrap();
    assert_eq!(run.details().environment, Some(Environment::Indoor));
    assert_eq!(run.details().distance, Some(metres(5_000)));
}

#[test]
fn a_run_with_only_a_time_is_accepted() {
    let run = run_for(1_800, ActivityDetails::default()).unwrap();
    assert_eq!(run.details().distance, None);
}

#[test]
fn moving_time_may_equal_but_never_exceed_elapsed_time() {
    let at = |moving| ActivityDetails {
        moving: Some(secs(moving)),
        ..ActivityDetails::default()
    };
    assert!(run_for(1_800, at(1_800)).is_ok());
    assert!(run_for(1_800, at(1_620)).is_ok());
    assert_eq!(
        run_for(1_800, at(1_801)),
        Err(ActivityError::MovingExceedsElapsed)
    );
}

#[test]
fn max_heart_rate_may_equal_but_never_be_below_the_average() {
    let hr = |average, max| ActivityDetails {
        average_heart_rate: Some(bpm(average)),
        max_heart_rate: Some(bpm(max)),
        ..ActivityDetails::default()
    };
    assert!(run_for(1_800, hr(150, 150)).is_ok());
    assert!(run_for(1_800, hr(150, 182)).is_ok());
    assert_eq!(
        run_for(1_800, hr(150, 149)),
        Err(ActivityError::MaxHeartRateBelowAverage)
    );
    /* Either one alone is fine. */
    assert!(
        run_for(
            1_800,
            ActivityDetails {
                max_heart_rate: Some(bpm(190)),
                ..ActivityDetails::default()
            }
        )
        .is_ok()
    );
}

#[test]
fn an_ultra_run_may_last_days_but_not_more_than_a_week() {
    let thirty_hours = 30 * 3_600;
    assert!(
        run_for(
            thirty_hours,
            ActivityDetails {
                distance: Some(metres(160_934)),
                ..ActivityDetails::default()
            }
        )
        .is_ok()
    );
    assert!(ElapsedTime::new(ElapsedTime::MAX).is_ok());
    assert!(ElapsedTime::new(ElapsedTime::MAX + 1).is_err());
    assert!(ElapsedTime::new(0).is_err());
}

#[test]
fn a_run_cannot_link_to_a_strength_session() {
    let details = ActivityDetails {
        linked_session: Some(SessionId::from_uuid(Uuid::from_u128(2))),
        ..ActivityDetails::default()
    };
    assert_eq!(
        run_for(1_800, details),
        Err(ActivityError::SessionLinkNotAllowed(ActivityKind::Run))
    );
}

#[test]
fn correcting_a_run_keeps_it_valid_or_leaves_it_unchanged() {
    let mut run = run_for(
        1_800,
        ActivityDetails {
            distance: Some(metres(5_000)),
            ..ActivityDetails::default()
        },
    )
    .unwrap();
    /* The watch said 5.0 km; the course was 5.1 km. */
    let corrected = ActivityDetails {
        distance: Some(metres(5_100)),
        ..ActivityDetails::default()
    };
    run.set_details(corrected.clone()).unwrap();
    assert_eq!(run.details(), &corrected);

    let broken = ActivityDetails {
        moving: Some(secs(1_900)),
        ..corrected.clone()
    };
    assert_eq!(
        run.set_details(broken),
        Err(ActivityError::MovingExceedsElapsed)
    );
    assert_eq!(run.details(), &corrected);
}

#[test]
fn an_imported_run_keeps_its_provenance() {
    let source = Source::Import {
        format: ImportFormat::GarminActivities,
        importer_version: 2,
    };
    let run = Activity::new(
        ActivityId::from_uuid(Uuid::from_u128(3)),
        ActivityKind::Run,
        Timestamp::new(0, UtcOffset::UTC),
        ElapsedTime::new(2_700).unwrap(),
        ActivityDetails {
            distance: Some(metres(10_000)),
            ..ActivityDetails::default()
        },
        source,
    )
    .unwrap();
    assert_eq!(run.source(), source);
}

/**
 * Which kinds cover a distance, written from DD-14. The match is
 * exhaustive, so a new activity kind fails to compile here until someone
 * decides whether it records a distance.
 */
const fn covers_distance(kind: ActivityKind) -> bool {
    match kind {
        ActivityKind::Run
        | ActivityKind::Ride
        | ActivityKind::Swim
        | ActivityKind::Walk
        | ActivityKind::Hike
        | ActivityKind::Row
        | ActivityKind::Elliptical
        | ActivityKind::Paddle
        | ActivityKind::Ski
        | ActivityKind::Skate
        | ActivityKind::TeamSport
        | ActivityKind::Golf
        | ActivityKind::Other => true,
        ActivityKind::StairStepper
        | ActivityKind::StrengthTraining
        | ActivityKind::Hiit
        | ActivityKind::Pilates
        | ActivityKind::Yoga
        | ActivityKind::Dance
        | ActivityKind::Combat
        | ActivityKind::Climbing
        | ActivityKind::RacketSport => false,
    }
}

#[test]
fn every_kind_accepts_a_distance_exactly_when_it_covers_one() {
    let kinds = [
        ActivityKind::Run,
        ActivityKind::Ride,
        ActivityKind::Swim,
        ActivityKind::Walk,
        ActivityKind::Hike,
        ActivityKind::Row,
        ActivityKind::Elliptical,
        ActivityKind::StairStepper,
        ActivityKind::Paddle,
        ActivityKind::Ski,
        ActivityKind::Skate,
        ActivityKind::StrengthTraining,
        ActivityKind::Hiit,
        ActivityKind::Pilates,
        ActivityKind::Yoga,
        ActivityKind::Dance,
        ActivityKind::Combat,
        ActivityKind::Climbing,
        ActivityKind::RacketSport,
        ActivityKind::TeamSport,
        ActivityKind::Golf,
        ActivityKind::Other,
    ];
    for kind in kinds {
        assert_eq!(kind.tracks_distance(), covers_distance(kind), "{kind:?}");
        let with_distance = ActivityDetails {
            distance: Some(metres(1_000)),
            ..ActivityDetails::default()
        };
        let result = activity(kind, 1_800, with_distance);
        if covers_distance(kind) {
            assert!(result.is_ok(), "{kind:?}");
        } else {
            assert_eq!(result, Err(ActivityError::DistanceNotTracked(kind)));
        }
    }
}
