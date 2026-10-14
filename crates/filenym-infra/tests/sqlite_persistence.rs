mod support;
use filenym_core::*;
use filenym_infra::SqliteRepository;
use filenym_ports::{CanonicalFileRepository, PersistenceError as Error, PersistenceRevision};
use std::{
    fs,
    time::{Duration, UNIX_EPOCH},
};
use support::*;

#[test]
fn every_field_round_trips_across_close_and_reopen() {
    let dir = OwnedDirectory::new();
    let original = fixture("record' OR 1=1 --");
    {
        let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
        assert!(repo.list_ids().unwrap().is_empty());
        assert_eq!(repo.insert(&original).unwrap().value(), 0);
        assert_eq!(repo.load(original.id()).unwrap().unwrap().file(), &original);
    }
    let mut repo = SqliteRepository::open_existing(dir.db()).unwrap();
    let loaded = repo.load(original.id()).unwrap().unwrap();
    assert_eq!(loaded.file(), &original);
    assert_eq!(loaded.revision().value(), 0);
    assert_eq!(repo.list_ids().unwrap(), vec![original.id().clone()]);
    assert_eq!(repo.load(&id("missing")).unwrap(), None);
    eprintln!("bundled SQLite {}", rusqlite::version());
}

#[test]
fn create_open_and_ownership_are_explicit() {
    let dir = OwnedDirectory::new();
    assert!(matches!(
        SqliteRepository::open_existing(dir.db()),
        Err(Error::Unavailable)
    ));
    assert!(!dir.db().exists());
    let repo = SqliteRepository::create_new(dir.db()).unwrap();
    assert!(matches!(
        SqliteRepository::create_new(dir.db()),
        Err(Error::DatabaseAlreadyExists)
    ));
    drop(repo);
    let raw = dir.raw();
    assert_eq!(
        raw.pragma_query_value(None, "application_id", |r| r.get::<_, i64>(0))
            .unwrap(),
        0x464e594d
    );
    assert_eq!(
        raw.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    drop(raw);
    for sql in [
        "PRAGMA application_id=5",
        "PRAGMA user_version=99",
        "PRAGMA user_version=-1",
        "DROP TABLE aliases",
    ] {
        let isolated = OwnedDirectory::new();
        drop(SqliteRepository::create_new(isolated.db()).unwrap());
        let raw = isolated.raw();
        raw.execute_batch(sql).unwrap();
        drop(raw);
        assert!(
            matches!(
                SqliteRepository::open_existing(isolated.db()),
                Err(Error::IncompatibleDatabase)
            ),
            "{sql}"
        );
    }
}

#[test]
fn unrelated_empty_and_corrupt_databases_are_not_adopted() {
    for bytes in [vec![], b"not a SQLite file".to_vec()] {
        let dir = OwnedDirectory::new();
        fs::write(dir.db(), &bytes).unwrap();
        assert!(SqliteRepository::open_existing(dir.db()).is_err());
        assert_eq!(fs::read(dir.db()).unwrap(), bytes);
    }
    let dir = OwnedDirectory::new();
    let raw = dir.raw();
    raw.execute_batch("CREATE TABLE unrelated(secret TEXT); INSERT INTO unrelated VALUES('keep');")
        .unwrap();
    drop(raw);
    assert!(matches!(
        SqliteRepository::open_existing(dir.db()),
        Err(Error::IncompatibleDatabase)
    ));
    assert_eq!(
        dir.raw()
            .query_row("SELECT secret FROM unrelated", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "keep"
    );
}

#[test]
fn alias_edits_order_and_sibling_records_survive_updates() {
    let dir = OwnedDirectory::new();
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    let mut first = fixture("b");
    let sibling = fixture("a");
    let mut revision = repo.insert(&first).unwrap();
    repo.insert(&sibling).unwrap();
    assert_eq!(repo.list_ids().unwrap(), vec![id("a"), id("b")]);
    first
        .rename_alias(&alias_id("0"), name("Renamed.txt"), time(20))
        .unwrap();
    first.delete_alias(&alias_id("1")).unwrap();
    first
        .create_alias(alias_id("new"), name("New.alias"), time(21))
        .unwrap();
    revision = repo.update(&first, revision).unwrap();
    assert_eq!(revision.value(), 1);
    assert_eq!(repo.load(first.id()).unwrap().unwrap().file(), &first);
    assert_eq!(repo.load(sibling.id()).unwrap().unwrap().file(), &sibling);
    first
        .rename_alias(&alias_id("0"), name("RENAMED.TXT"), time(22))
        .unwrap();
    revision = repo.update(&first, revision).unwrap();
    let original = first.clone();
    first
        .rename_alias(&alias_id("0"), name("RENAMED.TXT"), time(30))
        .unwrap();
    assert_eq!(first, original);
    for alias in first.aliases().to_vec() {
        first.delete_alias(alias.id()).unwrap();
    }
    revision = repo.update(&first, revision).unwrap();
    assert_eq!(repo.load(first.id()).unwrap().unwrap().file(), &first);
    assert!(first.aliases().is_empty());
    assert_eq!(revision.value(), 3);
    assert_eq!(
        repo.update(&fixture("missing"), revision),
        Err(Error::NotFound)
    );
}

#[test]
fn all_lifecycle_states_round_trip_without_reclassification() {
    let dir = OwnedDirectory::new();
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    for (index, status) in [
        FileStatus::Available,
        FileStatus::Modified,
        FileStatus::PossibleReplacement,
        FileStatus::Unavailable(UnavailableReason::NotFound),
        FileStatus::Unavailable(UnavailableReason::PermissionDenied),
        FileStatus::Unavailable(UnavailableReason::Unknown),
    ]
    .into_iter()
    .enumerate()
    {
        let base = fixture(&index.to_string());
        let aliases = base
            .aliases()
            .iter()
            .map(|a| {
                AliasSnapshot::new(
                    a.id().clone(),
                    a.canonical_file_id().clone(),
                    a.name().clone(),
                    a.created_at(),
                    a.updated_at(),
                )
                .unwrap()
            })
            .collect();
        let file = CanonicalFile::restore(
            CanonicalFileSnapshot::new(base, status, UNIX_EPOCH - Duration::from_nanos(5), aliases)
                .unwrap(),
        )
        .unwrap();
        repo.insert(&file).unwrap();
        let mut loaded = repo.load(file.id()).unwrap().unwrap().into_file();
        assert_eq!(loaded, file);
        if status == FileStatus::PossibleReplacement {
            assert_eq!(
                loaded.observe(FileObservation::ConfirmedSameFile(
                    SameFileObservation::new(
                        loaded.location().clone(),
                        loaded.metadata().clone(),
                        loaded.identity().cloned(),
                        time(30)
                    )
                )),
                Err(DomainError::UnresolvedReplacement)
            );
        }
    }
    drop(repo);
    let mut repo = SqliteRepository::open_existing(dir.db()).unwrap();
    assert_eq!(repo.list_ids().unwrap().len(), 6);
    for index in 0..6 {
        assert!(repo.load(&id(&index.to_string())).unwrap().is_some());
    }
}

#[test]
fn revisions_prevent_lost_updates_and_preserve_immutable_facts() {
    let dir = OwnedDirectory::new();
    let mut first = SqliteRepository::create_new(dir.db()).unwrap();
    let file = fixture("one");
    first.insert(&file).unwrap();
    let mut second = SqliteRepository::open_existing(dir.db()).unwrap();
    let old = second.load(file.id()).unwrap().unwrap();
    let next = first.update(&file, old.revision()).unwrap();
    assert_eq!(second.update(&file, old.revision()), Err(Error::Conflict));
    assert_eq!(second.load(file.id()).unwrap().unwrap().revision(), next);
    assert_eq!(second.update(&file, next).unwrap().value(), 2);
    for altered in [
        CanonicalFile::new(
            file.id().clone(),
            file.location().clone(),
            None,
            file.metadata().clone(),
            file.registered_at(),
        ),
        CanonicalFile::new(
            file.id().clone(),
            file.location().clone(),
            Some(FileIdentity::new("different", vec![2]).unwrap()),
            file.metadata().clone(),
            file.registered_at(),
        ),
        CanonicalFile::new(
            file.id().clone(),
            file.location().clone(),
            file.identity().cloned(),
            file.metadata().clone(),
            time(100),
        ),
    ] {
        assert_eq!(
            second.update(&altered, PersistenceRevision::new(2).unwrap()),
            Err(Error::InvalidAggregateChange)
        );
    }
    assert_eq!(second.load(file.id()).unwrap().unwrap().file(), &file);
    let raw = dir.raw();
    raw.execute("UPDATE canonical_files SET revision=?1", [i64::MAX])
        .unwrap();
    drop(raw);
    assert_eq!(
        second.update(&file, PersistenceRevision::new(i64::MAX).unwrap()),
        Err(Error::UnrepresentableValue)
    );
    assert_eq!(
        second.load(file.id()).unwrap().unwrap().revision().value(),
        i64::MAX
    );
}

#[test]
fn duplicate_ids_are_safe_and_alias_id_scope_is_per_owner() {
    let dir = OwnedDirectory::new();
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    let file = fixture("one");
    repo.insert(&file).unwrap();
    assert_eq!(repo.insert(&file), Err(Error::DuplicateCanonicalId));
    assert_eq!(repo.load(file.id()).unwrap().unwrap().file(), &file);
    repo.insert(&fixture("two")).unwrap(); // Same alias IDs and paths are valid.
    let raw = dir.raw();
    assert!(raw
        .execute(
            "INSERT INTO aliases SELECT * FROM aliases WHERE canonical_file_id='one' AND id='0'",
            []
        )
        .is_err());
    assert!(raw
        .execute(
            "UPDATE aliases SET id='0' WHERE canonical_file_id='one' AND id='1'",
            []
        )
        .is_err());
    assert!(raw.execute("UPDATE aliases SET name_key=(SELECT name_key FROM aliases WHERE canonical_file_id='one' AND id='0') WHERE canonical_file_id='one' AND id='1'",[]).is_err());
    assert!(raw
        .execute(
            "UPDATE aliases SET canonical_file_id='missing' WHERE id='0'",
            []
        )
        .is_err());
    assert!(raw
        .execute("DELETE FROM canonical_files WHERE id='one'", [])
        .is_err());
    assert!(raw
        .execute("UPDATE canonical_files SET id='renamed' WHERE id='one'", [])
        .is_err());
}

#[test]
fn unsigned_sizes_and_nanosecond_times_preserve_exact_values() {
    let dir = OwnedDirectory::new();
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    for (index, size) in [
        0,
        i64::MAX.unsigned_abs(),
        i64::MAX.unsigned_abs() + 1,
        u64::MAX,
    ]
    .into_iter()
    .enumerate()
    {
        let file = CanonicalFile::new(
            id(&index.to_string()),
            KnownFileLocation::new("relative.txt").unwrap(),
            None,
            FileMetadata::new(size, Some(UNIX_EPOCH - Duration::from_millis(500))),
            UNIX_EPOCH + Duration::from_nanos(999_999_999),
        );
        repo.insert(&file).unwrap();
        assert_eq!(repo.load(file.id()).unwrap().unwrap().file(), &file);
    }
}

#[test]
fn persisted_operations_never_touch_the_physical_file() {
    let dir = OwnedDirectory::new();
    let sentinel = dir.path.join("sentinel.pdf");
    fs::write(&sentinel, b"only physical contents").unwrap();
    let before = fs::metadata(&sentinel).unwrap();
    let mut file = CanonicalFile::new(
        id("sentinel"),
        KnownFileLocation::new(&sentinel).unwrap(),
        None,
        FileMetadata::new(before.len(), None),
        time(1),
    );
    file.create_alias(alias_id("a"), name("Context.exe"), time(2))
        .unwrap();
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    let revision = repo.insert(&file).unwrap();
    file.rename_alias(&alias_id("a"), name("Other.txt"), time(3))
        .unwrap();
    let revision = repo.update(&file, revision).unwrap();
    file.delete_alias(&alias_id("a")).unwrap();
    repo.update(&file, revision).unwrap();
    assert_eq!(fs::read(&sentinel).unwrap(), b"only physical contents");
    let after = fs::metadata(&sentinel).unwrap();
    assert_eq!(before.len(), after.len());
    assert_eq!(before.modified().unwrap(), after.modified().unwrap());
    assert!(repo.load(file.id()).unwrap().is_some());
}

#[test]
fn lifecycle_changes_and_verified_recovery_persist_exact_facts() {
    let dir = OwnedDirectory::new();
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    let mut file = fixture("lifecycle");
    let aliases = file.aliases().to_vec();
    let mut revision = repo.insert(&file).unwrap();
    let same = SameFileObservation::new(
        KnownFileLocation::new("moved/renamed.pdf").unwrap(),
        FileMetadata::new(5, None),
        file.identity().cloned(),
        time(30),
    );
    file.observe(FileObservation::ConfirmedSameFile(same.clone()))
        .unwrap();
    revision = repo.update(&file, revision).unwrap();
    assert_eq!(repo.load(file.id()).unwrap().unwrap().file(), &file);
    for observation in [
        FileObservation::Unavailable {
            reason: UnavailableReason::PermissionDenied,
            observed_at: time(31),
        },
        FileObservation::PossibleReplacement {
            observed_at: time(32),
        },
    ] {
        file.observe(observation).unwrap();
        revision = repo.update(&file, revision).unwrap();
        let loaded = repo.load(file.id()).unwrap().unwrap();
        assert_eq!(loaded.file(), &file);
        assert_eq!(loaded.file().aliases(), aliases);
    }
    file = repo.load(file.id()).unwrap().unwrap().into_file();
    file.recover_verified_same_file(same).unwrap();
    repo.update(&file, revision).unwrap();
    assert_eq!(repo.load(file.id()).unwrap().unwrap().file(), &file);
    assert_eq!(file.aliases(), aliases);
}

#[test]
fn attaching_initial_evidence_and_byte_boundary_names_is_lossless() {
    let dir = OwnedDirectory::new();
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    let mut file = CanonicalFile::new(
        id("new"),
        KnownFileLocation::new("fixture").unwrap(),
        None,
        FileMetadata::new(0, None),
        time(1),
    );
    file.create_alias(alias_id("a"), name(&"a".repeat(255)), time(2))
        .unwrap();
    file.create_alias(
        alias_id("b"),
        name(&format!("{}a", "é".repeat(127))),
        time(2),
    )
    .unwrap();
    let revision = repo.insert(&file).unwrap();
    file.observe(FileObservation::ConfirmedSameFile(
        SameFileObservation::new(
            file.location().clone(),
            file.metadata().clone(),
            Some(FileIdentity::new("new", vec![0, 255]).unwrap()),
            time(3),
        ),
    ))
    .unwrap();
    repo.update(&file, revision).unwrap();
    assert_eq!(repo.load(file.id()).unwrap().unwrap().file(), &file);
}

#[test]
fn opaque_scheme_embedded_nuls_are_not_truncated() {
    let directory = OwnedDirectory::new();
    let mut repository = SqliteRepository::create_new(directory.db()).unwrap();
    for (index, scheme) in ["\0", "opaque\0scheme"].into_iter().enumerate() {
        let file = CanonicalFile::new(
            id(&index.to_string()),
            KnownFileLocation::new("fixture").unwrap(),
            Some(FileIdentity::new(scheme, vec![0, 1]).unwrap()),
            FileMetadata::new(0, None),
            time(1),
        );
        repository.insert(&file).unwrap();
        assert_eq!(repository.load(file.id()).unwrap().unwrap().file(), &file);
    }
    drop(repository);
    let mut repository = SqliteRepository::open_existing(directory.db()).unwrap();
    assert_eq!(
        repository
            .load(&id("0"))
            .unwrap()
            .unwrap()
            .file()
            .identity()
            .unwrap()
            .scheme(),
        "\0"
    );
}
