/*!
 * Typed identifiers.
 *
 * Each entity gets its own ID type, so a `SetId` can never be passed where a
 * `SessionId` is expected. IDs are UUID v7 values generated outside the domain
 * (by the engine's `IdGen` port) and only wrapped here.
 */

use core::fmt;

use uuid::Uuid;

macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Uuid);

        impl $name {
            /** Wraps an existing UUID. The domain never generates IDs itself. */
            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /** The underlying UUID. */
            pub const fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }
    };
}

id_type! {
    /** Identifies a workout session. */
    SessionId
}

id_type! {
    /**
     * Identifies one exercise entry within a session. The same exercise may
     * appear twice in a session, so commands name the entry, not the
     * exercise (DD-15). It is also the stable key sync needs (DD-04).
     */
    SessionExerciseId
}

id_type! {
    /** Identifies one logged set. */
    SetId
}

id_type! {
    /** Identifies a catalog or custom exercise. */
    ExerciseId
}

id_type! {
    /** Identifies a workout template (for example "Push A"). */
    TemplateId
}

id_type! {
    /** Identifies one measurement (a body-weight entry, a VO2 max reading). */
    MeasurementId
}

id_type! {
    /** Identifies one activity (a run, a ride, a yoga class). */
    ActivityId
}

id_type! {
    /** Identifies a user-defined custom metric (for example "Grip strength"). */
    MetricId
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_the_wrapped_uuid() {
        let uuid = Uuid::from_u128(0x0190_0000_0000_7000_8000_0000_0000_0001);
        assert_eq!(SetId::from_uuid(uuid).as_uuid(), uuid);
    }

    #[test]
    fn displays_as_hyphenated_uuid() {
        let id = SessionId::from_uuid(Uuid::from_u128(1));
        assert_eq!(id.to_string(), "00000000-0000-0000-0000-000000000001");
    }
}
