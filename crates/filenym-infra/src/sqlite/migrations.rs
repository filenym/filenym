use super::map_error;
use filenym_ports::PersistenceError as Error;
use rusqlite::{Connection, TransactionBehavior};

/// Private FileNym format marker ("FNYM"), not a registered global file type.
pub(super) const APPLICATION_ID: i64 = 0x464e594d;
pub(super) const VERSION: i64 = 1;
const INITIAL: &str = include_str!("../../migrations/0001_initial.sql");

pub(super) fn identity(conn: &Connection) -> Result<(i64, i64), Error> {
    Ok((
        conn.pragma_query_value(None, "application_id", |row| row.get(0))
            .map_err(map_error)?,
        conn.pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(map_error)?,
    ))
}

pub(super) fn verify(conn: &Connection) -> Result<(), Error> {
    if identity(conn)? != (APPLICATION_ID, VERSION) {
        return Err(Error::IncompatibleDatabase);
    }
    // Exact immutable migration DDL protects against dropped checks/indexes.
    for (name, statement) in ["canonical_files", "aliases"]
        .into_iter()
        .zip(INITIAL.split(';').filter(|s| !s.trim().is_empty()))
    {
        let actual: String = conn
            .query_row(
                "SELECT sql FROM sqlite_schema WHERE type='table' AND name=?1",
                [name],
                |row| row.get(0),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Error::IncompatibleDatabase,
                _ => map_error(error),
            })?;
        if actual != statement.trim() {
            return Err(Error::IncompatibleDatabase);
        }
    }
    let mut stmt=conn.prepare("SELECT name FROM sqlite_schema WHERE substr(name,1,7) <> 'sqlite_' AND name NOT IN ('canonical_files','aliases')").map_err(map_error)?;
    if stmt.exists([]).map_err(map_error)? {
        return Err(Error::IncompatibleDatabase);
    }
    let index_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type='index'",
            [],
            |row| row.get(0),
        )
        .map_err(map_error)?;
    if index_count != 4 {
        return Err(Error::IncompatibleDatabase);
    }
    let mut check = conn.prepare("PRAGMA quick_check").map_err(map_error)?;
    let results = check
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(map_error)?;
    for result in results {
        if result.map_err(map_error)? != "ok" {
            return Err(Error::CorruptDatabase);
        }
    }
    if conn
        .prepare("PRAGMA foreign_key_check")
        .map_err(map_error)?
        .exists([])
        .map_err(map_error)?
    {
        return Err(Error::InvalidStoredData);
    }
    Ok(())
}

pub(super) fn initialize(conn: &mut Connection) -> Result<(), Error> {
    apply_initial(conn, INITIAL)
}

fn apply_initial(conn: &mut Connection, sql: &str) -> Result<(), Error> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_error)?;
    if identity(&tx)? != (0, 0) {
        return Err(Error::IncompatibleDatabase);
    }
    let count: i64 = tx
        .query_row("SELECT count(*) FROM sqlite_schema", [], |row| row.get(0))
        .map_err(map_error)?;
    if count != 0 {
        return Err(Error::IncompatibleDatabase);
    }
    let result = (|| {
        tx.execute_batch(sql).map_err(|_| Error::MigrationFailed)?;
        tx.pragma_update(None, "application_id", APPLICATION_ID)
            .map_err(|_| Error::MigrationFailed)?;
        tx.pragma_update(None, "user_version", VERSION)
            .map_err(|_| Error::MigrationFailed)?;
        verify(&tx)?;
        Ok(())
    })();
    if let Err(error) = result {
        return if tx.rollback().is_ok() {
            Err(error)
        } else {
            Err(Error::UncertainOutcome)
        };
    }
    tx.commit().map_err(|_| Error::UncertainOutcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{fixture, OwnedDirectory};
    use filenym_ports::CanonicalFileRepository;
    #[test]
    fn initial_migration_rolls_back_schema_and_marker() {
        let directory = OwnedDirectory::new();
        let mut conn = directory.raw();
        conn.pragma_update(None, "journal_mode", "WAL").unwrap();
        let failed = format!("{INITIAL}\n PRAGMA application_id={APPLICATION_ID}; PRAGMA user_version={VERSION}; INVALID SQL;");
        assert_eq!(
            apply_initial(&mut conn, &failed),
            Err(Error::MigrationFailed)
        );
        assert_eq!(identity(&conn).unwrap(), (0, 0));
        assert_eq!(
            conn.query_row("SELECT count(*) FROM sqlite_schema", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        drop(conn);
        assert!(matches!(
            crate::SqliteRepository::open_existing(directory.db()),
            Err(Error::IncompatibleDatabase)
        ));
        let mut conn = directory.raw();
        apply_initial(&mut conn, INITIAL).unwrap();
        verify(&conn).unwrap();
        drop(conn);
        drop(crate::SqliteRepository::open_existing(directory.db()).unwrap());
    }
    #[test]
    fn later_fixture_migration_failure_preserves_version_and_records() {
        let directory = OwnedDirectory::new();
        let mut conn = directory.raw();
        conn.pragma_update(None, "journal_mode", "WAL").unwrap();
        initialize(&mut conn).unwrap();
        let original = fixture("migration");
        {
            let mut repository = crate::SqliteRepository::open_existing(directory.db()).unwrap();
            repository.insert(&original).unwrap();
        }
        for failing in [true, false] {
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .unwrap();
            tx.execute_batch("CREATE TABLE fixture(value TEXT) STRICT; UPDATE canonical_files SET revision=1; DELETE FROM aliases; PRAGMA user_version=2;")
                .unwrap();
            if failing {
                assert!(tx.execute_batch("INVALID SQL").is_err());
                tx.rollback().unwrap();
                assert_eq!(identity(&conn).unwrap(), (APPLICATION_ID, VERSION));
                verify(&conn).unwrap();
                let mut repository =
                    crate::SqliteRepository::open_existing(directory.db()).unwrap();
                let stored = repository.load(original.id()).unwrap().unwrap();
                assert_eq!(stored.file(), &original);
                assert_eq!(stored.revision().value(), 0);
            } else {
                tx.commit().unwrap();
                assert_eq!(identity(&conn).unwrap(), (APPLICATION_ID, 2));
            }
        }
    }
}
