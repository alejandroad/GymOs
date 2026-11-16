/*!
 * Provenance: where a piece of data came from (data-model rule 4).
 *
 * Every session, measurement and activity records whether the user typed it
 * or an importer produced it, and which importer version. That makes an
 * import auditable and lets a buggy importer's rows be found and re-imported.
 */

/** A file format GymOs can import (product D12; the importers come later). */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ImportFormat {
    /** Strong app CSV export. */
    Strong,
    /** Hevy app CSV export. */
    Hevy,
    /** Garmin Connect activities CSV export. */
    GarminActivities,
    /** Garmin Connect VO2 max report CSV. */
    GarminVo2Max,
}

/** Where a record came from. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    /** Entered by the user in GymOs. */
    Manual,
    /** Produced by an importer. */
    Import {
        /** The file format that was read. */
        format: ImportFormat,
        /** The importer's own version, bumped whenever its mapping changes. */
        importer_version: u16,
    },
}
