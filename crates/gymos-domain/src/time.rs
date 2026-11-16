/*!
 * Points in time.
 *
 * A [`Timestamp`] stores the UTC instant **and** the local offset the user
 * was in, so "Monday's session" stays Monday even after travelling. The
 * domain never reads the clock; timestamps arrive from the engine's `Clock`
 * port or from stored data.
 */

use thiserror::Error;

/** Why a time value was rejected. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TimeError {
    /** A UTC offset outside ±18 hours, the range real time zones use. */
    #[error("UTC offset must be within ±18 hours, got {0} minutes")]
    OffsetOutOfRange(i16),
}

/** A local offset from UTC, in whole minutes (for example `-360` for UTC−6). */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UtcOffset(i16);

impl UtcOffset {
    /** UTC itself. */
    pub const UTC: Self = Self(0);
    /** The most negative offset accepted: UTC−18:00. */
    pub const MIN_MINUTES: i16 = -18 * 60;
    /** The most positive offset accepted: UTC+18:00. */
    pub const MAX_MINUTES: i16 = 18 * 60;

    /**
     * Builds an offset from minutes east of UTC.
     *
     * # Errors
     *
     * [`TimeError::OffsetOutOfRange`] outside ±18 hours.
     */
    pub const fn from_minutes(minutes: i16) -> Result<Self, TimeError> {
        if minutes < Self::MIN_MINUTES || minutes > Self::MAX_MINUTES {
            return Err(TimeError::OffsetOutOfRange(minutes));
        }
        Ok(Self(minutes))
    }

    /** Minutes east of UTC. */
    pub const fn minutes(self) -> i16 {
        self.0
    }
}

/**
 * A UTC instant plus the local offset it was recorded in.
 *
 * Equality compares both parts. To order events in time, compare
 * [`Timestamp::unix_millis`], which ignores the offset.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Timestamp {
    unix_millis: i64,
    offset: UtcOffset,
}

impl Timestamp {
    /**
     * Builds a timestamp from milliseconds since the Unix epoch (UTC) and
     * the local offset in effect at that moment.
     */
    pub const fn new(unix_millis: i64, offset: UtcOffset) -> Self {
        Self {
            unix_millis,
            offset,
        }
    }

    /** Milliseconds since the Unix epoch, in UTC. */
    pub const fn unix_millis(self) -> i64 {
        self.unix_millis
    }

    /** The local offset the user was in. */
    pub const fn offset(self) -> UtcOffset {
        self.offset
    }

    /** True if `self` happened strictly before `other`, regardless of offset. */
    pub const fn is_before(self, other: Self) -> bool {
        self.unix_millis < other.unix_millis
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_offsets_within_eighteen_hours() {
        assert_eq!(UtcOffset::from_minutes(-360).unwrap().minutes(), -360);
        assert!(UtcOffset::from_minutes(UtcOffset::MAX_MINUTES).is_ok());
        assert!(UtcOffset::from_minutes(UtcOffset::MIN_MINUTES).is_ok());
    }

    #[test]
    fn rejects_offsets_beyond_eighteen_hours() {
        assert_eq!(
            UtcOffset::from_minutes(18 * 60 + 1),
            Err(TimeError::OffsetOutOfRange(1081))
        );
    }

    #[test]
    fn orders_by_instant_not_by_offset() {
        let earlier = Timestamp::new(1_000, UtcOffset::from_minutes(600).unwrap());
        let later = Timestamp::new(2_000, UtcOffset::from_minutes(-600).unwrap());
        assert!(earlier.is_before(later));
        assert!(!later.is_before(earlier));
    }
}
