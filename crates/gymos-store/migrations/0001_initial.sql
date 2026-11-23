-- GymOs schema v1 (DD-04, PROMPT-3 §4).
--
-- Forward-only: once released, this file is never edited. Changes go in a
-- new migration.
--
-- Conventions:
-- * Every table is STRICT, so a column holds exactly its declared type.
-- * IDs are 16-byte UUID blobs (S-4). UUID v7 sorts bytewise, which matches
--   the engine's SessionId tie-break (DD-15).
-- * Timestamps are UTC milliseconds plus the local offset in minutes, as
--   entered (DD-04). Order by the UTC column only.
-- * A load is stored as entered (hundredths + unit) plus normalized
--   nanograms (`_ng`) for sorting. The store re-derives `_ng` on every read
--   and reports a mismatch as corrupt.
-- * Enums are stable lowercase codes, never Rust discriminants. The mapping
--   lives in src/rows/codes.rs.
-- * CHECKs pin down the *shape* of a row (which columns a mode uses, which
--   go together). Value *ranges* (reps 1-1000, RPE 1-10, ...) belong to the
--   domain and are re-validated on read, so they are not duplicated here
--   where they could drift.
-- * Tables that will sync carry the D-7 columns: `hlc_updated` and
--   `field_clocks` are placeholders ('' and '{}') until sync defines their
--   format (S-2); `deleted` is the tombstone every read filters on (S-3);
--   `schema_version` is the schema that wrote the row.

CREATE TABLE exercises (
    id                          BLOB    NOT NULL PRIMARY KEY CHECK (length(id) = 16),
    name                        TEXT    NOT NULL,
    mode                        TEXT    NOT NULL CHECK (mode IN (
                                    'weight_reps', 'bodyweight_reps', 'weighted_bodyweight',
                                    'assisted_bodyweight', 'duration', 'distance_load',
                                    'reps_only', 'time_reps')),
    -- The progression scheme: all five set, or all five null.
    scheme_min_reps             INTEGER,
    scheme_max_reps             INTEGER,
    scheme_increment_hundredths INTEGER,
    scheme_increment_unit       TEXT    CHECK (scheme_increment_unit IN ('kg', 'lb')),
    scheme_increment_ng         INTEGER,
    hlc_updated                 TEXT    NOT NULL DEFAULT '',
    field_clocks                TEXT    NOT NULL DEFAULT '{}',
    deleted                     INTEGER NOT NULL DEFAULT 0 CHECK (deleted IN (0, 1)),
    schema_version              INTEGER NOT NULL,
    CHECK ((scheme_min_reps IS NULL) = (scheme_max_reps IS NULL)
       AND (scheme_min_reps IS NULL) = (scheme_increment_hundredths IS NULL)
       AND (scheme_min_reps IS NULL) = (scheme_increment_unit IS NULL)
       AND (scheme_min_reps IS NULL) = (scheme_increment_ng IS NULL))
) STRICT;

CREATE TABLE sessions (
    id                 BLOB    NOT NULL PRIMARY KEY CHECK (length(id) = 16),
    started_utc_ms     INTEGER NOT NULL,
    started_offset_min INTEGER NOT NULL,
    ended_utc_ms       INTEGER,
    ended_offset_min   INTEGER,
    title              TEXT,
    note               TEXT,
    -- No templates table yet: a plain ID (PROMPT-3 §3 "Out").
    template_id        BLOB    CHECK (template_id IS NULL OR length(template_id) = 16),
    deload             INTEGER NOT NULL CHECK (deload IN (0, 1)),
    -- Provenance (data-model rule 4).
    source_kind        TEXT    NOT NULL CHECK (source_kind IN ('manual', 'import')),
    import_format      TEXT    CHECK (import_format IN (
                           'strong', 'hevy', 'garmin_activities', 'garmin_vo2max')),
    importer_version   INTEGER,
    hlc_updated        TEXT    NOT NULL DEFAULT '',
    field_clocks       TEXT    NOT NULL DEFAULT '{}',
    deleted            INTEGER NOT NULL DEFAULT 0 CHECK (deleted IN (0, 1)),
    schema_version     INTEGER NOT NULL,
    CHECK ((ended_utc_ms IS NULL) = (ended_offset_min IS NULL)),
    CHECK (CASE source_kind
        WHEN 'manual' THEN import_format IS NULL AND importer_version IS NULL
        WHEN 'import' THEN import_format IS NOT NULL AND importer_version IS NOT NULL
    END)
) STRICT;

CREATE INDEX sessions_by_start ON sessions (started_utc_ms, id);

CREATE TABLE session_entries (
    id             BLOB    NOT NULL PRIMARY KEY CHECK (length(id) = 16),
    session_id     BLOB    NOT NULL REFERENCES sessions (id),
    exercise_id    BLOB    NOT NULL REFERENCES exercises (id),
    -- Order within the session, rewritten on every save (S-3). Occurrence
    -- matching (DD-15 rule 4) depends on it.
    position       INTEGER NOT NULL,
    -- The tracking mode captured when the entry was created.
    mode           TEXT    NOT NULL CHECK (mode IN (
                       'weight_reps', 'bodyweight_reps', 'weighted_bodyweight',
                       'assisted_bodyweight', 'duration', 'distance_load',
                       'reps_only', 'time_reps')),
    note           TEXT,
    hlc_updated    TEXT    NOT NULL DEFAULT '',
    field_clocks   TEXT    NOT NULL DEFAULT '{}',
    deleted        INTEGER NOT NULL DEFAULT 0 CHECK (deleted IN (0, 1)),
    schema_version INTEGER NOT NULL
) STRICT;

-- `sessions_containing`.
CREATE INDEX session_entries_by_exercise ON session_entries (exercise_id) WHERE deleted = 0;
-- Loading a session's entries in order, and tombstoning them on save. Not
-- partial: the tombstone pass also visits rows that are already deleted.
CREATE INDEX session_entries_by_session ON session_entries (session_id, position);

CREATE TABLE sets (
    id              BLOB    NOT NULL PRIMARY KEY CHECK (length(id) = 16),
    entry_id        BLOB    NOT NULL REFERENCES session_entries (id),
    position        INTEGER NOT NULL,
    set_type        TEXT    NOT NULL CHECK (set_type IN (
                        'warmup', 'working', 'drop', 'failure', 'top_set', 'backoff')),
    -- Which SetMetrics variant this row holds; the CHECK below says which
    -- measurement columns that variant uses.
    mode            TEXT    NOT NULL CHECK (mode IN (
                        'weight_reps', 'bodyweight_reps', 'weighted_bodyweight',
                        'assisted_bodyweight', 'duration', 'distance_load',
                        'reps_only', 'time_reps')),
    -- The load, added load or assistance, depending on the mode.
    load_hundredths INTEGER,
    load_unit       TEXT    CHECK (load_unit IN ('kg', 'lb')),
    load_ng         INTEGER,
    reps            INTEGER,
    duration_s      INTEGER,
    distance_mm     INTEGER,
    -- RPE in half-points, or RIR in reps.
    effort_scale    TEXT    CHECK (effort_scale IN ('rpe', 'rir')),
    effort_value    INTEGER,
    note            TEXT,
    hlc_updated     TEXT    NOT NULL DEFAULT '',
    field_clocks    TEXT    NOT NULL DEFAULT '{}',
    deleted         INTEGER NOT NULL DEFAULT 0 CHECK (deleted IN (0, 1)),
    schema_version  INTEGER NOT NULL,
    CHECK ((load_hundredths IS NULL) = (load_unit IS NULL)
       AND (load_hundredths IS NULL) = (load_ng IS NULL)),
    CHECK ((effort_scale IS NULL) = (effort_value IS NULL)),
    CHECK (CASE mode
        WHEN 'weight_reps' THEN load_unit IS NOT NULL AND reps IS NOT NULL
                            AND duration_s IS NULL AND distance_mm IS NULL
        WHEN 'bodyweight_reps' THEN load_unit IS NULL AND reps IS NOT NULL
                            AND duration_s IS NULL AND distance_mm IS NULL
        WHEN 'weighted_bodyweight' THEN load_unit IS NOT NULL AND reps IS NOT NULL
                            AND duration_s IS NULL AND distance_mm IS NULL
        WHEN 'assisted_bodyweight' THEN load_unit IS NOT NULL AND reps IS NOT NULL
                            AND duration_s IS NULL AND distance_mm IS NULL
        WHEN 'duration' THEN duration_s IS NOT NULL
                            AND reps IS NULL AND distance_mm IS NULL
        WHEN 'distance_load' THEN distance_mm IS NOT NULL AND load_unit IS NOT NULL
                            AND reps IS NULL
        WHEN 'reps_only' THEN reps IS NOT NULL
                            AND load_unit IS NULL AND duration_s IS NULL AND distance_mm IS NULL
        WHEN 'time_reps' THEN reps IS NOT NULL AND duration_s IS NOT NULL
                            AND load_unit IS NULL AND distance_mm IS NULL
    END)
) STRICT;

CREATE INDEX sets_by_entry ON sets (entry_id, position) WHERE deleted = 0;

-- Idempotency (DD-15, S-7): every applied CommandId with what it did, so a
-- replay returns the same outcome. A local log, not synced: it has no D-7
-- columns. The sync outbox is a separate table that comes with sync.
CREATE TABLE applied_commands (
    id           BLOB NOT NULL PRIMARY KEY CHECK (length(id) = 16),
    outcome_kind TEXT NOT NULL CHECK (outcome_kind IN (
                     'exercise_created', 'session_started', 'exercise_added',
                     'set_logged', 'session_finished', 'deload_set')),
    outcome_id   BLOB CHECK (outcome_id IS NULL OR length(outcome_id) = 16),
    CHECK ((outcome_kind IN ('session_finished', 'deload_set')) = (outcome_id IS NULL))
) STRICT;
