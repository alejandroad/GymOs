/*!
 * Activities: runs, rides, swims, classes (product §3.6).
 *
 * Lifting is the core (product D5); endurance is tracked at summary level.
 * Unlike sets, activities share almost every field, so an [`Activity`] is
 * one struct with optional details. The rules that tie fields together
 * (moving time within elapsed time, distance only where it means something)
 * are checked whenever details are set, so the fields stay private.
 *
 * Indoor versus outdoor is a detail ([`Environment`]), not a separate kind,
 * so "treadmill run" is a `Run` that took place `Indoor`. A strength-training
 * activity (usually a watch summary) can link to the logged
 * [`Session`](crate::Session) instead of repeating its sets.
 */

use thiserror::Error;

use crate::effort::PerceivedEffort;
use crate::health::HeartRate;
use crate::ids::{ActivityId, SessionId};
use crate::provenance::Source;
use crate::text::{Name, Note};
use crate::time::Timestamp;
use crate::units::{Distance, ElapsedTime, ElevationGain, Energy};

/** Why an activity was rejected. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ActivityError {
    /** A distance on an activity kind that doesn't cover one. */
    #[error("{0:?} activities don't record a distance")]
    DistanceNotTracked(ActivityKind),
    /** Moving time longer than elapsed time. */
    #[error("moving time cannot exceed elapsed time")]
    MovingExceedsElapsed,
    /** A maximum heart rate below the average. */
    #[error("maximum heart rate cannot be below the average")]
    MaxHeartRateBelowAverage,
    /** A linked session on a kind other than strength training. */
    #[error("only strength-training activities can link to a session, not {0:?}")]
    SessionLinkNotAllowed(ActivityKind),
}

/** The type of activity. Decides which details make sense. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActivityKind {
    /* Endurance */
    /** Running: road, trail or treadmill. */
    Run,
    /** Cycling: road, gravel, mountain or indoor trainer. */
    Ride,
    /** Swimming: pool or open water. */
    Swim,
    /** Walking. */
    Walk,
    /** Hiking. */
    Hike,
    /** Rowing: on the water or an erg. */
    Row,
    /** Elliptical trainer. */
    Elliptical,
    /** Stair stepper or stair climbing. */
    StairStepper,
    /** Kayak, canoe or stand-up paddleboard. */
    Paddle,
    /** Alpine or cross-country skiing, and snowboarding. */
    Ski,
    /** Skating: inline, ice or skateboard. */
    Skate,

    /* Gym and classes */
    /**
     * Strength training recorded as a summary (often from a watch). The sets
     * themselves belong in a [`Session`](crate::Session); link it with
     * [`ActivityDetails::linked_session`].
     */
    StrengthTraining,
    /** High-intensity interval training and circuits. */
    Hiit,
    /** Pilates. */
    Pilates,
    /** Yoga and mobility. */
    Yoga,
    /** Dance. */
    Dance,

    /* Sports */
    /** Martial arts, boxing and similar. */
    Combat,
    /** Climbing and bouldering. */
    Climbing,
    /** Team sports: football, basketball, hockey and similar. */
    TeamSport,
    /** Racket sports: tennis, padel, pickleball, squash. */
    RacketSport,
    /** Golf. */
    Golf,

    /** Anything else. */
    Other,
}

impl ActivityKind {
    /** Whether this kind of activity covers a distance worth recording. */
    pub const fn tracks_distance(self) -> bool {
        match self {
            Self::Run
            | Self::Ride
            | Self::Swim
            | Self::Walk
            | Self::Hike
            | Self::Row
            | Self::Elliptical
            | Self::Paddle
            | Self::Ski
            | Self::Skate
            | Self::TeamSport
            | Self::Golf
            | Self::Other => true,
            Self::StairStepper
            | Self::StrengthTraining
            | Self::Hiit
            | Self::Pilates
            | Self::Yoga
            | Self::Dance
            | Self::Combat
            | Self::Climbing
            | Self::RacketSport => false,
        }
    }
}

/**
 * Where an activity took place.
 *
 * A detail rather than separate kinds, so every kind can be either without
 * doubling the enum. Indoor distances are often machine estimates (a
 * treadmill, a trainer, an erg), which records may treat differently later.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Environment {
    /** Treadmill, trainer, erg, pool, gym, studio. */
    Indoor,
    /** Road, trail, open water, field. */
    Outdoor,
}

/**
 * The optional details of an activity. Build one freely, then hand it to
 * [`Activity::new`] or [`Activity::set_details`], which check it.
 */
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActivityDetails {
    /** A title, such as "Morning long run". */
    pub title: Option<Name>,
    /** Indoor or outdoor, if known. */
    pub environment: Option<Environment>,
    /**
     * The logged workout this activity summarizes. Only for
     * [`ActivityKind::StrengthTraining`], so a watch's heart rate and energy
     * attach to the session without recording the sets twice.
     */
    pub linked_session: Option<SessionId>,
    /** Time spent moving, excluding stops. At most the elapsed time. */
    pub moving: Option<ElapsedTime>,
    /** Distance covered. Only for kinds that track distance. */
    pub distance: Option<Distance>,
    /** Total climb. */
    pub elevation_gain: Option<ElevationGain>,
    /** Average heart rate. */
    pub average_heart_rate: Option<HeartRate>,
    /** Maximum heart rate. At least the average. */
    pub max_heart_rate: Option<HeartRate>,
    /** Energy expended. */
    pub energy: Option<Energy>,
    /** How hard it felt, 1–10. */
    pub effort: Option<PerceivedEffort>,
    /** A free-text note. */
    pub note: Option<Note>,
}

/** One activity: a kind, when it happened, how long it took, and details. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Activity {
    id: ActivityId,
    kind: ActivityKind,
    started_at: Timestamp,
    elapsed: ElapsedTime,
    details: ActivityDetails,
    source: Source,
}

impl Activity {
    /**
     * Builds an activity.
     *
     * # Errors
     *
     * [`ActivityError`] if the details break a rule for this kind and time.
     */
    pub fn new(
        id: ActivityId,
        kind: ActivityKind,
        started_at: Timestamp,
        elapsed: ElapsedTime,
        details: ActivityDetails,
        source: Source,
    ) -> Result<Self, ActivityError> {
        check(kind, elapsed, &details)?;
        Ok(Self {
            id,
            kind,
            started_at,
            elapsed,
            details,
            source,
        })
    }

    /**
     * Replaces the details, for example after an edit.
     *
     * # Errors
     *
     * [`ActivityError`] if the new details break a rule; the activity is
     * left unchanged.
     */
    pub fn set_details(&mut self, details: ActivityDetails) -> Result<(), ActivityError> {
        check(self.kind, self.elapsed, &details)?;
        self.details = details;
        Ok(())
    }

    /** Stable identity. */
    pub const fn id(&self) -> ActivityId {
        self.id
    }

    /** The type of activity. */
    pub const fn kind(&self) -> ActivityKind {
        self.kind
    }

    /** When it started. */
    pub const fn started_at(&self) -> Timestamp {
        self.started_at
    }

    /** Total time from start to finish, including stops. */
    pub const fn elapsed(&self) -> ElapsedTime {
        self.elapsed
    }

    /** The optional details. */
    pub const fn details(&self) -> &ActivityDetails {
        &self.details
    }

    /** Typed in, or imported. */
    pub const fn source(&self) -> Source {
        self.source
    }
}

fn check(
    kind: ActivityKind,
    elapsed: ElapsedTime,
    details: &ActivityDetails,
) -> Result<(), ActivityError> {
    if details.distance.is_some() && !kind.tracks_distance() {
        return Err(ActivityError::DistanceNotTracked(kind));
    }
    if details.linked_session.is_some() && kind != ActivityKind::StrengthTraining {
        return Err(ActivityError::SessionLinkNotAllowed(kind));
    }
    if details.moving.is_some_and(|moving| moving > elapsed) {
        return Err(ActivityError::MovingExceedsElapsed);
    }
    if let (Some(average), Some(max)) = (details.average_heart_rate, details.max_heart_rate)
        && max < average
    {
        return Err(ActivityError::MaxHeartRateBelowAverage);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;
    use crate::time::UtcOffset;

    fn activity(kind: ActivityKind, details: ActivityDetails) -> Result<Activity, ActivityError> {
        Activity::new(
            ActivityId::from_uuid(Uuid::from_u128(1)),
            kind,
            Timestamp::new(0, UtcOffset::UTC),
            ElapsedTime::new(1_800).unwrap(),
            details,
            Source::Manual,
        )
    }

    #[test]
    fn a_run_records_distance() {
        let details = ActivityDetails {
            distance: Some(Distance::from_millimetres(5_000_000).unwrap()),
            ..ActivityDetails::default()
        };
        let run = activity(ActivityKind::Run, details).unwrap();
        assert_eq!(run.details().distance.unwrap().millimetres(), 5_000_000);
    }

    #[test]
    fn yoga_cannot_record_distance() {
        let details = ActivityDetails {
            distance: Some(Distance::from_millimetres(1_000).unwrap()),
            ..ActivityDetails::default()
        };
        assert_eq!(
            activity(ActivityKind::Yoga, details),
            Err(ActivityError::DistanceNotTracked(ActivityKind::Yoga))
        );
    }

    #[test]
    fn moving_time_fits_within_elapsed_time() {
        let details = ActivityDetails {
            moving: Some(ElapsedTime::new(1_801).unwrap()),
            ..ActivityDetails::default()
        };
        assert_eq!(
            activity(ActivityKind::Run, details),
            Err(ActivityError::MovingExceedsElapsed)
        );
    }

    #[test]
    fn max_heart_rate_is_at_least_the_average() {
        let details = ActivityDetails {
            average_heart_rate: Some(HeartRate::from_bpm(150).unwrap()),
            max_heart_rate: Some(HeartRate::from_bpm(140).unwrap()),
            ..ActivityDetails::default()
        };
        assert_eq!(
            activity(ActivityKind::Ride, details),
            Err(ActivityError::MaxHeartRateBelowAverage)
        );
    }

    #[test]
    fn a_treadmill_run_is_an_indoor_run_with_distance() {
        let details = ActivityDetails {
            environment: Some(Environment::Indoor),
            distance: Some(Distance::from_millimetres(5_000_000).unwrap()),
            ..ActivityDetails::default()
        };
        let run = activity(ActivityKind::Run, details).unwrap();
        assert_eq!(run.details().environment, Some(Environment::Indoor));
    }

    #[test]
    fn strength_training_links_to_its_session() {
        let session = SessionId::from_uuid(Uuid::from_u128(7));
        let details = ActivityDetails {
            linked_session: Some(session),
            average_heart_rate: Some(HeartRate::from_bpm(118).unwrap()),
            ..ActivityDetails::default()
        };
        let lift = activity(ActivityKind::StrengthTraining, details).unwrap();
        assert_eq!(lift.details().linked_session, Some(session));
    }

    #[test]
    fn only_strength_training_can_link_a_session() {
        let details = ActivityDetails {
            linked_session: Some(SessionId::from_uuid(Uuid::from_u128(7))),
            ..ActivityDetails::default()
        };
        assert_eq!(
            activity(ActivityKind::Run, details),
            Err(ActivityError::SessionLinkNotAllowed(ActivityKind::Run))
        );
    }

    #[test]
    fn strength_training_has_no_distance() {
        assert!(!ActivityKind::StrengthTraining.tracks_distance());
        assert!(ActivityKind::Paddle.tracks_distance());
    }

    #[test]
    fn rejected_edits_leave_the_activity_unchanged() {
        let mut run = activity(ActivityKind::Run, ActivityDetails::default()).unwrap();
        let bad = ActivityDetails {
            moving: Some(ElapsedTime::new(9_999).unwrap()),
            ..ActivityDetails::default()
        };
        assert!(run.set_details(bad).is_err());
        assert_eq!(run.details(), &ActivityDetails::default());
    }
}
