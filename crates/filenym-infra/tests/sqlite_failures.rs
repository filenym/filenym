mod support;
use filenym_core::*;
use filenym_infra::SqliteRepository;
use filenym_ports::{CanonicalFileRepository, PersistenceError as Error};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};
use support::*;

#[test]
fn failed_inserts_and_updates_rollback_the_complete_aggregate() {
    for trigger in [
        "CREATE TRIGGER fail_parent AFTER UPDATE ON canonical_files BEGIN SELECT RAISE(ABORT,'fixture'); END;",
        "CREATE TRIGGER fail_delete AFTER DELETE ON aliases BEGIN SELECT RAISE(ABORT,'fixture'); END;",
        "CREATE TRIGGER fail_alias AFTER INSERT ON aliases WHEN NEW.id='1' BEGIN SELECT RAISE(ABORT,'fixture'); END;",
    ] {
        let dir=OwnedDirectory::new();let mut repo=SqliteRepository::create_new(dir.db()).unwrap();let original=fixture("one");let rev=repo.insert(&original).unwrap();let snapshot=repo.load(original.id()).unwrap().unwrap();
        let raw=dir.raw();raw.execute_batch(trigger).unwrap();let mut changed=original.clone();changed.rename_alias(&alias_id("0"),name("Changed.txt"),time(30)).unwrap();
        assert!(repo.update(&changed,rev).is_err());assert_eq!(repo.load(original.id()).unwrap().unwrap(),snapshot);
        raw.execute_batch("DROP TRIGGER IF EXISTS fail_parent; DROP TRIGGER IF EXISTS fail_delete; DROP TRIGGER IF EXISTS fail_alias;").unwrap();drop(raw);drop(repo);
        let mut reopened=SqliteRepository::open_existing(dir.db()).unwrap();assert_eq!(reopened.load(original.id()).unwrap().unwrap(),snapshot);
    }
    let dir = OwnedDirectory::new();
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    let raw = dir.raw();
    raw.execute_batch("CREATE TRIGGER fail_insert AFTER INSERT ON aliases BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    assert!(repo.insert(&fixture("one")).is_err());
    assert!(repo.list_ids().unwrap().is_empty());
    assert_eq!(
        raw.query_row("SELECT count(*) FROM aliases", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn deferred_constraint_failure_at_commit_rolls_back() {
    let dir = OwnedDirectory::new();
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    let file = fixture("one");
    let rev = repo.insert(&file).unwrap();
    let snapshot = repo.load(file.id()).unwrap().unwrap();
    let raw = dir.raw();
    raw.execute_batch("CREATE TABLE fixture_parent(id INTEGER PRIMARY KEY); CREATE TABLE fixture_child(id INTEGER REFERENCES fixture_parent(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail_commit AFTER INSERT ON aliases BEGIN INSERT INTO fixture_child VALUES(999); END;").unwrap();
    assert_eq!(repo.update(&file, rev), Err(Error::InvalidStoredData));
    assert_eq!(repo.load(file.id()).unwrap().unwrap(), snapshot);
    assert_eq!(
        raw.query_row("SELECT count(*) FROM fixture_child", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn malformed_stored_values_are_rejected_without_repair() {
    for sql in [
        "UPDATE aliases SET name='CON.pdf' WHERE id='0'",
        "UPDATE aliases SET name_key=X'01' WHERE id='0'",
        "UPDATE aliases SET position=100 WHERE id='7'",
        "UPDATE aliases SET created_seconds=0 WHERE id='0'",
        "UPDATE aliases SET updated_seconds=0 WHERE id='0'",
        "UPDATE canonical_files SET identity_scheme=' '",
        "UPDATE canonical_files SET identity_value=NULL",
        "UPDATE canonical_files SET size_u64_be=X'01'",
        "UPDATE canonical_files SET modified_seconds=NULL",
        "UPDATE canonical_files SET status='future'",
        "UPDATE canonical_files SET unavailable_reason='unknown'",
        "UPDATE canonical_files SET observed_nanoseconds=-1",
        "UPDATE canonical_files SET revision=-1",
        "UPDATE canonical_files SET path_value=X'0000'",
    ] {
        let dir = OwnedDirectory::new();
        let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
        let file = fixture("one");
        repo.insert(&file).unwrap();
        let raw = dir.raw();
        raw.execute_batch("PRAGMA ignore_check_constraints=ON;")
            .unwrap();
        raw.execute_batch(sql).unwrap();
        assert_eq!(repo.load(file.id()), Err(Error::InvalidStoredData), "{sql}");
        assert_eq!(repo.load(file.id()), Err(Error::InvalidStoredData));
    }
    let dir = OwnedDirectory::new();
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    repo.insert(&fixture("one")).unwrap();
    let raw = dir.raw();
    raw.execute_batch(
        "PRAGMA ignore_check_constraints=ON; UPDATE canonical_files SET path_encoding='future';",
    )
    .unwrap();
    assert_eq!(repo.load(&id("one")), Err(Error::UnsupportedPathEncoding));
}

#[test]
fn all_schema_constraints_reject_invalid_direct_writes() {
    let dir = OwnedDirectory::new();
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    repo.insert(&fixture("one")).unwrap();
    let raw = dir.raw();
    let error = raw
        .execute(
            "INSERT INTO canonical_files
         SELECT '',path_encoding,path_value,identity_scheme,identity_value,
                size_u64_be,modified_seconds,modified_nanoseconds,
                registered_seconds,registered_nanoseconds,observed_seconds,
                observed_nanoseconds,status,unavailable_reason,revision
         FROM canonical_files WHERE id='one'",
            [],
        )
        .unwrap_err();
    assert!(
        matches!(error, rusqlite::Error::SqliteFailure(code, _) if code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_CHECK)
    );
    for assignment in [
        "path_encoding='future'",
        "path_value=X''",
        "path_value='text'",
        "identity_scheme=''",
        "identity_value=X''",
        "identity_scheme=NULL",
        "identity_value=NULL",
        "size_u64_be=X''",
        "modified_seconds=NULL",
        "modified_nanoseconds=NULL",
        "modified_nanoseconds=-1",
        "modified_nanoseconds=1000000000",
        "registered_seconds=NULL",
        "registered_seconds='invalid'",
        "registered_nanoseconds=-1",
        "registered_nanoseconds=1000000000",
        "observed_seconds=NULL",
        "observed_nanoseconds=-1",
        "observed_nanoseconds=1000000000",
        "status='future'",
        "status=NULL",
        "unavailable_reason='unknown'",
        "status='unavailable'",
        "status='unavailable',unavailable_reason='future'",
        "revision=-1",
        "revision=NULL",
    ] {
        assert!(
            raw.execute(&format!("UPDATE canonical_files SET {assignment}"), [])
                .is_err(),
            "{assignment}"
        );
    }
    for assignment in [
        "id=''",
        "id=NULL",
        "name=''",
        "name=NULL",
        "name=printf('%0256d',0)",
        "name_key=X''",
        "name_key=NULL",
        "name_key='text'",
        "position=-1",
        "position=1",
        "position=NULL",
        "created_seconds=NULL",
        "created_nanoseconds=-1",
        "created_nanoseconds=1000000000",
        "updated_seconds=NULL",
        "updated_nanoseconds=-1",
        "updated_nanoseconds=1000000000",
        "canonical_file_id='missing'",
        "canonical_file_id=NULL",
    ] {
        assert!(
            raw.execute(&format!("UPDATE aliases SET {assignment} WHERE id='0'"), [])
                .is_err(),
            "{assignment}"
        );
    }
    assert_eq!(
        repo.load(&id("one")).unwrap().unwrap().file(),
        &fixture("one")
    );
}

#[test]
fn busy_timeout_is_bounded_and_retry_succeeds_after_unlock() {
    let dir = OwnedDirectory::new();
    drop(SqliteRepository::create_new(dir.db()).unwrap());
    let mut repo =
        SqliteRepository::open_existing_with_timeout(dir.db(), Duration::from_millis(25)).unwrap();
    let raw = dir.raw();
    raw.execute_batch("BEGIN IMMEDIATE;").unwrap();
    let start = Instant::now();
    assert_eq!(repo.insert(&fixture("one")), Err(Error::Busy));
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(repo.list_ids().unwrap().is_empty());
    raw.execute_batch("ROLLBACK;").unwrap();
    repo.insert(&fixture("one")).unwrap();
    assert!(matches!(
        SqliteRepository::open_existing_with_timeout(dir.db(), Duration::from_secs(6)),
        Err(Error::UnrepresentableValue)
    ));
}

#[test]
fn wal_read_snapshot_remains_consistent_while_writer_commits() {
    let dir = OwnedDirectory::new();
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    let mut file = fixture("one");
    let rev = repo.insert(&file).unwrap();
    let raw = dir.raw();
    assert_eq!(
        raw.pragma_query_value(None, "journal_mode", |r| r.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
    raw.execute_batch("BEGIN;").unwrap();
    assert_eq!(
        raw.query_row("SELECT revision FROM canonical_files", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    let count = i64::try_from(file.aliases().len()).unwrap();
    file.delete_alias(&alias_id("0")).unwrap();
    repo.update(&file, rev).unwrap();
    assert_eq!(
        raw.query_row("SELECT revision FROM canonical_files", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        raw.query_row("SELECT count(*) FROM aliases", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        count
    );
    raw.execute_batch("COMMIT;").unwrap();
    assert_eq!(repo.load(file.id()).unwrap().unwrap().file(), &file);
}

#[test]
fn creation_is_exclusive_even_under_concurrent_calls() {
    let dir = OwnedDirectory::new();
    let path = dir.db();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let other = barrier.clone();
    let other_path = path.clone();
    let worker = std::thread::spawn(move || {
        other.wait();
        SqliteRepository::create_new(other_path)
    });
    barrier.wait();
    let one = SqliteRepository::create_new(&path);
    let two = worker.join().unwrap();
    assert_eq!(usize::from(one.is_ok()) + usize::from(two.is_ok()), 1);
    assert!(
        matches!(one, Err(Error::DatabaseAlreadyExists))
            || matches!(two, Err(Error::DatabaseAlreadyExists))
    );
    drop(one);
    drop(two);
    drop(SqliteRepository::open_existing(path).unwrap());
}

#[test]
fn database_path_failures_are_safe() {
    let dir = OwnedDirectory::new();
    assert!(matches!(
        SqliteRepository::create_new("relative.sqlite3"),
        Err(Error::Unavailable)
    ));
    assert!(
        SqliteRepository::create_new(dir.path.join("absent").join("registry.sqlite3")).is_err()
    );
    assert!(SqliteRepository::open_existing(&dir.path).is_err());
    let repo = SqliteRepository::create_new(dir.db()).unwrap();
    drop(repo);
    #[cfg(windows)]
    {
        // Owned fixture only: deny native write opening through a read-only file.
        let original_permissions = fs::metadata(dir.db()).unwrap().permissions();
        let mut permissions = original_permissions.clone();
        permissions.set_readonly(true);
        fs::set_permissions(dir.db(), permissions).unwrap();
        let result = SqliteRepository::open_existing(dir.db());
        fs::set_permissions(dir.db(), original_permissions).unwrap();
        assert!(
            matches!(
                result,
                Err(Error::PermissionDenied) | Err(Error::Unavailable)
            ),
            "read-only opening should fail"
        );
    }
}

fn child(mode: &str, path: &std::path::Path) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "persistence_restart_worker", "--nocapture"])
        .env("FILENYM_TEST_DB", path)
        .env("FILENYM_TEST_MODE", mode);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

#[test]
fn persistence_restart_worker() {
    let Some(path) = std::env::var_os("FILENYM_TEST_DB") else {
        return;
    };
    match std::env::var("FILENYM_TEST_MODE").unwrap().as_str() {
        "write" => {
            let mut repo = SqliteRepository::open_existing(&path).unwrap();
            repo.insert(&fixture("one")).unwrap();
        }
        "read" => {
            let mut repo = SqliteRepository::open_existing(&path).unwrap();
            assert_eq!(
                repo.load(&id("one")).unwrap().unwrap().file(),
                &fixture("one")
            );
        }
        "uncommitted" => {
            let raw = rusqlite::Connection::open(path).unwrap();
            raw.execute_batch("BEGIN IMMEDIATE; UPDATE canonical_files SET revision=revision+1; DELETE FROM aliases;").unwrap();
            println!("FILENYM_READY");
            std::io::stdout().flush().unwrap();
            let _ = std::io::stdin().read(&mut [0]);
            panic!("controller should terminate child before commit");
        }
        _ => panic!("unknown test worker mode"),
    }
}

#[test]
fn subprocess_commit_restart_and_termination_preserve_records() {
    let dir = OwnedDirectory::new();
    drop(SqliteRepository::create_new(dir.db()).unwrap());
    for mode in ["write", "read"] {
        let output = child(mode, &dir.db()).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let mut worker = child("uncommitted", &dir.db())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = worker.stdout.take().unwrap();
    let (send, receive) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if line.unwrap().contains("FILENYM_READY") {
                send.send(()).unwrap();
                return;
            }
        }
    });
    if receive.recv_timeout(Duration::from_secs(10)).is_err() {
        let _ = worker.kill();
        let _ = worker.wait();
        panic!("restart worker did not reach transaction barrier")
    }
    worker.kill().unwrap();
    worker.wait().unwrap();
    reader.join().unwrap();
    let mut repo = SqliteRepository::open_existing(dir.db()).unwrap();
    let stored = repo.load(&id("one")).unwrap().unwrap();
    assert_eq!(stored.file(), &fixture("one"));
    assert_eq!(stored.revision().value(), 0);
}

#[test]
fn invalid_inventory_ids_and_timestamp_precision_do_not_silently_normalize() {
    let dir = OwnedDirectory::new();
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    let file = CanonicalFile::new(
        id("one"),
        KnownFileLocation::new("fixture").unwrap(),
        None,
        FileMetadata::new(0, None),
        time(1),
    );
    repo.insert(&file).unwrap();
    let raw = dir.raw();
    raw.execute_batch("UPDATE canonical_files SET id=' bad '")
        .unwrap();
    assert_eq!(repo.list_ids(), Err(Error::InvalidStoredData));
    #[cfg(windows)]
    {
        raw.execute_batch("UPDATE canonical_files SET id='one', observed_nanoseconds=1")
            .unwrap();
        assert_eq!(repo.load(&id("one")), Err(Error::InvalidStoredData));
    }
}
