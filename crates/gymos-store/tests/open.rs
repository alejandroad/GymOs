/*!
 * Opening a database (DD-04, S-6, S-8) and reading rows the domain rejects
 * (S-5).
 */

#![expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test setup helpers; a panic is a failed test"
)]

#[path = "support/sqlite.rs"]
mod support;

use gymos_domain::{SessionId, TrackingMode};
use gymos_engine::{EngineError, ReadTx, RepoError, Store};
use gymos_store::{OpenError, SqliteStore};
use gymos_testkit::{SessionPlan, kg, working};
use support::{TempDb, file_gym, user_version};

/* ---- fresh, reopened, newer ---------------------------------------------- */

#[test]
fn a_fresh_file_gets_the_latest_schema_and_no_backup() {
    let db = TempDb::new();
    drop(db.open());
    assert_eq!(user_version(&db.path), 1);
    assert!(
        !db.config("test").backup_path().exists(),
        "nothing to protect"
    );
}

#[test]
fn reopening_applies_nothing_and_keeps_the_data() {
    let db = TempDb::new();
    let session = {
        let gym = file_gym(&db);
        let bench = gym.create_exercise("Bench Press", TrackingMode::WeightReps, None);
        gym.log_session(&SessionPlan::new().exercise(bench, [working(kg(100, 0), 5)]))
            .session
    };
    let before = std::fs::read(&db.path).unwrap();

    let reopened = SqliteStore::open(&db.config("second")).unwrap();
    assert_eq!(user_version(&db.path), 1);
    assert!(!db.config("second").backup_path().exists());
    assert_eq!(std::fs::read(&db.path).unwrap(), before, "no migration ran");
    let stored = reopened.read(|tx| Ok(tx.session(session)?)).unwrap();
    assert_eq!(stored.unwrap().exercises().len(), 1);
}

#[test]
fn the_connection_enforces_foreign_keys_and_uses_wal() {
    let db = TempDb::new();
    drop(db.open());
    let raw = db.raw();
    let mode: String = raw
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .unwrap();
    assert_eq!(mode, "wal", "journal mode is stored in the file");

    /* An entry for a session that doesn't exist is refused by the store's
     * own connection, which has foreign keys on. */
    let store = db.open();
    let orphan = gymos_testkit::SessionBuilder::new(
        SessionId::from_uuid(gymos_domain::Uuid::from_u128(1)),
        gymos_testkit::at(0),
    )
    .entry(
        gymos_domain::SessionExerciseId::from_uuid(gymos_domain::Uuid::from_u128(2)),
        &gymos_testkit::exercise(
            gymos_domain::ExerciseId::from_uuid(gymos_domain::Uuid::from_u128(404)),
            "Never saved",
            TrackingMode::WeightReps,
        ),
        [],
    )
    .build(&gymos_testkit::SeqIds::new());
    let result = store.write(|tx| Ok(gymos_engine::WriteTx::save_session(tx, &orphan)?));
    assert!(
        matches!(result, Err(EngineError::Repo(RepoError::Backend(_)))),
        "{result:?}"
    );
}

#[test]
fn a_newer_schema_is_refused_and_the_file_is_untouched() {
    let db = TempDb::new();
    drop(db.open());
    db.raw().pragma_update(None, "user_version", 99).unwrap();
    let before = std::fs::read(&db.path).unwrap();

    let refused = SqliteStore::open(&db.config("test")).unwrap_err();
    assert_eq!(
        refused,
        OpenError::NewerSchema {
            found: 99,
            supported: 1
        }
    );
    assert_eq!(std::fs::read(&db.path).unwrap(), before, "byte for byte");
    assert!(!db.config("test").backup_path().exists());
}

/** Set by the parent test to tell [`wal_child`] which file to write. */
const WAL_CHILD: &str = "GYMOS_STORE_TEST_WAL_CHILD";

/**
 * Not a test of its own: [`a_newer_schema_left_in_the_wal_leaves_the_main_file_untouched`]
 * runs this in a child process. It bumps the schema version with
 * checkpoints off and exits without closing, as a crashed newer GymOs
 * would, so the change exists only as frames in the `-wal` file.
 */
#[test]
#[ignore = "helper process for a_newer_schema_left_in_the_wal_leaves_the_main_file_untouched"]
fn wal_child() {
    let Some(path) = std::env::var_os(WAL_CHILD) else {
        return;
    };
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
    conn.pragma_update(None, "user_version", 99).unwrap();
    #[expect(
        clippy::exit,
        reason = "leave without closing, so the WAL keeps its frames"
    )]
    std::process::exit(0);
}

#[test]
fn a_newer_schema_left_in_the_wal_leaves_the_main_file_untouched() {
    let db = TempDb::new();
    drop(db.open());
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "wal_child", "--test-threads=1"])
        .env(WAL_CHILD, &db.path)
        .status()
        .unwrap();
    assert!(status.success());
    let mut wal = db.path.clone().into_os_string();
    wal.push("-wal");
    let wal = std::path::PathBuf::from(wal);
    assert!(
        std::fs::metadata(&wal).unwrap().len() > 0,
        "the child left frames in the WAL"
    );
    let before = std::fs::read(&db.path).unwrap();

    assert_eq!(
        SqliteStore::open(&db.config("test")).unwrap_err(),
        OpenError::NewerSchema {
            found: 99,
            supported: 1
        }
    );
    assert_eq!(
        std::fs::read(&db.path).unwrap(),
        before,
        "the WAL was not checkpointed into the main file"
    );
    assert!(wal.exists(), "the WAL is left for the newer GymOs");
}

#[test]
fn a_negative_schema_version_is_unknown() {
    let db = TempDb::new();
    db.raw().pragma_update(None, "user_version", -1).unwrap();
    assert_eq!(
        SqliteStore::open(&db.config("test")).unwrap_err(),
        OpenError::UnknownSchema(-1)
    );
}

/* ---- backups (S-6) ------------------------------------------------------- */

/** A schema-v0 file that already holds someone's data. */
fn v0_with_data(db: &TempDb) {
    db.raw()
        .execute_batch(
            "CREATE TABLE notes (body TEXT NOT NULL);
             INSERT INTO notes (body) VALUES ('keep me');",
        )
        .unwrap();
}

#[test]
fn a_pending_migration_on_a_database_with_data_is_backed_up_first() {
    let db = TempDb::new();
    v0_with_data(&db);
    let config = db.config("2026-11-19T07-30-00Z");

    drop(SqliteStore::open(&config).unwrap());
    assert_eq!(user_version(&db.path), 1);
    let backup = config.backup_path();
    assert!(
        backup.ends_with("gymos.db.2026-11-19T07-30-00Z.bak"),
        "{backup:?}"
    );
    assert_eq!(
        user_version(&backup),
        0,
        "the backup is the pre-migration file"
    );
    let kept: String = rusqlite::Connection::open(&backup)
        .unwrap()
        .query_row("SELECT body FROM notes", [], |row| row.get(0))
        .unwrap();
    assert_eq!(kept, "keep me");

    /* Nothing is pending any more, so a later open takes no backup. */
    let later = db.config("2026-11-20T07-30-00Z");
    drop(SqliteStore::open(&later).unwrap());
    assert!(!later.backup_path().exists());
}

#[test]
fn an_existing_backup_is_never_overwritten() {
    let db = TempDb::new();
    v0_with_data(&db);
    let config = db.config("same-label");
    std::fs::write(config.backup_path(), b"an older backup").unwrap();

    assert_eq!(
        SqliteStore::open(&config).unwrap_err(),
        OpenError::BackupExists(config.backup_path())
    );
    assert_eq!(
        std::fs::read(config.backup_path()).unwrap(),
        b"an older backup"
    );
    assert_eq!(user_version(&db.path), 0, "nothing was migrated");
}

#[test]
fn a_backup_label_that_could_leave_the_directory_is_refused() {
    let db = TempDb::new();
    for label in ["", "../escape", "a/b", ".hidden"] {
        assert_eq!(
            SqliteStore::open(&db.config(label)).unwrap_err(),
            OpenError::InvalidBackupLabel,
            "{label:?}"
        );
    }
    assert!(!db.path.exists(), "refused before the file was created");
}

#[test]
fn a_failed_migration_is_rolled_back_after_the_backup() {
    let db = TempDb::new();
    /* A v0 file with a clashing table: the v1 migration can't create it. */
    db.raw()
        .execute_batch("CREATE TABLE exercises (id INTEGER); INSERT INTO exercises VALUES (1);")
        .unwrap();
    let config = db.config("before-v1");

    assert!(matches!(
        SqliteStore::open(&config).unwrap_err(),
        OpenError::Migration(_)
    ));
    assert_eq!(user_version(&db.path), 0);
    assert!(config.backup_path().exists());
    let tables: i64 = db
        .raw()
        .query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type = 'table'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tables, 1, "the half-applied migration was rolled back");
}

/* ---- corrupt rows (S-5) -------------------------------------------------- */

/**
 * Logs one session of 100 kg × 5 through the engine, closes the store, then
 * runs `sql` against the file with CHECK constraints off, as a hand edit or
 * a bug could. Returns the session.
 */
fn plant(db: &TempDb, sql: &str) -> SessionId {
    let session = {
        let gym = file_gym(db);
        let bench = gym.create_exercise("Bench Press", TrackingMode::WeightReps, None);
        gym.log_session(&SessionPlan::new().exercise(bench, [working(kg(100, 0), 5)]))
            .session
    };
    let raw = db.raw();
    raw.pragma_update(None, "ignore_check_constraints", true)
        .unwrap();
    assert_eq!(raw.execute(sql, []).unwrap(), 1, "{sql}");
    session
}

/** Reading the session fails as `Corrupt`; returns the detail. */
fn corrupt_detail(db: &TempDb, session: SessionId) -> String {
    let store = db.open();
    match store.read(|tx| Ok(tx.session(session)?)) {
        Err(EngineError::Repo(RepoError::Corrupt(detail))) => detail,
        other => panic!("expected Corrupt, got {other:?}"),
    }
}

#[test]
fn an_unknown_enum_code_is_corrupt() {
    let db = TempDb::new();
    let session = plant(&db, "UPDATE sets SET set_type = 'superset'");
    let detail = corrupt_detail(&db, session);
    assert!(
        detail.contains("sets") && detail.contains("set_type"),
        "{detail}"
    );
    assert!(
        !detail.contains("superset"),
        "the value never leaks: {detail}"
    );
}

#[test]
fn an_out_of_range_rep_count_is_corrupt() {
    let db = TempDb::new();
    let session = plant(&db, "UPDATE sets SET reps = 0");
    assert!(corrupt_detail(&db, session).contains("reps"));
}

#[test]
fn a_load_whose_nanograms_disagree_is_corrupt() {
    let db = TempDb::new();
    let session = plant(&db, "UPDATE sets SET load_ng = load_ng + 1");
    assert!(corrupt_detail(&db, session).contains("load"));
}

#[test]
fn a_null_in_a_column_the_mode_requires_is_corrupt() {
    let db = TempDb::new();
    let session = plant(&db, "UPDATE sets SET reps = NULL");
    assert!(corrupt_detail(&db, session).contains("measurements"));
}

#[test]
fn text_that_is_not_utf8_is_corrupt() {
    let db = TempDb::new();
    let session = plant(&db, "UPDATE sets SET note = CAST(x'ff' AS TEXT)");
    corrupt_detail(&db, session);
}

#[test]
fn a_corrupt_row_fails_the_engine_query_too() {
    let db = TempDb::new();
    let session = plant(&db, "UPDATE sessions SET source_kind = 'telepathy'");
    let gym = file_gym(&db);
    assert!(matches!(
        gym.os().session_verdicts(session),
        Err(EngineError::Repo(RepoError::Corrupt(_)))
    ));
}
