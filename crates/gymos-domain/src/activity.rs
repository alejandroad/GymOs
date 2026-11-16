/*!
 * Activities: runs, rides, swims, classes (product §3.6).
 *
 * Lifting is the core (product D5); endurance is tracked at summary level.
 * Unlike sets, activities share almost every field, so an [`Activity`] is
 * one struct with optional details. The rules that tie fields together
 * (moving time within elapsed time, distance only where it means something)
 * are checked whenever details are set, so the fields stay private.
 */

use thiserror::Error;

use crate::effort::PerceivedEffort;
use crate::health::HeartRate;
use crate::ids::ActivityId;
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
}

/** The type of activity. Decides which details make sense. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActivityKind {
    /** Running. */
    Run,
    /** Cycling. */
    Ride,
    /** Swimming. */
    Swim,
    /** Walking. */
    Walk,
    /** Hiking. */
    Hike,
    /** Rowing. */
    Row,
    /** Martial arts, boxing and similar. */
    Combat,
    /** Climbing and bouldering. */
    Climbing,
    /** Yoga and mobility. */
    Yoga,
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
            | Self::Other => true,
            Self::Combat | Self::Climbing | Self::Yoga => false,
        }
    }
}

/**
 * The optional details of an activity. Build one freely, then hand it to
 * [`Activity::new`] or [`Activity::set_details`], which check it.
 */
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActivityDetails {
    /** A title, such as "Morning long run". */
    pub title: Option<Name>,
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
