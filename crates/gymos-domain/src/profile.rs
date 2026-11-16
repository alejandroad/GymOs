/*!
 * The local profile: who the user is and how they like numbers shown.
 *
 * There is one profile per device and no account (product D7). The profile
 * deliberately holds **no stats**. Body weight, VO2 max and the rest are
 * measurement time series, and records are derived data. A stats page is a
 * *view* the engine builds from all three, so nothing here can go stale.
 */

use thiserror::Error;

use crate::text::Name;
use crate::units::WeightUnit;

/** Why a profile value was rejected. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ProfileError {
    /** A birth year outside [`BirthYear::MIN`]..=[`BirthYear::MAX`]. */
    #[error("birth year must be between 1900 and 2100, got {0}")]
    BirthYearOutOfRange(u16),
}

/** The unit distances are shown in. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DistanceUnit {
    /** Kilometres and metres. */
    Kilometres,
    /** Miles and feet. */
    Miles,
}

/**
 * How values are displayed. Storage is unit-independent and exact, so
 * changing these never changes any stored number.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnitPreferences {
    /** Unit for loads and body weight. */
    pub weight: WeightUnit,
    /** Unit for distances. */
    pub distance: DistanceUnit,
}

impl Default for UnitPreferences {
    /** Pounds and miles by default (product D10 sets lbs as the default). */
    fn default() -> Self {
        Self {
            weight: WeightUnit::Pounds,
            distance: DistanceUnit::Miles,
        }
    }
}

/**
 * A year of birth, used only to choose an age-banded reference population
 * for percentiles later (product D8). A year, not a date, to keep less data.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BirthYear(u16);

impl BirthYear {
    /** The earliest year accepted. */
    pub const MIN: u16 = 1900;
    /** The latest year accepted. The domain has no clock to check "not in the future". */
    pub const MAX: u16 = 2100;

    /**
     * Builds a birth year.
     *
     * # Errors
     *
     * [`ProfileError::BirthYearOutOfRange`] outside 1900..=2100.
     */
    pub const fn new(year: u16) -> Result<Self, ProfileError> {
        if year < Self::MIN || year > Self::MAX {
            return Err(ProfileError::BirthYearOutOfRange(year));
        }
        Ok(Self(year))
    }

    /** The year. */
    pub const fn get(self) -> u16 {
        self.0
    }
}

/**
 * The sex of the published reference population to compare against.
 *
 * Strength and VO2 max norms are published separately for female and male
 * populations, and every percentile is labelled with its population
 * (product D8). This only selects which table to use. It is optional, local,
 * and nothing else reads it.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferencePopulation {
    /** Compare against published female norms. */
    Female,
    /** Compare against published male norms. */
    Male,
}

/**
 * The user's profile. Every field is optional except display preferences.
 *
 * All data stays on the device. Nothing here is required to use GymOs.
 */
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Profile {
    /** What to call the user, if they want a greeting. */
    pub display_name: Option<Name>,
    /** How numbers are shown. */
    pub units: UnitPreferences,
    /** For age-banded percentiles; optional. */
    pub birth_year: Option<BirthYear>,
    /** For population-specific percentiles; optional. */
    pub reference_population: Option<ReferencePopulation>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_profile_needs_nothing_and_shows_pounds() {
        let profile = Profile::default();
        assert_eq!(profile.display_name, None);
        assert_eq!(profile.units.weight, WeightUnit::Pounds);
        assert_eq!(profile.birth_year, None);
        assert_eq!(profile.reference_population, None);
    }

    #[test]
    fn birth_year_range() {
        assert_eq!(BirthYear::new(1990).unwrap().get(), 1990);
        assert_eq!(
            BirthYear::new(1899),
            Err(ProfileError::BirthYearOutOfRange(1899))
        );
        assert_eq!(
            BirthYear::new(2101),
            Err(ProfileError::BirthYearOutOfRange(2101))
        );
    }
}
