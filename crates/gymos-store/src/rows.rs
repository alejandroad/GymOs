/*!
 * Row mappers: domain values to rows and back (S-4, S-5).
 *
 * Writing flattens a domain value into columns. Reading goes the other
 * way through the domain's own fallible constructors, so a row is never
 * trusted just because the store wrote it. Anything the domain rejects is
 * [`RepoError::Corrupt`](gymos_engine::RepoError::Corrupt), never a panic.
 *
 * Each module reads raw columns into a plain struct inside the driver's
 * row callback, then converts that struct to domain values outside it,
 * where errors can name the table, the row and the field.
 */

mod codes;
mod command;
mod entry;
mod exercise;
mod session;
mod set;

pub(crate) use command::{load_outcome, save_outcome};
pub(crate) use exercise::{load_exercise, save_exercise};
pub(crate) use session::{load_session, save_session, sessions_containing};

use gymos_domain::{Load, Name, Note, Timestamp, UtcOffset, Uuid};

/** The schema version stamped on every row written (D-7). */
const SCHEMA_VERSION: i64 = crate::migrations::LATEST;

/** A UUID as its 16 stored bytes. */
const fn blob(id: Uuid) -> [u8; 16] {
    id.into_bytes()
}

/** A stored ID, if it is exactly 16 bytes. */
fn uuid(raw: &[u8]) -> Option<Uuid> {
    <[u8; 16]>::try_from(raw).ok().map(Uuid::from_bytes)
}

/** A load's three columns: as entered, plus normalized nanograms. */
struct LoadColumns {
    hundredths: i64,
    unit: &'static str,
    nanograms: i64,
}

impl LoadColumns {
    fn of(load: Load) -> Self {
        Self {
            hundredths: i64::from(load.hundredths()),
            unit: codes::unit_code(load.unit()),
            nanograms: load.mass().nanograms(),
        }
    }
}

/**
 * A load from its columns. `None` if the domain rejects it or the stored
 * nanograms don't match what the entered value converts to.
 */
fn load(hundredths: i64, unit: &str, nanograms: i64) -> Option<Load> {
    let load = Load::new(u32::try_from(hundredths).ok()?, codes::unit(unit)?).ok()?;
    (load.mass().nanograms() == nanograms).then_some(load)
}

/**
 * A nullable column held a value the domain rejects. The caller turns it
 * into `RepoError::Corrupt` naming the field.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Invalid;

/** Parses a nullable column: `Ok(None)` if null. */
fn nullable<R, T>(
    raw: Option<R>,
    parse: impl FnOnce(R) -> Option<T>,
) -> Result<Option<T>, Invalid> {
    raw.map(|raw| parse(raw).ok_or(Invalid)).transpose()
}

/** An optional load: all three columns null, or all three a valid load. */
fn optional_load(
    hundredths: Option<i64>,
    unit: Option<&str>,
    nanograms: Option<i64>,
) -> Result<Option<Load>, Invalid> {
    match (hundredths, unit, nanograms) {
        (None, None, None) => Ok(None),
        (Some(h), Some(u), Some(n)) => load(h, u, n).map(Some).ok_or(Invalid),
        _ => Err(Invalid),
    }
}

/** A timestamp from UTC milliseconds and an offset in minutes. */
fn timestamp(utc_ms: i64, offset_min: i64) -> Option<Timestamp> {
    let offset = UtcOffset::from_minutes(i16::try_from(offset_min).ok()?).ok()?;
    Some(Timestamp::new(utc_ms, offset))
}

/** An optional timestamp: both columns null, or both valid. */
fn optional_timestamp(
    utc_ms: Option<i64>,
    offset_min: Option<i64>,
) -> Result<Option<Timestamp>, Invalid> {
    match (utc_ms, offset_min) {
        (None, None) => Ok(None),
        (Some(ms), Some(offset)) => timestamp(ms, offset).map(Some).ok_or(Invalid),
        _ => Err(Invalid),
    }
}

/**
 * A stored name. It must already be in the form `Name::new` produces
 * (trimmed), since only the domain writes names.
 */
fn name(raw: &str) -> Option<Name> {
    Name::new(raw).ok().filter(|n| n.as_str() == raw)
}

/** A stored optional note, held to the same rule as [`name`]. */
fn note(raw: Option<&str>) -> Result<Option<Note>, Invalid> {
    nullable(raw, |raw| Note::new(raw).ok().filter(|n| n.as_str() == raw))
}

/** A stored 0/1 flag. */
fn flag(raw: i64) -> Option<bool> {
    match raw {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}
