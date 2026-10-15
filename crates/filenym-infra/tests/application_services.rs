mod support;
use filenym_app::{
    AddAlias, ApplicationError as E, DeleteAlias, FileRegistryService, RegisterCanonicalFile,
    RenameAlias,
};
use filenym_core::*;
use filenym_infra::SqliteRepository;
use filenym_ports::*;
use std::{
    fs,
    time::{Duration, Instant, UNIX_EPOCH},
};
use support::*;

fn registration(value: &str) -> RegisterCanonicalFile {
    RegisterCanonicalFile {
        canonical_id: value.into(),
        location: KnownFileLocation::new("nonexistent/é.pdf").unwrap(),
        identity: Some(FileIdentity::new("opaque\0scheme", vec![0, 255, 1]).unwrap()),
        metadata: FileMetadata::new(u64::MAX, Some(UNIX_EPOCH - Duration::from_micros(123))),
        registered_at: time(10),
    }
}
fn add(stored: &StoredCanonicalFile, alias: &str, name: &str) -> AddAlias {
    AddAlias {
        canonical_id: stored.file().id().as_str().into(),
        expected_revision: stored.revision(),
        alias_id: alias.into(),
        name: name.into(),
        created_at: time(11),
    }
}
fn rename(stored: &StoredCanonicalFile, alias: &str, name: &str) -> RenameAlias {
    RenameAlias {
        canonical_id: stored.file().id().as_str().into(),
        expected_revision: stored.revision(),
        alias_id: alias.into(),
        name: name.into(),
        updated_at: time(12),
    }
}
fn delete(stored: &StoredCanonicalFile, alias: &str) -> DeleteAlias {
    DeleteAlias {
        canonical_id: stored.file().id().as_str().into(),
        expected_revision: stored.revision(),
        alias_id: alias.into(),
    }
}
fn assert_facts(actual: &CanonicalFile, expected: &CanonicalFile) {
    assert_eq!(actual.id(), expected.id());
    assert_eq!(actual.location(), expected.location());
    assert_eq!(actual.identity(), expected.identity());
    assert_eq!(actual.metadata(), expected.metadata());
    assert_eq!(actual.registered_at(), expected.registered_at());
    assert_eq!(actual.last_observed_at(), expected.last_observed_at());
    assert_eq!(actual.status(), expected.status());
}
fn mutate<R: CanonicalFileRepository>(
    service: &mut FileRegistryService<'_, R>,
    before: &StoredCanonicalFile,
    kind: u8,
) -> Result<StoredCanonicalFile, E> {
    match kind {
        0 => service.add_alias(add(before, "extra", "Extra.txt")),
        1 => service.rename_alias(rename(before, "0", "Changed.exe")),
        2 => service.delete_alias(delete(before, "0")),
        _ => service.rename_alias(rename(
            before,
            "0",
            before.file().aliases()[0].name().as_str(),
        )),
    }
}
fn expected_mutation(before: &StoredCanonicalFile, kind: u8) -> StoredCanonicalFile {
    let mut file = before.file().clone();
    match kind {
        0 => file
            .create_alias(alias_id("extra"), name("Extra.txt"), time(11))
            .unwrap(),
        1 => file
            .rename_alias(&alias_id("0"), name("Changed.exe"), time(12))
            .unwrap(),
        2 => file.delete_alias(&alias_id("0")).unwrap(),
        _ => file
            .rename_alias(
                &alias_id("0"),
                before.file().aliases()[0].name().clone(),
                time(12),
            )
            .unwrap(),
    }
    StoredCanonicalFile::new(file, before.revision().next().unwrap())
}

#[test]
fn complete_service_journey_preserves_exact_snapshots_across_reopens() {
    let dir = OwnedDirectory::new();
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    let (mut current, sibling) = {
        let mut service = FileRegistryService::new(&mut repo);
        assert!(service.list_registered_file_ids().unwrap().is_empty());
        let current = service
            .register_canonical_file(registration("A' OR 1=1 --"))
            .unwrap();
        let mut minimal = registration("B");
        minimal.identity = None;
        minimal.metadata = FileMetadata::new(0, None);
        let sibling = service.register_canonical_file(minimal).unwrap();
        assert_eq!(current.revision().value(), 0);
        assert!(current.file().aliases().is_empty());
        assert_eq!(current.file().status(), FileStatus::Available);
        assert_eq!(
            current.file().registered_at(),
            current.file().last_observed_at()
        );
        assert_eq!(
            service.list_registered_file_ids().unwrap(),
            vec![current.file().id().clone(), id("B")]
        );
        let mut duplicate = registration(current.file().id().as_str());
        duplicate.metadata = FileMetadata::new(42, None);
        assert_eq!(
            service.register_canonical_file(duplicate),
            Err(E::DuplicateCanonicalId)
        );
        assert_eq!(
            service
                .get_canonical_file(current.file().id().as_str())
                .unwrap(),
            current
        );
        (current, sibling)
    };
    for (alias, value) in [("0", "Report.PDF"), ("1", "Second.txt"), ("2", "Third.txt")] {
        let before = current.clone();
        current = FileRegistryService::new(&mut repo)
            .add_alias(add(&current, alias, value))
            .unwrap();
        let mut expected = before.file().clone();
        expected
            .create_alias(alias_id(alias), name(value), time(11))
            .unwrap();
        assert_eq!(
            current,
            StoredCanonicalFile::new(expected, before.revision().next().unwrap())
        );
        assert_eq!(
            FileRegistryService::new(&mut repo)
                .get_canonical_file(current.file().id().as_str())
                .unwrap(),
            current
        );
    }
    let first = current.file().aliases()[0].clone();
    let before = current.clone();
    current = FileRegistryService::new(&mut repo)
        .rename_alias(rename(&current, "0", "report.PDF"))
        .unwrap();
    let mut expected = before.file().clone();
    expected
        .rename_alias(&alias_id("0"), name("report.PDF"), time(12))
        .unwrap();
    assert_eq!(
        current,
        StoredCanonicalFile::new(expected, before.revision().next().unwrap())
    );
    assert_eq!(current.file().aliases()[0].id(), first.id());
    assert_eq!(current.file().aliases()[0].created_at(), first.created_at());
    let before = current.clone();
    current = FileRegistryService::new(&mut repo)
        .rename_alias(rename(&current, "0", "report.PDF"))
        .unwrap();
    assert_eq!(current.file(), before.file());
    assert_eq!(current.revision(), before.revision().next().unwrap());
    {
        let mut service = FileRegistryService::new(&mut repo);
        assert_eq!(
            service.add_alias(add(&current, "x", "REPORT.pdf")),
            Err(E::DuplicateAliasName)
        );
        assert_eq!(
            service.add_alias(add(&current, "0", "unique.txt")),
            Err(E::DuplicateAliasId)
        );
        assert!(matches!(
            service.add_alias(add(&current, "x", "../bad")),
            Err(E::Domain(DomainError::InvalidAliasName(_)))
        ));
        assert_eq!(service.delete_alias(delete(&before, "1")), Err(E::Conflict));
        assert_eq!(
            service
                .get_canonical_file(current.file().id().as_str())
                .unwrap(),
            current
        );
    }
    let before = current.clone();
    current = FileRegistryService::new(&mut repo)
        .delete_alias(delete(&current, "1"))
        .unwrap();
    let mut expected = before.file().clone();
    expected.delete_alias(&alias_id("1")).unwrap();
    assert_eq!(
        current,
        StoredCanonicalFile::new(expected, before.revision().next().unwrap())
    );
    drop(repo);
    let mut repo = SqliteRepository::open_existing(dir.db()).unwrap();
    assert_eq!(
        FileRegistryService::new(&mut repo)
            .get_canonical_file(current.file().id().as_str())
            .unwrap(),
        current
    );
    assert_eq!(
        FileRegistryService::new(&mut repo)
            .get_canonical_file("B")
            .unwrap(),
        sibling
    );
    for alias in ["0", "2"] {
        let before = current.clone();
        current = FileRegistryService::new(&mut repo)
            .delete_alias(delete(&current, alias))
            .unwrap();
        let mut expected = before.file().clone();
        expected.delete_alias(&alias_id(alias)).unwrap();
        assert_eq!(
            current,
            StoredCanonicalFile::new(expected, before.revision().next().unwrap())
        );
    }
    assert!(current.file().aliases().is_empty());
    assert_facts(current.file(), before.file());
    assert_eq!(
        FileRegistryService::new(&mut repo).delete_alias(delete(&current, "2")),
        Err(E::AliasNotFound)
    );
    drop(repo);
    let mut repo = SqliteRepository::open_existing(dir.db()).unwrap();
    assert_eq!(
        FileRegistryService::new(&mut repo)
            .get_canonical_file(current.file().id().as_str())
            .unwrap(),
        current
    );
    assert_eq!(
        FileRegistryService::new(&mut repo)
            .get_canonical_file("B")
            .unwrap(),
        sibling
    );
}

#[test]
fn supplied_registration_facts_require_no_filesystem_resolution() {
    let dir = OwnedDirectory::new();
    let missing = dir.path.join("does-not-exist.pdf");
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    let mut request = registration("first");
    request.location = KnownFileLocation::new(&missing).unwrap();
    request.registered_at = UNIX_EPOCH - Duration::from_micros(123);
    let expected = CanonicalFile::new(
        id("first"),
        request.location.clone(),
        request.identity.clone(),
        request.metadata.clone(),
        request.registered_at,
    );
    let first = FileRegistryService::new(&mut repo)
        .register_canonical_file(request.clone())
        .unwrap();
    assert_eq!(first.file(), &expected);
    assert_eq!(first.revision().value(), 0);
    request.canonical_id = "second".into();
    let second = FileRegistryService::new(&mut repo)
        .register_canonical_file(request)
        .unwrap();
    assert_ne!(first.file().id(), second.file().id());
    assert_eq!(first.file().location(), second.file().location());
    assert_eq!(first.file().identity(), second.file().identity());
    assert!(!missing.exists());
    drop(repo);
    let mut repo = SqliteRepository::open_existing(dir.db()).unwrap();
    let mut service = FileRegistryService::new(&mut repo);
    assert_eq!(service.get_canonical_file("first").unwrap(), first);
    assert_eq!(service.get_canonical_file("second").unwrap(), second);
    assert!(!missing.exists());
}

#[test]
fn unicode_name_boundaries_and_owner_scope_round_trip_through_services() {
    let dir = OwnedDirectory::new();
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    let mut service = FileRegistryService::new(&mut repo);
    let mut current = service
        .register_canonical_file(registration("one"))
        .unwrap();
    let sibling = service
        .register_canonical_file(registration("two"))
        .unwrap();
    let values = [
        "É.pdf".into(),
        "é.pdf".into(),
        "e\u{301}.pdf".into(),
        ".dotfile".into(),
        "README".into(),
        "internal space.tar.gz".into(),
        "report..pdf".into(),
        "explicit.exe".into(),
        "a".repeat(255),
        format!("{}a", "é".repeat(127)),
        "O'Brien.txt".into(),
    ];
    for (i, value) in values.iter().enumerate() {
        current = service
            .add_alias(add(&current, &i.to_string(), value))
            .unwrap();
    }
    let other = service.add_alias(add(&sibling, "0", "É.pdf")).unwrap();
    assert_eq!(other.file().aliases()[0].name().as_str(), "É.pdf");
    for value in [
        "a".repeat(256),
        "é".repeat(128),
        "CON.pdf".into(),
        " a".into(),
        "a\u{2003}".into(),
        "a:".into(),
        "a\0".into(),
    ] {
        assert!(matches!(
            service.add_alias(add(&current, "invalid", &value)),
            Err(E::Domain(DomainError::InvalidAliasName(_)))
        ));
        assert!(matches!(
            service.rename_alias(rename(&current, "0", &value)),
            Err(E::Domain(DomainError::InvalidAliasName(_)))
        ));
    }
    drop(repo);
    let mut repo = SqliteRepository::open_existing(dir.db()).unwrap();
    let loaded = FileRegistryService::new(&mut repo)
        .get_canonical_file("one")
        .unwrap();
    assert_eq!(loaded, current);
    assert_eq!(
        loaded
            .file()
            .aliases()
            .iter()
            .map(|a| a.name().as_str())
            .collect::<Vec<_>>(),
        values.iter().map(String::as_str).collect::<Vec<_>>()
    );
}

#[test]
fn timestamp_boundaries_missing_records_and_metadata_are_preserved() {
    let dir = OwnedDirectory::new();
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    let mut service = FileRegistryService::new(&mut repo);
    for kind in 0..3 {
        let missing =
            StoredCanonicalFile::new(fixture("missing"), PersistenceRevision::new(0).unwrap());
        assert_eq!(
            mutate(&mut service, &missing, kind),
            Err(E::CanonicalNotFound)
        );
    }
    assert_eq!(
        service.get_canonical_file("missing"),
        Err(E::CanonicalNotFound)
    );
    let current = service
        .register_canonical_file(registration("one"))
        .unwrap();
    let mut request = add(&current, "0", "Report.PDF");
    request.created_at = time(9);
    assert_eq!(
        service.add_alias(request.clone()),
        Err(E::Domain(DomainError::InvalidTimestamp))
    );
    request.created_at = current.file().registered_at();
    let current = service.add_alias(request).unwrap();
    let mut r = rename(&current, "0", "Changed.txt");
    r.updated_at = time(30);
    let later = service.rename_alias(r.clone()).unwrap();
    r.expected_revision = later.revision();
    r.updated_at = current.file().registered_at();
    r.name = "Earlier.txt".into();
    let earlier = service.rename_alias(r.clone()).unwrap();
    assert_eq!(
        earlier.file().aliases()[0].updated_at(),
        current.file().registered_at()
    );
    assert_facts(earlier.file(), current.file());
    r.expected_revision = earlier.revision();
    r.updated_at = time(9);
    r.name = "Earlier.txt".into();
    assert_eq!(
        service.rename_alias(r),
        Err(E::Domain(DomainError::InvalidTimestamp))
    );
    assert_eq!(service.get_canonical_file("one").unwrap(), earlier);
}

#[test]
fn every_persisted_lifecycle_state_survives_all_alias_commands_and_reopen() {
    let dir = OwnedDirectory::new();
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    let statuses = [
        FileStatus::Available,
        FileStatus::Modified,
        FileStatus::Unavailable(UnavailableReason::NotFound),
        FileStatus::Unavailable(UnavailableReason::PermissionDenied),
        FileStatus::Unavailable(UnavailableReason::Unknown),
        FileStatus::PossibleReplacement,
    ];
    let mut expected = Vec::new();
    for (i, status) in statuses.into_iter().enumerate() {
        let file = fixture(&i.to_string());
        let snapshot = CanonicalFileSnapshot::new(file, status, time(1), vec![]).unwrap();
        let file = CanonicalFile::restore(snapshot).unwrap();
        repo.insert(&file).unwrap();
        let mut service = FileRegistryService::new(&mut repo);
        let initial = service.get_canonical_file(&i.to_string()).unwrap();
        let added = service.add_alias(add(&initial, "0", "First.pdf")).unwrap();
        let renamed = service
            .rename_alias(rename(&added, "0", "Explicit.exe"))
            .unwrap();
        let deleted = service.delete_alias(delete(&renamed, "0")).unwrap();
        assert_facts(deleted.file(), initial.file());
        assert_eq!(deleted.file(), initial.file());
        expected.push(deleted);
    }
    assert_eq!(
        FileRegistryService::new(&mut repo)
            .list_registered_file_ids()
            .unwrap(),
        (0..6).map(|i| id(&i.to_string())).collect::<Vec<_>>()
    );
    drop(repo);
    let mut repo = SqliteRepository::open_existing(dir.db()).unwrap();
    for stored in expected {
        assert_eq!(
            FileRegistryService::new(&mut repo)
                .get_canonical_file(stored.file().id().as_str())
                .unwrap(),
            stored
        );
    }
}

struct InterveningWriter {
    first: SqliteRepository,
    second: SqliteRepository,
    fired: bool,
}
impl CanonicalFileRepository for InterveningWriter {
    fn insert(&mut self, f: &CanonicalFile) -> Result<PersistenceRevision, PersistenceError> {
        self.first.insert(f)
    }
    fn load(
        &mut self,
        id: &CanonicalFileId,
    ) -> Result<Option<StoredCanonicalFile>, PersistenceError> {
        self.first.load(id)
    }
    fn list_ids(&mut self) -> Result<Vec<CanonicalFileId>, PersistenceError> {
        self.first.list_ids()
    }
    fn update(
        &mut self,
        f: &CanonicalFile,
        expected: PersistenceRevision,
    ) -> Result<PersistenceRevision, PersistenceError> {
        assert!(!self.fired, "must never retry");
        self.fired = true;
        let old = self.second.load(f.id())?.unwrap();
        let mut winning = old.file().clone();
        winning
            .create_alias(alias_id("winner"), name("Winner.pdf"), time(30))
            .unwrap();
        self.second.update(&winning, old.revision())?;
        self.first.update(f, expected)
    }
}

#[test]
fn caller_staleness_and_intervening_sqlite_writers_conflict_without_retries() {
    for kind in 0..4 {
        let dir = OwnedDirectory::new();
        let mut first = SqliteRepository::create_new(dir.db()).unwrap();
        first.insert(&fixture("one")).unwrap();
        let mut second = SqliteRepository::open_existing(dir.db()).unwrap();
        let old = FileRegistryService::new(&mut first)
            .get_canonical_file("one")
            .unwrap();
        let winning = FileRegistryService::new(&mut second)
            .add_alias(add(&old, "winner", "Winner.pdf"))
            .unwrap();
        assert_eq!(
            mutate(&mut FileRegistryService::new(&mut first), &old, kind),
            Err(E::Conflict)
        );
        let mut future = add(&winning, "future", "Future.pdf");
        future.expected_revision = PersistenceRevision::new(100).unwrap();
        assert_eq!(
            FileRegistryService::new(&mut first).add_alias(future),
            Err(E::Conflict)
        );
        let loaded = FileRegistryService::new(&mut first)
            .get_canonical_file("one")
            .unwrap();
        assert_eq!(loaded, winning);
        let expected = expected_mutation(&loaded, kind);
        assert_eq!(
            mutate(&mut FileRegistryService::new(&mut first), &loaded, kind).unwrap(),
            expected
        );
        let mut raced = InterveningWriter {
            first,
            second,
            fired: false,
        };
        let before = FileRegistryService::new(&mut raced)
            .get_canonical_file("one")
            .unwrap();
        // Use a fresh owner so the intervening winner ID is unique in this race.
        raced.first.insert(&fixture("race")).unwrap();
        let before_race = FileRegistryService::new(&mut raced)
            .get_canonical_file("race")
            .unwrap();
        assert_eq!(
            mutate(
                &mut FileRegistryService::new(&mut raced),
                &before_race,
                kind
            ),
            Err(E::Conflict)
        );
        assert!(raced.fired);
        let actual = FileRegistryService::new(&mut raced)
            .get_canonical_file("race")
            .unwrap();
        let mut expected = before_race.file().clone();
        expected
            .create_alias(alias_id("winner"), name("Winner.pdf"), time(30))
            .unwrap();
        assert_eq!(
            actual,
            StoredCanonicalFile::new(expected, before_race.revision().next().unwrap())
        );
        assert_eq!(
            FileRegistryService::new(&mut raced)
                .get_canonical_file("one")
                .unwrap(),
            before
        );
    }
}

#[test]
fn write_stage_failures_rollback_service_commands_and_allow_explicit_retry() {
    let update_triggers = [
        "CREATE TRIGGER fail AFTER UPDATE ON canonical_files BEGIN SELECT RAISE(ABORT,'fixture'); END;",
        "CREATE TRIGGER fail AFTER DELETE ON aliases BEGIN SELECT RAISE(ABORT,'fixture'); END;",
        "CREATE TRIGGER fail AFTER INSERT ON aliases BEGIN SELECT RAISE(ABORT,'fixture'); END;",
    ];
    for trigger in update_triggers {
        for kind in 0..4 {
            let dir = OwnedDirectory::new();
            let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
            repo.insert(&fixture("one")).unwrap();
            repo.insert(&fixture("sibling")).unwrap();
            let before = FileRegistryService::new(&mut repo)
                .get_canonical_file("one")
                .unwrap();
            let sibling = FileRegistryService::new(&mut repo)
                .get_canonical_file("sibling")
                .unwrap();
            let raw = dir.raw();
            raw.execute_batch(trigger).unwrap();
            assert_eq!(
                mutate(&mut FileRegistryService::new(&mut repo), &before, kind),
                Err(E::Storage(PersistenceError::InvalidStoredData))
            );
            assert_eq!(
                FileRegistryService::new(&mut repo)
                    .get_canonical_file("one")
                    .unwrap(),
                before
            );
            raw.execute_batch("DROP TRIGGER fail").unwrap();
            drop(raw);
            drop(repo);
            let mut repo = SqliteRepository::open_existing(dir.db()).unwrap();
            let fresh = FileRegistryService::new(&mut repo)
                .get_canonical_file("one")
                .unwrap();
            assert_eq!(fresh, before);
            assert_eq!(
                mutate(&mut FileRegistryService::new(&mut repo), &fresh, kind).unwrap(),
                expected_mutation(&fresh, kind)
            );
            assert_eq!(
                FileRegistryService::new(&mut repo)
                    .get_canonical_file("sibling")
                    .unwrap(),
                sibling
            );
        }
    }
    let insert_triggers = [
        "CREATE TRIGGER fail AFTER INSERT ON canonical_files BEGIN SELECT RAISE(ABORT,'fixture'); END;",
        "CREATE TRIGGER fail BEFORE INSERT ON canonical_files BEGIN SELECT RAISE(ABORT,'fixture'); END;",
    ];
    for trigger in insert_triggers {
        let dir = OwnedDirectory::new();
        let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
        let raw = dir.raw();
        raw.execute_batch(trigger).unwrap();
        assert_eq!(
            FileRegistryService::new(&mut repo).register_canonical_file(registration("one")),
            Err(E::Storage(PersistenceError::InvalidStoredData))
        );
        assert!(FileRegistryService::new(&mut repo)
            .list_registered_file_ids()
            .unwrap()
            .is_empty());
        raw.execute_batch("DROP TRIGGER fail").unwrap();
        drop(raw);
        drop(repo);
        let mut repo = SqliteRepository::open_existing(dir.db()).unwrap();
        assert_eq!(
            FileRegistryService::new(&mut repo).get_canonical_file("one"),
            Err(E::CanonicalNotFound)
        );
    }
}

#[test]
fn commit_failure_never_returns_success_and_preserves_previous_snapshot() {
    let dir = OwnedDirectory::new();
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    repo.insert(&fixture("one")).unwrap();
    repo.insert(&fixture("sibling")).unwrap();
    let sibling = FileRegistryService::new(&mut repo)
        .get_canonical_file("sibling")
        .unwrap();
    let before = FileRegistryService::new(&mut repo)
        .get_canonical_file("one")
        .unwrap();
    let raw = dir.raw();
    raw.execute_batch("CREATE TABLE fixture_parent(id INTEGER PRIMARY KEY); CREATE TABLE fixture_child(id INTEGER REFERENCES fixture_parent(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail AFTER INSERT ON aliases BEGIN INSERT INTO fixture_child VALUES(999); END;").unwrap();
    for kind in 0..4 {
        assert_eq!(
            mutate(&mut FileRegistryService::new(&mut repo), &before, kind),
            Err(E::Storage(PersistenceError::InvalidStoredData))
        );
        assert_eq!(
            FileRegistryService::new(&mut repo)
                .get_canonical_file("one")
                .unwrap(),
            before
        );
        assert_eq!(
            FileRegistryService::new(&mut repo)
                .get_canonical_file("sibling")
                .unwrap(),
            sibling
        );
    }
    raw.execute_batch("DROP TRIGGER fail; DROP TABLE fixture_child; DROP TABLE fixture_parent;")
        .unwrap();
    drop(raw);
    drop(repo);
    let mut repo = SqliteRepository::open_existing(dir.db()).unwrap();
    assert_eq!(
        FileRegistryService::new(&mut repo)
            .get_canonical_file("one")
            .unwrap(),
        before
    );
    assert_eq!(
        FileRegistryService::new(&mut repo)
            .get_canonical_file("sibling")
            .unwrap(),
        sibling
    );
}

#[test]
fn service_writes_obey_bounded_locking_and_explicit_reload_retry() {
    for kind in 0..5 {
        let dir = OwnedDirectory::new();
        let mut repo =
            SqliteRepository::create_new_with_timeout(dir.db(), Duration::from_millis(25)).unwrap();
        repo.insert(&fixture("one")).unwrap();
        let before = FileRegistryService::new(&mut repo)
            .get_canonical_file("one")
            .unwrap();
        let raw = dir.raw();
        raw.execute_batch("BEGIN IMMEDIATE").unwrap();
        let started = Instant::now();
        let actual = if kind == 4 {
            FileRegistryService::new(&mut repo).register_canonical_file(registration("new"))
        } else {
            mutate(&mut FileRegistryService::new(&mut repo), &before, kind)
        };
        assert_eq!(actual, Err(E::Storage(PersistenceError::Busy)));
        assert!(started.elapsed() < Duration::from_secs(3));
        raw.execute_batch("ROLLBACK").unwrap();
        drop(raw);
        let fresh = FileRegistryService::new(&mut repo)
            .get_canonical_file("one")
            .unwrap();
        assert_eq!(fresh, before);
        if kind == 4 {
            assert_eq!(
                FileRegistryService::new(&mut repo)
                    .register_canonical_file(registration("new"))
                    .unwrap()
                    .revision()
                    .value(),
                0
            );
        } else {
            assert_eq!(
                mutate(&mut FileRegistryService::new(&mut repo), &fresh, kind).unwrap(),
                expected_mutation(&fresh, kind)
            );
        }
    }
}

#[test]
fn invalid_stored_data_inventory_and_revision_exhaustion_are_never_repaired() {
    for sql in [
        "UPDATE aliases SET name='CON.pdf' WHERE id='0'",
        "UPDATE aliases SET name_key=X'01' WHERE id='0'",
        "UPDATE aliases SET position=100 WHERE id='7'",
        "UPDATE aliases SET created_seconds=0 WHERE id='0'",
    ] {
        let dir = OwnedDirectory::new();
        let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
        repo.insert(&fixture("one")).unwrap();
        let before = repo.load(&id("one")).unwrap().unwrap();
        let raw = dir.raw();
        raw.execute_batch(sql).unwrap();
        let mut service = FileRegistryService::new(&mut repo);
        assert_eq!(service.list_registered_file_ids().unwrap(), vec![id("one")]);
        assert_eq!(
            service.get_canonical_file("one"),
            Err(E::Storage(PersistenceError::InvalidStoredData))
        );
        for kind in 0..4 {
            assert_eq!(
                mutate(&mut service, &before, kind),
                Err(E::Storage(PersistenceError::InvalidStoredData))
            );
        }
        assert_eq!(
            raw.query_row("SELECT revision FROM canonical_files", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    let dir = OwnedDirectory::new();
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    repo.insert(&fixture("one")).unwrap();
    let raw = dir.raw();
    raw.execute_batch("UPDATE canonical_files SET revision=9223372036854775807")
        .unwrap();
    let before = FileRegistryService::new(&mut repo)
        .get_canonical_file("one")
        .unwrap();
    for kind in 0..4 {
        assert_eq!(
            mutate(&mut FileRegistryService::new(&mut repo), &before, kind),
            Err(E::Storage(PersistenceError::UnrepresentableValue))
        );
        assert_eq!(
            FileRegistryService::new(&mut repo)
                .get_canonical_file("one")
                .unwrap(),
            before
        );
    }
    raw.execute_batch("PRAGMA foreign_keys=OFF; UPDATE canonical_files SET id=' bad '")
        .unwrap();
    assert_eq!(
        FileRegistryService::new(&mut repo).list_registered_file_ids(),
        Err(E::Storage(PersistenceError::InvalidStoredData))
    );
}

#[test]
fn service_alias_journey_never_changes_owned_physical_files() {
    let dir = OwnedDirectory::new();
    let path = dir.path.join("canonical.pdf");
    let sibling = dir.path.join("sibling.txt");
    fs::write(&path, b"canonical sentinel").unwrap();
    fs::write(&sibling, b"sibling sentinel").unwrap();
    let sibling_metadata = fs::metadata(&sibling).unwrap();
    let metadata = fs::metadata(&path).unwrap();
    let modified = metadata.modified().unwrap();
    let mut repo = SqliteRepository::create_new(dir.db()).unwrap();
    let mut request = registration("one");
    request.location = KnownFileLocation::new(&path).unwrap();
    let mut current = FileRegistryService::new(&mut repo)
        .register_canonical_file(request)
        .unwrap();
    current = FileRegistryService::new(&mut repo)
        .add_alias(add(&current, "0", "Logical.pdf"))
        .unwrap();
    current = FileRegistryService::new(&mut repo)
        .rename_alias(rename(&current, "0", "Context.exe"))
        .unwrap();
    assert_eq!(
        FileRegistryService::new(&mut repo).add_alias(add(&current, "1", "CON")),
        Err(E::Domain(DomainError::InvalidAliasName(
            AliasNameError::DeviceName
        )))
    );
    current = FileRegistryService::new(&mut repo)
        .delete_alias(delete(&current, "0"))
        .unwrap();
    drop(repo);
    let mut repo = SqliteRepository::open_existing(dir.db()).unwrap();
    assert_eq!(
        FileRegistryService::new(&mut repo)
            .get_canonical_file("one")
            .unwrap(),
        current
    );
    assert_eq!(fs::read(&path).unwrap(), b"canonical sentinel");
    assert_eq!(fs::read(&sibling).unwrap(), b"sibling sentinel");
    let sibling_after = fs::metadata(&sibling).unwrap();
    assert_eq!(sibling_after.len(), sibling_metadata.len());
    assert_eq!(
        sibling_after.modified().unwrap(),
        sibling_metadata.modified().unwrap()
    );
    let after = fs::metadata(&path).unwrap();
    assert_eq!(after.len(), metadata.len());
    assert_eq!(after.modified().unwrap(), modified);
    let mut entries: Vec<_> = fs::read_dir(&dir.path)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    entries.sort();
    for alias in ["Logical.pdf", "Context.exe", "CON"] {
        assert!(!entries.contains(&alias.into()));
    }
    assert!(current.file().aliases().is_empty());
}
