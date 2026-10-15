mod support;
use filenym_app::{ApplicationError as E, FileRegistryService};
use filenym_core::*;
use filenym_ports::*;
use std::error::Error;
use support::*;

#[test]
fn registration_and_reads_use_only_the_required_calls_and_return_repository_revisions() {
    let file = file();
    let stored = StoredCanonicalFile::new(file.clone(), rev(19));
    let mut spy = Spy::new([
        Step::Insert(file, Ok(rev(19))),
        Step::Load("canonical".into(), Ok(Some(stored.clone()))),
        Step::List(Ok(vec![CanonicalFileId::new("canonical").unwrap()])),
        Step::List(Ok(vec![])),
        Step::Load("missing".into(), Ok(None)),
    ]);
    let mut service = FileRegistryService::new(&mut spy as &mut dyn CanonicalFileRepository);
    assert_eq!(
        service.register_canonical_file(registration()).unwrap(),
        stored
    );
    assert_eq!(service.get_canonical_file("canonical").unwrap(), stored);
    assert_eq!(service.list_registered_file_ids().unwrap().len(), 1);
    assert!(service.list_registered_file_ids().unwrap().is_empty());
    assert_eq!(
        service.get_canonical_file("missing"),
        Err(E::CanonicalNotFound)
    );
    spy.done();
}

#[test]
fn mutations_load_apply_and_write_once_without_post_commit_reads() {
    for kind in 0..3 {
        let old = populated();
        let mut expected = old.clone();
        match kind {
            0 => expected
                .create_alias(
                    AliasId::new("other").unwrap(),
                    AliasName::new("Other.txt").unwrap(),
                    at(12),
                )
                .unwrap(),
            1 => expected
                .rename_alias(
                    &AliasId::new("alias").unwrap(),
                    AliasName::new("Changed.exe").unwrap(),
                    at(13),
                )
                .unwrap(),
            _ => expected
                .delete_alias(&AliasId::new("alias").unwrap())
                .unwrap(),
        }
        let mut spy = Spy::new([
            loaded(old),
            Step::Update(expected.clone(), rev(3), Ok(rev(27))),
        ]);
        let mut service = FileRegistryService::new(&mut spy);
        let actual = match kind {
            0 => service.add_alias(add()),
            1 => service.rename_alias(rename()),
            _ => service.delete_alias(delete()),
        }
        .unwrap();
        assert_eq!(actual, StoredCanonicalFile::new(expected, rev(27)));
        spy.done();
    }
}

#[test]
fn identical_rename_still_updates_and_invalid_time_prevents_write() {
    let old = populated();
    let mut request = rename();
    request.name = "Report.PDF".into();
    let mut spy = Spy::new([
        loaded(old.clone()),
        Step::Update(old.clone(), rev(3), Ok(rev(4))),
    ]);
    assert_eq!(
        FileRegistryService::new(&mut spy)
            .rename_alias(request.clone())
            .unwrap(),
        StoredCanonicalFile::new(old.clone(), rev(4))
    );
    spy.done();
    request.updated_at = at(10);
    let mut spy = Spy::new([loaded(old)]);
    assert_eq!(
        FileRegistryService::new(&mut spy).rename_alias(request),
        Err(E::Domain(DomainError::InvalidTimestamp))
    );
    spy.done();
}

#[test]
fn primitive_validation_precedes_repository_access_in_field_order() {
    for bad in ["", " ", " id", "id ", "x\0y", "x\ny", "\u{2003}"] {
        let mut spy = Spy::new([]);
        let mut service = FileRegistryService::new(&mut spy);
        let mut r = registration();
        r.canonical_id = bad.into();
        assert_eq!(
            service.register_canonical_file(r),
            Err(E::InvalidCanonicalId)
        );
        assert_eq!(service.get_canonical_file(bad), Err(E::InvalidCanonicalId));
        let mut a = add();
        a.canonical_id = bad.into();
        a.alias_id = "".into();
        a.name = "".into();
        assert_eq!(service.add_alias(a), Err(E::InvalidCanonicalId));
        let mut r = rename();
        r.canonical_id = bad.into();
        assert_eq!(service.rename_alias(r), Err(E::InvalidCanonicalId));
        let mut d = delete();
        d.canonical_id = bad.into();
        assert_eq!(service.delete_alias(d), Err(E::InvalidCanonicalId));
        let mut a = add();
        a.alias_id = bad.into();
        a.name = "".into();
        assert_eq!(service.add_alias(a), Err(E::InvalidAliasId));
        let mut r = rename();
        r.alias_id = bad.into();
        r.name = "".into();
        assert_eq!(service.rename_alias(r), Err(E::InvalidAliasId));
        let mut d = delete();
        d.alias_id = bad.into();
        assert_eq!(service.delete_alias(d), Err(E::InvalidAliasId));
        spy.done();
    }
}

#[test]
fn every_alias_validation_category_is_preserved_without_repository_access() {
    let cases = [
        ("", AliasNameError::Empty),
        (" ", AliasNameError::Whitespace),
        ("\u{2003}a", AliasNameError::Whitespace),
        ("a\u{2003}", AliasNameError::Whitespace),
        (".", AliasNameError::TraversalOrSeparator),
        ("..", AliasNameError::TraversalOrSeparator),
        ("../x", AliasNameError::TraversalOrSeparator),
        ("x/y", AliasNameError::TraversalOrSeparator),
        ("x\\y", AliasNameError::TraversalOrSeparator),
        ("a.", AliasNameError::TrailingPeriod),
        ("a\0", AliasNameError::ForbiddenCharacter),
        ("a\n", AliasNameError::ForbiddenCharacter),
        ("a\u{0085}", AliasNameError::ForbiddenCharacter),
    ];
    let mut cases: Vec<(String, AliasNameError)> =
        cases.into_iter().map(|(s, e)| (s.into(), e)).collect();
    for ch in "<>:\"|?*".chars() {
        cases.push((format!("a{ch}"), AliasNameError::ForbiddenCharacter));
    }
    for stem in ["CON", "PRN", "AUX", "NUL"] {
        for ext in ["", ".pdf"] {
            cases.push((format!("{stem}{ext}"), AliasNameError::DeviceName));
        }
    }
    for prefix in ["COM", "LPT"] {
        for digit in ["1", "2", "3", "4", "5", "6", "7", "8", "9", "¹", "²", "³"] {
            for ext in ["", ".pdf"] {
                cases.push((format!("{prefix}{digit}{ext}"), AliasNameError::DeviceName));
            }
        }
    }
    cases.extend([
        ("a".repeat(256), AliasNameError::TooLong),
        ("é".repeat(128), AliasNameError::TooLong),
    ]);
    for (name, reason) in cases {
        let mut spy = Spy::new([]);
        let mut service = FileRegistryService::new(&mut spy);
        let mut a = add();
        a.name = name.clone();
        assert_eq!(
            service.add_alias(a),
            Err(E::Domain(DomainError::InvalidAliasName(reason))),
            "{name:?}"
        );
        let mut r = rename();
        r.name = name;
        assert_eq!(
            service.rename_alias(r),
            Err(E::Domain(DomainError::InvalidAliasName(reason)))
        );
        spy.done();
    }
}

#[test]
fn contextual_domain_failures_never_write() {
    for kind in 0..7 {
        let old = populated();
        let mut spy = Spy::new([loaded(old)]);
        let mut service = FileRegistryService::new(&mut spy);
        let (actual, expected) = match kind {
            0 => {
                let mut a = add();
                a.alias_id = "alias".into();
                (service.add_alias(a), E::DuplicateAliasId)
            }
            1 => {
                let mut a = add();
                a.name = "REPORT.pdf".into();
                (service.add_alias(a), E::DuplicateAliasName)
            }
            2 => {
                let mut a = add();
                a.created_at = at(9);
                (
                    service.add_alias(a),
                    E::Domain(DomainError::InvalidTimestamp),
                )
            }
            3 => {
                let mut r = rename();
                r.alias_id = "absent".into();
                (service.rename_alias(r), E::AliasNotFound)
            }
            4 => {
                let mut r = rename();
                r.updated_at = at(10);
                (
                    service.rename_alias(r),
                    E::Domain(DomainError::InvalidTimestamp),
                )
            }
            5 => {
                let mut d = delete();
                d.alias_id = "absent".into();
                (service.delete_alias(d), E::AliasNotFound)
            }
            _ => {
                let mut old = populated();
                old.create_alias(
                    AliasId::new("other").unwrap(),
                    AliasName::new("Changed.exe").unwrap(),
                    at(12),
                )
                .unwrap();
                spy = Spy::new([loaded(old)]);
                (
                    FileRegistryService::new(&mut spy).rename_alias(rename()),
                    E::DuplicateAliasName,
                )
            }
        };
        assert_eq!(actual, Err(expected));
        spy.done();
    }
}

#[test]
fn missing_canonical_and_stale_or_future_revisions_precede_contextual_errors() {
    for kind in 0..3 {
        for revision in [None, Some(2), Some(4)] {
            let mut spy = Spy::new([match revision {
                None => Step::Load("canonical".into(), Ok(None)),
                Some(_) => loaded(populated()),
            }]);
            let mut service = FileRegistryService::new(&mut spy);
            let expected = if revision.is_none() {
                E::CanonicalNotFound
            } else {
                E::Conflict
            };
            let result = match kind {
                0 => {
                    let mut a = add();
                    a.alias_id = "alias".into();
                    a.expected_revision = rev(revision.unwrap_or(3));
                    service.add_alias(a)
                }
                1 => {
                    let mut r = rename();
                    r.alias_id = "missing".into();
                    r.expected_revision = rev(revision.unwrap_or(3));
                    service.rename_alias(r)
                }
                _ => {
                    let mut d = delete();
                    d.alias_id = "missing".into();
                    d.expected_revision = rev(revision.unwrap_or(3));
                    service.delete_alias(d)
                }
            };
            assert_eq!(result, Err(expected));
            spy.done();
        }
    }
}

#[test]
fn every_persistence_error_propagates_without_retry_or_false_success() {
    use PersistenceError as P;
    let cases = [
        (P::DuplicateCanonicalId, E::DuplicateCanonicalId),
        (P::DuplicateAliasId, E::DuplicateAliasId),
        (P::DuplicateAliasName, E::DuplicateAliasName),
        (P::NotFound, E::CanonicalNotFound),
        (P::Conflict, E::Conflict),
        (
            P::InvalidAggregateChange,
            E::Storage(P::InvalidAggregateChange),
        ),
        (P::InvalidStoredData, E::Storage(P::InvalidStoredData)),
        (P::CorruptDatabase, E::Storage(P::CorruptDatabase)),
        (P::IncompatibleDatabase, E::Storage(P::IncompatibleDatabase)),
        (
            P::DatabaseAlreadyExists,
            E::Storage(P::DatabaseAlreadyExists),
        ),
        (
            P::UnsupportedPathEncoding,
            E::Storage(P::UnsupportedPathEncoding),
        ),
        (P::UnrepresentableValue, E::Storage(P::UnrepresentableValue)),
        (P::Busy, E::Storage(P::Busy)),
        (P::PermissionDenied, E::Storage(P::PermissionDenied)),
        (P::Unavailable, E::Storage(P::Unavailable)),
        (P::MigrationFailed, E::Storage(P::MigrationFailed)),
        (P::StorageFailure, E::Storage(P::StorageFailure)),
        (P::UncertainOutcome, E::Storage(P::UncertainOutcome)),
    ];
    for (error, expected) in cases {
        let mut spy = Spy::new([Step::Insert(file(), Err(error))]);
        assert_eq!(
            FileRegistryService::new(&mut spy).register_canonical_file(registration()),
            Err(expected)
        );
        spy.done();
        let mut spy = Spy::new([Step::List(Err(error))]);
        assert_eq!(
            FileRegistryService::new(&mut spy).list_registered_file_ids(),
            Err(expected)
        );
        spy.done();
        for kind in 0..4 {
            let mut spy = Spy::new([Step::Load("canonical".into(), Err(error))]);
            let mut service = FileRegistryService::new(&mut spy);
            let result = match kind {
                0 => service.get_canonical_file("canonical"),
                1 => service.add_alias(add()),
                2 => service.rename_alias(rename()),
                _ => service.delete_alias(delete()),
            };
            assert_eq!(result, Err(expected));
            spy.done();
        }
        for kind in 0..3 {
            let old = populated();
            let mut changed = old.clone();
            match kind {
                0 => changed
                    .create_alias(
                        AliasId::new("other").unwrap(),
                        AliasName::new("Other.txt").unwrap(),
                        at(12),
                    )
                    .unwrap(),
                1 => changed
                    .rename_alias(
                        &AliasId::new("alias").unwrap(),
                        AliasName::new("Changed.exe").unwrap(),
                        at(13),
                    )
                    .unwrap(),
                _ => changed
                    .delete_alias(&AliasId::new("alias").unwrap())
                    .unwrap(),
            };
            let mut spy = Spy::new([loaded(old), Step::Update(changed, rev(3), Err(error))]);
            let mut service = FileRegistryService::new(&mut spy);
            let result = match kind {
                0 => service.add_alias(add()),
                1 => service.rename_alias(rename()),
                _ => service.delete_alias(delete()),
            };
            assert_eq!(result, Err(expected));
            spy.done();
        }
    }
}

#[test]
fn domain_mapping_and_error_sources_are_safe_and_complete() {
    let cases = [
        DomainError::InvalidRecordId,
        DomainError::InvalidLocation,
        DomainError::InvalidIdentityEvidence,
        DomainError::InvalidTimestamp,
        DomainError::InvalidAliasName(AliasNameError::Empty),
        DomainError::AliasOwnershipMismatch,
        DomainError::IdentityMismatch,
        DomainError::UnresolvedReplacement,
    ];
    for error in cases {
        let actual = E::from(error);
        assert_eq!(actual, E::Domain(error));
        assert_eq!(actual.source().unwrap().to_string(), error.to_string());
    }
    for (error, expected) in [
        (DomainError::AliasNotFound, E::AliasNotFound),
        (DomainError::DuplicateAliasId, E::DuplicateAliasId),
        (DomainError::DuplicateAliasName, E::DuplicateAliasName),
    ] {
        assert_eq!(E::from(error), expected);
        assert!(expected.source().is_none());
    }
    for error in [
        E::InvalidCanonicalId,
        E::InvalidAliasId,
        E::CanonicalNotFound,
        E::AliasNotFound,
        E::DuplicateCanonicalId,
        E::DuplicateAliasId,
        E::DuplicateAliasName,
        E::Conflict,
        E::Domain(DomainError::InvalidTimestamp),
        E::Storage(PersistenceError::UncertainOutcome),
    ] {
        let mut messages = error.to_string();
        if let Some(source) = error.source() {
            messages.push_str(&source.to_string());
        }
        for secret in ["private.pdf", "Report.PDF", "SELECT secret", "opaque-token"] {
            assert!(!messages.contains(secret));
        }
    }
    assert!(E::Storage(PersistenceError::UncertainOutcome)
        .to_string()
        .contains("reopen and reload"));
}
