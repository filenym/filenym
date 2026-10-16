#![cfg(windows)]
use super::*;
use std::{
    fs,
    path::PathBuf,
    time::{Duration, UNIX_EPOCH},
};
use transport::*;
struct Profile {
    root: PathBuf,
    parent: PathBuf,
}
impl Profile {
    fn new() -> Self {
        let parent = std::env::temp_dir().canonicalize().unwrap();
        let root = parent.join(format!("filenym-desktop-test-{}", ids::new_id().unwrap()));
        fs::create_dir(&root).unwrap();
        Self { root, parent }
    }
    fn state(&self) -> state::DesktopState {
        state::DesktopState::new(Some(self.root.join("registry")))
    }
}
impl Drop for Profile {
    fn drop(&mut self) {
        let actual = self.root.canonicalize().unwrap();
        assert_eq!(actual.parent(), Some(self.parent.as_path()));
        assert_eq!(actual, self.root);
        fs::remove_dir_all(actual).unwrap();
    }
}
fn epoch(s: &state::DesktopState) -> String {
    s.initialize(false).unwrap().storage_epoch.unwrap()
}
fn selected(s: &state::DesktopState, e: &str, path: PathBuf) -> CanonicalFileDto {
    match selection::selected(s, e.into(), path).unwrap().value {
        SelectionOutcomeDto::Added { file } => *file,
        _ => panic!("expected ordinary selection"),
    }
}
#[test]
fn bootstrap_preserves_marker_and_never_recreates_missing_database() {
    let p = Profile::new();
    let s = p.state();
    let old = epoch(&s);
    drop(s);
    let root = p.root.join("registry");
    let marker = fs::read(root.join("registry-initialization-v1")).unwrap();
    let s = p.state();
    assert_ne!(epoch(&s), old);
    drop(s);
    assert_eq!(
        fs::read(root.join("registry-initialization-v1")).unwrap(),
        marker
    );
    fs::remove_file(root.join("filenym.sqlite3")).unwrap();
    let s = p.state();
    assert_eq!(
        s.initialize(false).unwrap_err().code,
        Code::DatabaseUnavailable
    );
    assert!(!root.join("filenym.sqlite3").exists());
    drop(s);
}
#[test]
fn interrupted_invalid_and_existing_unmarked_initialization_are_safe() {
    for bytes in [
        b"".as_slice(),
        b"bad",
        b"FileNym registry initialization v1\n",
    ] {
        let p = Profile::new();
        let root = p.root.join("registry");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("registry-initialization-v1"), bytes).unwrap();
        assert!(storage::open(&root, true).is_err());
        assert_eq!(
            fs::read(root.join("registry-initialization-v1")).unwrap(),
            bytes
        );
        assert!(!root.join("filenym.sqlite3").exists());
    }
    let p = Profile::new();
    let root = p.root.join("registry");
    fs::create_dir(&root).unwrap();
    drop(filenym_infra::SqliteRepository::create_new(root.join("filenym.sqlite3")).unwrap());
    drop(storage::open(&root, true).unwrap());
    assert!(root.join("registry-initialization-v1").exists());
}
#[test]
fn complete_registry_journey_conflicts_duplicates_restart_and_file_safety() {
    let p = Profile::new();
    let fixture = p.root.join("résumé.txt");
    fs::write(&fixture, b"original sentinel").unwrap();
    let before = fs::metadata(&fixture).unwrap().modified().unwrap();
    let s = p.state();
    let e = epoch(&s);
    let f = selected(&s, &e, fixture.clone());
    assert_eq!(f.revision, "0");
    let a = registry::add(
        &s,
        AddAliasRequestDto {
            storage_epoch: e.clone(),
            canonical_id: f.id.clone(),
            expected_revision: f.revision.clone(),
            name: "First.PDF".into(),
        },
    )
    .unwrap()
    .value;
    assert_eq!(a.revision, "1");
    assert_eq!(
        registry::add(
            &s,
            AddAliasRequestDto {
                storage_epoch: e.clone(),
                canonical_id: f.id.clone(),
                expected_revision: f.revision.clone(),
                name: "Other".into()
            }
        )
        .unwrap_err()
        .code,
        Code::Conflict
    );
    assert_eq!(
        registry::add(
            &s,
            AddAliasRequestDto {
                storage_epoch: e.clone(),
                canonical_id: f.id.clone(),
                expected_revision: a.revision.clone(),
                name: "first.pdf".into()
            }
        )
        .unwrap_err()
        .code,
        Code::DuplicateAliasName
    );
    let a = registry::rename(
        &s,
        RenameAliasRequestDto {
            storage_epoch: e.clone(),
            canonical_id: f.id.clone(),
            expected_revision: a.revision.clone(),
            alias_id: a.aliases[0].id.clone(),
            name: "First.PDF".into(),
        },
    )
    .unwrap()
    .value;
    assert_eq!(a.revision, "2");
    let d = selection::selected(&s, e.clone(), fixture.clone())
        .unwrap()
        .value;
    let token = match d {
        SelectionOutcomeDto::DuplicateDecision {
            selection_token, ..
        } => selection_token,
        _ => panic!(),
    };
    assert_eq!(
        selection::decide(
            &s,
            RegisterSelectionRequestDto {
                storage_epoch: e.clone(),
                selection_token: "forged".into(),
                decision: Decision::AddSeparately,
                candidate_id: None
            }
        )
        .unwrap_err()
        .code,
        Code::SelectionInvalid
    );
    assert!(matches!(
        selection::decide(
            &s,
            RegisterSelectionRequestDto {
                storage_epoch: e.clone(),
                selection_token: token.clone(),
                decision: Decision::ViewExisting,
                candidate_id: Some(f.id.clone())
            }
        )
        .unwrap()
        .value,
        DuplicateDecisionOutcomeDto::ViewExisting { .. }
    ));
    assert!(selection::decide(
        &s,
        RegisterSelectionRequestDto {
            storage_epoch: e.clone(),
            selection_token: token,
            decision: Decision::AddSeparately,
            candidate_id: None
        }
    )
    .is_err());
    let deleted = registry::delete(
        &s,
        DeleteAliasRequestDto {
            storage_epoch: e.clone(),
            canonical_id: f.id.clone(),
            expected_revision: a.revision.clone(),
            alias_id: a.aliases[0].id.clone(),
        },
    )
    .unwrap()
    .value;
    assert!(deleted.aliases.is_empty());
    drop(s);
    let s = p.state();
    let new = epoch(&s);
    assert_eq!(
        registry::get(
            &s,
            GetFileRequestDto {
                storage_epoch: e,
                canonical_id: f.id.clone()
            }
        )
        .unwrap_err()
        .code,
        Code::StaleSession
    );
    assert_eq!(
        registry::get(
            &s,
            GetFileRequestDto {
                storage_epoch: new,
                canonical_id: f.id
            }
        )
        .unwrap()
        .value,
        deleted
    );
    drop(s);
    assert_eq!(fs::read(&fixture).unwrap(), b"original sentinel");
    assert_eq!(fs::metadata(&fixture).unwrap().modified().unwrap(), before);
    assert!(!p.root.join("First.PDF").exists());
}
#[test]
fn uncertainty_retires_adapter_and_requires_new_epoch_and_inventory() {
    let p = Profile::new();
    let s = p.state();
    let e = epoch(&s);
    s.with(|session| {
        assert_eq!(
            s.finish::<()>(
                session,
                Err(CommandErrorDto::new(Code::UncertainOutcome)),
                Some(Attempted {
                    canonical_id: Some("attempted".into()),
                    alias_id: None
                })
            )
            .unwrap_err()
            .code,
            Code::UncertainOutcome
        );
        assert!(session.repository.is_none());
        Ok(())
    })
    .unwrap();
    let reopened = s.initialize(true).unwrap();
    assert_eq!(reopened.value.phase, Phase::NeedsReload);
    let next = reopened.storage_epoch.unwrap();
    assert_ne!(next, e);
    assert_eq!(
        registry::list(&s, SessionRequestDto { storage_epoch: e })
            .unwrap_err()
            .code,
        Code::StaleSession
    );
    s.with(|session| {
        assert_eq!(
            s.check(session, &next, true).unwrap_err().code,
            Code::UncertainOutcome
        );
        Ok(())
    })
    .unwrap();
    registry::list(
        &s,
        SessionRequestDto {
            storage_epoch: next.clone(),
        },
    )
    .unwrap();
    s.with(|session| s.check(session, &next, true)).unwrap();
}
#[test]
fn poison_blocks_work_and_reopen_requires_inventory() {
    let p = Profile::new();
    let s = p.state();
    epoch(&s);
    let copy = s.clone();
    assert!(std::thread::spawn(move || {
        let _guard = copy.inner.lock().unwrap();
        panic!("simulated worker panic");
    })
    .join()
    .is_err());
    assert_eq!(s.with(|_| Ok(())).unwrap_err().code, Code::UncertainOutcome);
    assert_eq!(s.initialize(true).unwrap().value.phase, Phase::NeedsReload);
}
#[test]
fn exact_transport_rejects_forbidden_requests_and_preserves_native_units() {
    for value in ["-1", "01", "1e2", "9223372036854775808", ""] {
        assert!(revision(value).is_err());
    }
    assert_eq!(revision("0").unwrap().value(), 0);
    assert!(revision("9223372036854775807").is_ok());
    assert!(serde_json::from_value::<AddAliasRequestDto>(serde_json::json!({"storageEpoch":"epoch","canonicalId":"id","expectedRevision":"0","name":"a","aliasId":"caller"})).is_err());
    assert_eq!(
        timestamp(UNIX_EPOCH - Duration::from_nanos(100)).unwrap(),
        TimestampDto {
            seconds: "-1".into(),
            nanoseconds: 999999900
        }
    );
    use std::{ffi::OsString, os::windows::ffi::OsStringExt};
    let native = NativeTextDto::new(&OsString::from_wide(&[65, 0xd800, 10])).unwrap();
    assert_eq!(native.units, vec![65, 0xd800, 10]);
    assert!(native.escaped);
    assert_eq!(
        serde_json::to_value(native).unwrap()["encoding"],
        "windowsUtf16"
    );
    let json = serde_json::to_string(&CommandErrorDto::from(
        filenym_ports::PersistenceError::CorruptDatabase,
    ))
    .unwrap();
    assert!(!json.contains("sqlite"));
    assert!(json.contains("corruptDatabase"));
}
#[test]
fn concurrent_initialization_never_creates_two_registries() {
    let p = Profile::new();
    let root = p.root.join("registry");
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let root = root.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                storage::open(&root, true).is_ok()
            })
        })
        .collect();
    let successes = handles
        .into_iter()
        .map(|h| usize::from(h.join().unwrap()))
        .sum::<usize>();
    assert!(successes >= 1);
    drop(storage::open(&root, false).unwrap());
}
#[test]
fn shared_rust_typescript_fixture_is_lossless() {
    let raw = include_str!("../../tests/contracts/registry.json").trim_start_matches('\u{feff}');
    let json: serde_json::Value = serde_json::from_str(raw).unwrap();
    let dto: CanonicalFileDto = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(serde_json::to_value(dto).unwrap(), json);
}
#[test]
fn duplicate_cancel_add_separately_and_changed_candidate_are_explicit() {
    let p = Profile::new();
    let path = p.root.join("original.txt");
    fs::write(&path, b"original").unwrap();
    let s = p.state();
    let e = epoch(&s);
    selected(&s, &e, path.clone());
    let token = |s: &state::DesktopState| match selection::selected(s, e.clone(), path.clone())
        .unwrap()
        .value
    {
        SelectionOutcomeDto::DuplicateDecision {
            selection_token, ..
        } => selection_token,
        _ => panic!(),
    };
    let request = |token, decision| RegisterSelectionRequestDto {
        storage_epoch: e.clone(),
        selection_token: token,
        decision,
        candidate_id: None,
    };
    assert!(matches!(
        selection::decide(&s, request(token(&s), Decision::Cancel))
            .unwrap()
            .value,
        DuplicateDecisionOutcomeDto::Cancelled
    ));
    assert!(matches!(
        selection::decide(&s, request(token(&s), Decision::AddSeparately))
            .unwrap()
            .value,
        DuplicateDecisionOutcomeDto::Added { .. }
    ));
    let t = token(&s);
    fs::write(&path, b"changed length").unwrap();
    assert_eq!(
        selection::decide(&s, request(t, Decision::AddSeparately))
            .unwrap_err()
            .code,
        Code::SelectionChanged
    );
    assert_eq!(
        registry::list(&s, SessionRequestDto { storage_epoch: e })
            .unwrap()
            .value
            .entries
            .len(),
        2
    );
}
#[test]
fn every_recorded_status_is_preserved_by_desktop_alias_operations() {
    use filenym_core::*;
    use filenym_ports::CanonicalFileRepository;
    let p = Profile::new();
    let s = p.state();
    let e = epoch(&s);
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
        let id = format!("seed-{index}");
        let base = CanonicalFile::new(
            CanonicalFileId::new(&id).unwrap(),
            KnownFileLocation::new(p.root.join("does-not-exist.txt")).unwrap(),
            Some(FileIdentity::new("fixture", vec![0, 255]).unwrap()),
            FileMetadata::new(u64::MAX, None),
            UNIX_EPOCH,
        );
        let file = CanonicalFile::restore(
            CanonicalFileSnapshot::new(base, status, UNIX_EPOCH - Duration::from_secs(1), vec![])
                .unwrap(),
        )
        .unwrap();
        s.with(|session| {
            session.repository.as_mut().unwrap().insert(&file)?;
            Ok(())
        })
        .unwrap();
        let original = registry::get(
            &s,
            GetFileRequestDto {
                storage_epoch: e.clone(),
                canonical_id: id.clone(),
            },
        )
        .unwrap()
        .value;
        let added = registry::add(
            &s,
            AddAliasRequestDto {
                storage_epoch: e.clone(),
                canonical_id: id.clone(),
                expected_revision: original.revision.clone(),
                name: "Logical.pdf".into(),
            },
        )
        .unwrap()
        .value;
        assert_eq!(added.status, original.status);
        assert_eq!(added.last_observed_at, original.last_observed_at);
        assert_eq!(added.size_bytes, "18446744073709551615");
        let deleted = registry::delete(
            &s,
            DeleteAliasRequestDto {
                storage_epoch: e.clone(),
                canonical_id: id,
                expected_revision: added.revision,
                alias_id: added.aliases[0].id.clone(),
            },
        )
        .unwrap()
        .value;
        assert_eq!(deleted.status, original.status);
        assert_eq!(deleted.saved_path, original.saved_path);
    }
}
#[test]
fn corrupt_database_and_reopen_of_never_initialized_profile_do_not_create_records() {
    let p = Profile::new();
    let s = p.state();
    assert_eq!(
        s.initialize(true).unwrap_err().code,
        Code::DatabaseUnavailable
    );
    assert!(!p.root.join("registry/filenym.sqlite3").exists());
    drop(s);
    let p = Profile::new();
    let root = p.root.join("registry");
    fs::create_dir(&root).unwrap();
    let db = root.join("filenym.sqlite3");
    fs::write(&db, b"not a registry").unwrap();
    assert!(storage::open(&root, true).is_err());
    assert_eq!(fs::read(&db).unwrap(), b"not a registry");
    assert!(!root.join("registry-initialization-v1").exists());
}
#[test]
fn generated_record_identifiers_are_native_random_128_bit_hex() {
    let mut set = std::collections::BTreeSet::new();
    for _ in 0..256 {
        let id = ids::new_id().unwrap();
        assert_eq!(id.len(), 32);
        assert!(id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
        assert!(set.insert(id));
    }
}
#[test]
fn shutdown_gate_prevents_new_operations_and_reopen() {
    let p = Profile::new();
    let s = p.state();
    let e = epoch(&s);
    s.closing.store(true, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(
        registry::list(&s, SessionRequestDto { storage_epoch: e })
            .unwrap_err()
            .code,
        Code::DatabaseUnavailable
    );
    assert_eq!(
        s.initialize(true).unwrap_err().code,
        Code::DatabaseUnavailable
    );
}
#[test]
fn transport_maps_all_storage_categories_without_diagnostics() {
    use filenym_ports::PersistenceError as P;
    for (error, code) in [
        (P::DuplicateCanonicalId, Code::DuplicateCanonicalId),
        (P::DuplicateAliasId, Code::DuplicateAliasId),
        (P::DuplicateAliasName, Code::DuplicateAliasName),
        (P::NotFound, Code::CanonicalNotFound),
        (P::Conflict, Code::Conflict),
        (P::InvalidAggregateChange, Code::InvalidAggregateChange),
        (P::InvalidStoredData, Code::InvalidStoredData),
        (P::CorruptDatabase, Code::CorruptDatabase),
        (P::IncompatibleDatabase, Code::IncompatibleDatabase),
        (P::DatabaseAlreadyExists, Code::DatabaseAlreadyExists),
        (P::UnsupportedPathEncoding, Code::UnsupportedPathEncoding),
        (P::UnrepresentableValue, Code::UnrepresentableValue),
        (P::Busy, Code::Busy),
        (P::PermissionDenied, Code::PermissionDenied),
        (P::Unavailable, Code::DatabaseUnavailable),
        (P::MigrationFailed, Code::MigrationFailed),
        (P::StorageFailure, Code::StorageFailure),
        (P::UncertainOutcome, Code::UncertainOutcome),
    ] {
        let dto = CommandErrorDto::from(error);
        assert_eq!(dto.code, code);
        assert!(dto.field.is_none());
        assert!(dto.attempted.is_none());
        let value = serde_json::to_value(dto).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 6);
    }
}
#[test]
fn every_request_rejects_transport_shape_changes() {
    assert!(serde_json::from_value::<SessionRequestDto>(
        serde_json::json!({"storageEpoch":"e","path":"C:\\arbitrary"})
    )
    .is_err());
    assert!(serde_json::from_value::<GetFileRequestDto>(
        serde_json::json!({"storageEpoch":"e","canonicalId":5})
    )
    .is_err());
    assert!(serde_json::from_value::<AddAliasRequestDto>(
        serde_json::json!({"storageEpoch":"e","canonicalId":"c","expectedRevision":1,"name":"a"})
    )
    .is_err());
    assert!(serde_json::from_value::<RenameAliasRequestDto>(serde_json::json!({"storageEpoch":"e","canonicalId":"c","expectedRevision":"1","name":"a","aliasId":"a","updatedAt":0})).is_err());
    assert!(serde_json::from_value::<DeleteAliasRequestDto>(serde_json::json!({"storageEpoch":"e","canonicalId":"c","expectedRevision":"1","aliasId":"a","deletePhysical":true})).is_err());
    assert!(serde_json::from_value::<RegisterSelectionRequestDto>(serde_json::json!({"storageEpoch":"e","selectionToken":"t","decision":"overwrite","candidateId":null})).is_err());
}
#[test]
fn desktop_validation_unicode_boundaries_and_future_clock_failures_are_atomic() {
    use filenym_core::*;
    use filenym_ports::CanonicalFileRepository;
    let p = Profile::new();
    let path = p.root.join("file.txt");
    fs::write(&path, b"unchanged").unwrap();
    let s = p.state();
    let e = epoch(&s);
    let mut file = selected(&s, &e, path.clone());
    for name in [
        "".to_owned(),
        " a".into(),
        "a/../b".into(),
        "CON.pdf".into(),
        "a?b".into(),
        "a.".into(),
        "a".repeat(256),
        "é".repeat(128),
    ] {
        assert_eq!(
            registry::add(
                &s,
                AddAliasRequestDto {
                    storage_epoch: e.clone(),
                    canonical_id: file.id.clone(),
                    expected_revision: file.revision.clone(),
                    name
                }
            )
            .unwrap_err()
            .code,
            Code::InvalidAliasName
        );
        assert_eq!(
            registry::get(
                &s,
                GetFileRequestDto {
                    storage_epoch: e.clone(),
                    canonical_id: file.id.clone()
                }
            )
            .unwrap()
            .value,
            file
        );
    }
    for name in [
        "a".repeat(255),
        "é".repeat(127) + "a",
        "É.pdf".into(),
        "é.pdf".into(),
        "e\u{301}.pdf".into(),
    ] {
        file = registry::add(
            &s,
            AddAliasRequestDto {
                storage_epoch: e.clone(),
                canonical_id: file.id.clone(),
                expected_revision: file.revision.clone(),
                name,
            },
        )
        .unwrap()
        .value;
    }
    assert_eq!(file.aliases.len(), 5);
    let future = CanonicalFile::new(
        CanonicalFileId::new("future").unwrap(),
        KnownFileLocation::new(path).unwrap(),
        None,
        FileMetadata::new(0, None),
        std::time::SystemTime::now() + Duration::from_secs(600),
    );
    s.with(|session| {
        session.repository.as_mut().unwrap().insert(&future)?;
        Ok(())
    })
    .unwrap();
    assert_eq!(
        registry::add(
            &s,
            AddAliasRequestDto {
                storage_epoch: e.clone(),
                canonical_id: "future".into(),
                expected_revision: "0".into(),
                name: "Future.pdf".into()
            }
        )
        .unwrap_err()
        .code,
        Code::InvalidTimestamp
    );
    let unchanged = registry::get(
        &s,
        GetFileRequestDto {
            storage_epoch: e,
            canonical_id: "future".into(),
        },
    )
    .unwrap()
    .value;
    assert!(unchanged.aliases.is_empty());
    assert_eq!(unchanged.revision, "0");
}
#[test]
fn reserved_duplicate_cannot_be_consumed_twice_and_old_epoch_precedes_inspection() {
    let p = Profile::new();
    let path = p.root.join("file.txt");
    fs::write(&path, b"safe").unwrap();
    let s = p.state();
    let e = epoch(&s);
    selected(&s, &e, path.clone());
    let token = match selection::selected(&s, e.clone(), path).unwrap().value {
        SelectionOutcomeDto::DuplicateDecision {
            selection_token, ..
        } => selection_token,
        _ => panic!(),
    };
    s.with(|session| {
        session.pending.as_mut().unwrap().reserved = true;
        Ok(())
    })
    .unwrap();
    assert_eq!(
        selection::decide(
            &s,
            RegisterSelectionRequestDto {
                storage_epoch: e,
                selection_token: token,
                decision: Decision::AddSeparately,
                candidate_id: None
            }
        )
        .unwrap_err()
        .code,
        Code::SelectionInvalid
    );
    assert_eq!(
        selection::selected(&s, "old".into(), p.root.join("does-not-exist"))
            .unwrap_err()
            .code,
        Code::StaleSession
    );
}
#[test]
fn unreadable_inventory_keeps_uncertain_session_gated_until_verified_reload() {
    let p = Profile::new();
    let path = p.root.join("file.txt");
    fs::write(&path, b"safe").unwrap();
    let s = p.state();
    let e = epoch(&s);
    let file = selected(&s, &e, path);
    registry::add(
        &s,
        AddAliasRequestDto {
            storage_epoch: e,
            canonical_id: file.id.clone(),
            expected_revision: file.revision,
            name: "Valid.pdf".into(),
        },
    )
    .unwrap();
    let raw = rusqlite::Connection::open(p.root.join("registry/filenym.sqlite3")).unwrap();
    raw.execute("UPDATE aliases SET name='bad?'", []).unwrap();
    s.with(|session| {
        session.inventory_review = true;
        session.repository = None;
        Ok(())
    })
    .unwrap();
    let reopened = s.initialize(true).unwrap();
    let epoch = reopened.storage_epoch.unwrap();
    let list = registry::list(
        &s,
        SessionRequestDto {
            storage_epoch: epoch.clone(),
        },
    )
    .unwrap();
    assert!(matches!(
        &list.value.entries[0],
        RegistryEntryDto::Unreadable {
            code: Code::InvalidStoredData,
            ..
        }
    ));
    s.with(|session| {
        assert_eq!(
            s.check(session, &epoch, true).unwrap_err().code,
            Code::UncertainOutcome
        );
        Ok(())
    })
    .unwrap();
    // Only the test restores deliberately corrupted fixture data; production has no repair endpoint.
    raw.execute("UPDATE aliases SET name='Valid.pdf'", [])
        .unwrap();
    registry::get(
        &s,
        GetFileRequestDto {
            storage_epoch: epoch.clone(),
            canonical_id: file.id,
        },
    )
    .unwrap();
    s.with(|session| s.check(session, &epoch, true)).unwrap();
}
#[test]
fn external_revision_change_and_sqlite_busy_are_typed_without_retry() {
    let p = Profile::new();
    let path = p.root.join("file.txt");
    fs::write(&path, b"safe").unwrap();
    let s = p.state();
    let e = epoch(&s);
    let file = selected(&s, &e, path);
    let raw = rusqlite::Connection::open(p.root.join("registry/filenym.sqlite3")).unwrap();
    raw.execute("UPDATE canonical_files SET revision=revision+1", [])
        .unwrap();
    let request = || AddAliasRequestDto {
        storage_epoch: e.clone(),
        canonical_id: file.id.clone(),
        expected_revision: "0".into(),
        name: "New.pdf".into(),
    };
    assert_eq!(
        registry::add(&s, request()).unwrap_err().code,
        Code::Conflict
    );
    let fresh = registry::get(
        &s,
        GetFileRequestDto {
            storage_epoch: e.clone(),
            canonical_id: file.id.clone(),
        },
    )
    .unwrap()
    .value;
    raw.execute_batch("BEGIN IMMEDIATE").unwrap();
    let error = registry::add(
        &s,
        AddAliasRequestDto {
            expected_revision: fresh.revision.clone(),
            ..request()
        },
    )
    .unwrap_err();
    assert_eq!(error.code, Code::Busy);
    raw.execute_batch("ROLLBACK").unwrap();
    assert_eq!(
        registry::get(
            &s,
            GetFileRequestDto {
                storage_epoch: e.clone(),
                canonical_id: file.id.clone()
            }
        )
        .unwrap()
        .value,
        fresh
    );
    let added = registry::add(
        &s,
        AddAliasRequestDto {
            expected_revision: fresh.revision,
            ..request()
        },
    )
    .unwrap()
    .value;
    assert_eq!(added.aliases.len(), 1);
}
#[test]
fn desktop_security_configuration_has_narrow_local_capability_and_production_csp() {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let app = &config["app"];
    assert_eq!(app["windows"][0]["label"], "main");
    assert_eq!(app["windows"][0]["dragDropEnabled"], false);
    assert_eq!(app["windows"][0]["minWidth"], 640);
    assert_eq!(app["windows"][0]["minHeight"], 440);
    let csp = app["security"]["csp"].as_str().unwrap();
    assert!(!csp.contains("unsafe-inline"));
    assert!(!csp.contains("https:"));
    assert!(csp.contains("connect-src ipc: http://ipc.localhost"));
    assert!(csp.contains("object-src 'none'"));
    let capability: serde_json::Value =
        serde_json::from_str(include_str!("../capabilities/default.json")).unwrap();
    assert_eq!(capability["windows"], serde_json::json!(["main"]));
    assert!(capability.get("remote").is_none());
    let permissions = capability["permissions"].as_array().unwrap();
    assert_eq!(permissions.len(), 11);
    for value in permissions {
        let permission = value.as_str().unwrap();
        assert!(!permission.starts_with("fs:"));
        assert!(!permission.starts_with("dialog:"));
        assert!(!permission.contains("shell"));
        assert!(!permission.contains("sql"));
    }
}
#[test]
fn readonly_and_newer_database_are_blocked_without_repair_or_marker_changes() {
    let p = Profile::new();
    let root = p.root.join("registry");
    drop(storage::open(&root, true).unwrap());
    let db = root.join("filenym.sqlite3");
    let marker = fs::read(root.join("registry-initialization-v1")).unwrap();
    let metadata = fs::metadata(&db).unwrap();
    let mut permissions = metadata.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&db, permissions).unwrap();
    let result = storage::open(&root, true);
    fs::set_permissions(&db, metadata.permissions()).unwrap();
    assert_eq!(result.err().unwrap().code, Code::PermissionDenied);
    let raw = rusqlite::Connection::open(&db).unwrap();
    raw.pragma_update(None, "user_version", 2).unwrap();
    drop(raw);
    assert_eq!(
        storage::open(&root, true).err().unwrap().code,
        Code::IncompatibleDatabase
    );
    assert_eq!(
        fs::read(root.join("registry-initialization-v1")).unwrap(),
        marker
    );
}
#[test]
fn desktop_composition_survives_fresh_writer_reader_and_next_writer_processes() {
    use std::{
        process::{Command, Stdio},
        time::Instant,
    };
    let p = Profile::new();
    let path = p.root.join("process-original.txt");
    fs::write(&path, b"desktop process sentinel").unwrap();
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    for mode in ["write", "read-and-rename", "read"] {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "tests::desktop_restart_worker"])
            .env("FILENYM_DESKTOP_TEST_ROOT", &p.root)
            .env("FILENYM_DESKTOP_TEST_MODE", mode)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "desktop child mode {mode} failed");
                break;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("desktop child exceeded bounded wait");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    let receipt: CanonicalFileDto =
        serde_json::from_slice(&fs::read(p.root.join("receipt.json")).unwrap()).unwrap();
    assert_eq!(receipt.revision, "2");
    assert_eq!(receipt.aliases[0].name, "FreshProcess.PDF");
    assert_eq!(fs::read(&path).unwrap(), b"desktop process sentinel");
    assert_eq!(fs::metadata(path).unwrap().modified().unwrap(), modified);
    assert!(!p.root.join("FreshProcess.PDF").exists());
}
#[test]
fn desktop_restart_worker() {
    let Some(root) = std::env::var_os("FILENYM_DESKTOP_TEST_ROOT") else {
        return;
    };
    let root = PathBuf::from(root).canonicalize().unwrap();
    assert_eq!(
        root.parent(),
        Some(std::env::temp_dir().canonicalize().unwrap().as_path())
    );
    assert!(root
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("filenym-desktop-test-"));
    assert_eq!(
        fs::read(root.join("process-original.txt")).unwrap(),
        b"desktop process sentinel"
    );
    let mode = std::env::var("FILENYM_DESKTOP_TEST_MODE").unwrap();
    let s = state::DesktopState::new(Some(root.join("registry")));
    let e = epoch(&s);
    let receipt = root.join("receipt.json");
    if mode == "write" {
        let file = selected(&s, &e, root.join("process-original.txt"));
        let file = registry::add(
            &s,
            AddAliasRequestDto {
                storage_epoch: e,
                canonical_id: file.id,
                expected_revision: file.revision,
                name: "FreshProcess.PDF".into(),
            },
        )
        .unwrap()
        .value;
        fs::write(receipt, serde_json::to_vec(&file).unwrap()).unwrap();
    } else {
        let expected: CanonicalFileDto =
            serde_json::from_slice(&fs::read(&receipt).unwrap()).unwrap();
        let actual = registry::get(
            &s,
            GetFileRequestDto {
                storage_epoch: e.clone(),
                canonical_id: expected.id.clone(),
            },
        )
        .unwrap()
        .value;
        assert_eq!(actual, expected);
        if mode == "read-and-rename" {
            let renamed = registry::rename(
                &s,
                RenameAliasRequestDto {
                    storage_epoch: e,
                    canonical_id: actual.id,
                    expected_revision: actual.revision,
                    alias_id: actual.aliases[0].id.clone(),
                    name: actual.aliases[0].name.clone(),
                },
            )
            .unwrap()
            .value;
            fs::write(receipt, serde_json::to_vec(&renamed).unwrap()).unwrap();
        } else {
            assert_eq!(mode, "read");
        }
    }
}
#[test]
fn background_worker_panic_without_mutex_poison_still_retires_session() {
    let p = Profile::new();
    let s = p.state();
    epoch(&s);
    let result = tauri::async_runtime::block_on(state::blocking::<()>(s.clone(), |_| {
        panic!("simulated worker failure")
    }));
    assert_eq!(result.unwrap_err().code, Code::UncertainOutcome);
    assert_eq!(s.status.lock().unwrap().value.phase, Phase::NeedsReopen);
    assert_eq!(s.initialize(true).unwrap().value.phase, Phase::NeedsReload);
}
#[test]
fn generated_runtime_acl_denies_other_windows_remote_origins_and_ungranted_commands() {
    use tauri::ipc::Origin;
    let mut context: tauri::Context<tauri::Wry> = tauri::generate_context!();
    let authority = context.runtime_authority_mut();
    let remote = Origin::Remote {
        url: "https://untrusted.invalid".parse().unwrap(),
    };
    for command in [
        "version",
        "registry_status",
        "reopen_registry",
        "get_canonical_file",
        "list_registered_files",
        "add_alias",
        "rename_alias",
        "delete_alias",
        "select_existing_file",
        "register_selected_file",
    ] {
        assert!(
            authority
                .resolve_access(command, "main", "main", &Origin::Local)
                .is_some(),
            "main: {command}"
        );
        assert!(
            authority
                .resolve_access(command, "other", "other", &Origin::Local)
                .is_none(),
            "other: {command}"
        );
        assert!(
            authority
                .resolve_access(command, "main", "main", &remote)
                .is_none(),
            "remote: {command}"
        );
    }
    for command in [
        "plugin:dialog|open",
        "plugin:fs|read_file",
        "plugin:sql|execute",
        "plugin:webview|create_webview_window",
    ] {
        assert!(
            authority
                .resolve_access(command, "main", "main", &Origin::Local)
                .is_none(),
            "ungranted: {command}"
        );
    }
}
