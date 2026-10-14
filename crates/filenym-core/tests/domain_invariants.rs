use filenym_core::*;
use std::{
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}
fn location(parent: &str, name: &str) -> KnownFileLocation {
    let root = if cfg!(windows) {
        PathBuf::from("C:\\synthetic")
    } else {
        PathBuf::from("/synthetic")
    };
    KnownFileLocation::new(root.join(parent).join(name)).unwrap()
}
fn evidence(value: u8) -> FileIdentity {
    FileIdentity::new("synthetic", vec![value]).unwrap()
}
fn alias_id(value: &str) -> AliasId {
    AliasId::new(value).unwrap()
}
fn name(value: &str) -> AliasName {
    AliasName::new(value).unwrap()
}
fn file() -> CanonicalFile {
    CanonicalFile::new(
        CanonicalFileId::new("canonical").unwrap(),
        location("old", "original.pdf"),
        Some(evidence(1)),
        FileMetadata::new(0, None),
        at(10),
    )
}
fn same(
    parent: &str,
    filename: &str,
    size: u64,
    identity: Option<FileIdentity>,
    time: u64,
) -> SameFileObservation {
    SameFileObservation::new(
        location(parent, filename),
        FileMetadata::new(size, None),
        identity,
        at(time),
    )
}
fn populated() -> CanonicalFile {
    let mut file = file();
    file.create_alias(alias_id("a"), name("Report.PDF"), at(11))
        .unwrap();
    file.create_alias(alias_id("b"), name("Résumé.txt"), at(12))
        .unwrap();
    file
}
fn assert_canonical_unchanged(actual: &CanonicalFile, original: &CanonicalFile) {
    assert_eq!(actual.id(), original.id());
    assert_eq!(actual.location(), original.location());
    assert_eq!(actual.identity(), original.identity());
    assert_eq!(actual.metadata(), original.metadata());
    assert_eq!(actual.registered_at(), original.registered_at());
    assert_eq!(actual.last_observed_at(), original.last_observed_at());
    assert_eq!(actual.status(), original.status());
}

#[test]
fn aliases_are_owned_metadata_and_operations_do_not_change_canonical_facts() {
    let original = file();
    let mut file = original.clone();
    assert!(file.aliases().is_empty());
    file.create_alias(alias_id("a"), name("context.exe"), at(11))
        .unwrap();
    assert_eq!(file.aliases()[0].canonical_file_id(), file.id());
    assert_eq!(file.aliases()[0].created_at(), at(11));
    file.rename_alias(&alias_id("a"), name("context.txt"), at(12))
        .unwrap();
    assert_eq!(file.aliases()[0].name().as_str(), "context.txt");
    assert_eq!(file.aliases()[0].created_at(), at(11));
    assert_eq!(file.aliases()[0].updated_at(), at(12));
    assert_canonical_unchanged(&file, &original);
    file.delete_alias(&alias_id("a")).unwrap();
    assert!(file.aliases().is_empty());
    assert_eq!(file, original);
}

#[test]
fn rejected_alias_operations_are_atomic() {
    let original = populated();
    let mut file = original.clone();
    for (id, value, time, error) in [
        ("new", "report.pdf", 13, DomainError::DuplicateAliasName),
        ("a", "unique", 13, DomainError::DuplicateAliasId),
        ("new", "unique", 9, DomainError::InvalidTimestamp),
    ] {
        assert_eq!(
            file.create_alias(alias_id(id), name(value), at(time)),
            Err(error)
        );
        assert_eq!(file, original);
    }
    for (id, value, time, error) in [
        ("a", "résumé.txt", 13, DomainError::DuplicateAliasName),
        ("absent", "unique", 13, DomainError::AliasNotFound),
        ("a", "unique", 10, DomainError::InvalidTimestamp),
    ] {
        // ASCII case folds; non-ASCII characters here remain exactly the same.
        assert_eq!(
            file.rename_alias(&alias_id(id), name(value), at(time)),
            Err(error)
        );
        assert_eq!(file, original);
    }
    assert_eq!(
        file.delete_alias(&alias_id("absent")),
        Err(DomainError::AliasNotFound)
    );
    assert_eq!(file, original);
}

#[test]
fn identical_rename_is_noop_case_only_rename_updates_and_deletion_is_targeted() {
    let mut file = populated();
    let original = file.clone();
    file.rename_alias(&alias_id("a"), name("Report.PDF"), at(20))
        .unwrap();
    assert_eq!(file, original);
    file.rename_alias(&alias_id("a"), name("report.pdf"), at(20))
        .unwrap();
    assert_eq!(file.aliases()[0].updated_at(), at(20));
    assert_eq!(file.aliases()[0].name().as_str(), "report.pdf");
    let retained = file.aliases()[1].clone();
    file.delete_alias(&alias_id("a")).unwrap();
    assert_eq!(file.aliases(), &[retained]);
    assert_canonical_unchanged(&file, &original);
}

#[test]
fn uniqueness_is_per_canonical_and_ascii_only() {
    let mut first = file();
    let mut second = CanonicalFile::new(
        CanonicalFileId::new("other").unwrap(),
        first.location().clone(),
        first.identity().cloned(),
        first.metadata().clone(),
        at(10),
    );
    for file in [&mut first, &mut second] {
        for (index, value) in [
            "Report.pdf",
            "É.pdf",
            "é.pdf",
            "e\u{301}.pdf",
            "original.pdf",
        ]
        .iter()
        .enumerate()
        {
            file.create_alias(alias_id(&index.to_string()), name(value), at(11))
                .unwrap();
        }
        let snapshot = file.clone();
        assert_eq!(
            file.create_alias(alias_id("new"), name("REPORT.PDF"), at(11)),
            Err(DomainError::DuplicateAliasName)
        );
        assert_eq!(*file, snapshot);
    }
    assert_ne!(first.id(), second.id());
    assert_eq!(first.location(), second.location());
}

#[test]
fn lifecycle_change_results_cover_location_and_content_combinations() {
    for (parent, filename, location_change) in [
        ("old", "original.pdf", LocationChange::Unchanged),
        ("old", "renamed.pdf", LocationChange::Renamed),
        ("new", "original.pdf", LocationChange::Moved),
        ("new", "renamed.pdf", LocationChange::RenamedAndMoved),
    ] {
        for changed in [false, true] {
            let mut file = populated();
            let original = file.clone();
            let expected = match location_change {
                LocationChange::Unchanged if changed => FileChange::Modified,
                LocationChange::Unchanged => FileChange::Unchanged,
                LocationChange::Renamed => FileChange::Renamed {
                    metadata_changed: changed,
                },
                LocationChange::Moved => FileChange::Moved {
                    metadata_changed: changed,
                },
                LocationChange::RenamedAndMoved => FileChange::RenamedAndMoved {
                    metadata_changed: changed,
                },
            };
            assert_eq!(
                file.observe(FileObservation::ConfirmedSameFile(same(
                    parent,
                    filename,
                    u64::from(changed),
                    Some(evidence(1)),
                    20
                )))
                .unwrap(),
                expected
            );
            assert_eq!(
                file.status(),
                if changed {
                    FileStatus::Modified
                } else {
                    FileStatus::Available
                }
            );
            assert_eq!(file.id(), original.id());
            assert_eq!(file.aliases(), original.aliases());
            assert_eq!(file.location(), &location(parent, filename));
            assert_eq!(file.metadata().size(), u64::from(changed));
            assert_eq!(file.last_observed_at(), at(20));
            // A later unchanged observation clears transient Modified, even
            // with a clock-adjusted earlier observation time.
            assert_eq!(
                file.observe(FileObservation::ConfirmedSameFile(same(
                    parent,
                    filename,
                    u64::from(changed),
                    None,
                    5
                )))
                .unwrap(),
                FileChange::Unchanged
            );
            assert_eq!(file.status(), FileStatus::Available);
            assert_eq!(file.last_observed_at(), at(5));
        }
    }
}

#[test]
fn status_cross_product_preserves_aliases_and_guards_replacement() {
    for initial in [
        FileStatus::Available,
        FileStatus::Modified,
        FileStatus::Unavailable(UnavailableReason::NotFound),
        FileStatus::PossibleReplacement,
    ] {
        let mut seed = populated();
        match initial {
            FileStatus::Available => {}
            FileStatus::Modified => {
                seed.observe(FileObservation::ConfirmedSameFile(same(
                    "old",
                    "original.pdf",
                    1,
                    None,
                    13,
                )))
                .unwrap();
            }
            FileStatus::Unavailable(reason) => {
                seed.observe(FileObservation::Unavailable {
                    reason,
                    observed_at: at(13),
                })
                .unwrap();
            }
            FileStatus::PossibleReplacement => {
                seed.observe(FileObservation::PossibleReplacement {
                    observed_at: at(13),
                })
                .unwrap();
            }
        }
        assert_eq!(seed.status(), initial);
        for reason in [
            UnavailableReason::NotFound,
            UnavailableReason::PermissionDenied,
            UnavailableReason::Unknown,
        ] {
            let mut file = seed.clone();
            for time in [14, 15] {
                assert_eq!(
                    file.observe(FileObservation::Unavailable {
                        reason,
                        observed_at: at(time)
                    })
                    .unwrap(),
                    FileChange::Unavailable(reason)
                );
                assert_eq!(file.status(), FileStatus::Unavailable(reason));
                assert_eq!(file.location(), seed.location());
                assert_eq!(file.metadata(), seed.metadata());
                assert_eq!(file.identity(), seed.identity());
                assert_eq!(file.aliases(), seed.aliases());
                assert_eq!(file.last_observed_at(), at(time));
            }
        }
        let mut file = seed.clone();
        for time in [14, 15] {
            assert_eq!(
                file.observe(FileObservation::PossibleReplacement {
                    observed_at: at(time)
                })
                .unwrap(),
                FileChange::PossibleReplacement
            );
            assert_eq!(file.location(), seed.location());
            assert_eq!(file.metadata(), seed.metadata());
            assert_eq!(file.identity(), seed.identity());
            assert_eq!(file.aliases(), seed.aliases());
        }
        let mut file = seed.clone();
        let result = file.observe(FileObservation::ConfirmedSameFile(same(
            "old",
            "original.pdf",
            seed.metadata().size(),
            Some(evidence(1)),
            20,
        )));
        match initial {
            FileStatus::PossibleReplacement => {
                assert_eq!(result, Err(DomainError::UnresolvedReplacement));
                assert_eq!(file, seed);
            }
            FileStatus::Unavailable(_) => assert_eq!(
                result.unwrap(),
                FileChange::Recovered {
                    location_change: LocationChange::Unchanged,
                    metadata_changed: false
                }
            ),
            _ => assert_eq!(result.unwrap(), FileChange::Unchanged),
        }
        let mut file = seed.clone();
        let result = file
            .recover_verified_same_file(same("new", "renamed.pdf", 2, Some(evidence(1)), 21))
            .unwrap();
        if matches!(
            initial,
            FileStatus::Unavailable(_) | FileStatus::PossibleReplacement
        ) {
            assert_eq!(
                result,
                FileChange::Recovered {
                    location_change: LocationChange::RenamedAndMoved,
                    metadata_changed: true
                }
            );
        } else {
            assert_eq!(
                result,
                FileChange::RenamedAndMoved {
                    metadata_changed: true
                }
            );
        }
        assert_eq!(file.id(), seed.id());
        assert_eq!(file.aliases(), seed.aliases());
        assert_eq!(file.status(), FileStatus::Modified);
    }
}

#[test]
fn identity_conflicts_are_atomic_and_absence_never_erases_evidence() {
    for initial in [
        FileStatus::Available,
        FileStatus::Modified,
        FileStatus::Unavailable(UnavailableReason::Unknown),
        FileStatus::PossibleReplacement,
    ] {
        let mut file = populated();
        match initial {
            FileStatus::Available => {}
            FileStatus::Modified => {
                file.observe(FileObservation::ConfirmedSameFile(same(
                    "old",
                    "original.pdf",
                    1,
                    None,
                    15,
                )))
                .unwrap();
            }
            FileStatus::Unavailable(reason) => {
                file.observe(FileObservation::Unavailable {
                    reason,
                    observed_at: at(15),
                })
                .unwrap();
            }
            FileStatus::PossibleReplacement => {
                file.observe(FileObservation::PossibleReplacement {
                    observed_at: at(15),
                })
                .unwrap();
            }
        }
        let snapshot = file.clone();
        for conflicting in [
            evidence(2),
            FileIdentity::new("other-scheme", vec![1]).unwrap(),
        ] {
            let observation = same("new", "unrelated.exe", 100, Some(conflicting), 30);
            assert_eq!(
                file.observe(FileObservation::ConfirmedSameFile(observation.clone())),
                Err(if initial == FileStatus::PossibleReplacement {
                    DomainError::UnresolvedReplacement
                } else {
                    DomainError::IdentityMismatch
                })
            );
            assert_eq!(file, snapshot);
            assert_eq!(
                file.recover_verified_same_file(observation),
                Err(DomainError::IdentityMismatch)
            );
            assert_eq!(file, snapshot);
        }
        file.recover_verified_same_file(same("old", "original.pdf", 0, None, 31))
            .unwrap();
        assert_eq!(file.identity(), Some(&evidence(1)));
    }
}

#[test]
fn absent_identity_can_be_attached_only_through_supplied_same_file_classification() {
    let mut file = CanonicalFile::new(
        CanonicalFileId::new("id").unwrap(),
        location("old", "original.pdf"),
        None,
        FileMetadata::new(0, None),
        at(10),
    );
    assert!(file.identity().is_none());
    file.observe(FileObservation::ConfirmedSameFile(same(
        "old",
        "original.pdf",
        0,
        Some(evidence(1)),
        20,
    )))
    .unwrap();
    assert_eq!(file.identity(), Some(&evidence(1)));
}

#[test]
fn unavailable_and_replacement_states_allow_alias_metadata_management() {
    for observation in [
        FileObservation::Unavailable {
            reason: UnavailableReason::PermissionDenied,
            observed_at: at(20),
        },
        FileObservation::PossibleReplacement {
            observed_at: at(20),
        },
    ] {
        let mut file = populated();
        file.observe(observation).unwrap();
        let original = file.clone();
        file.create_alias(alias_id("new"), name("new.exe"), at(21))
            .unwrap();
        file.rename_alias(&alias_id("a"), name("renamed.txt"), at(22))
            .unwrap();
        file.delete_alias(&alias_id("b")).unwrap();
        assert_canonical_unchanged(&file, &original);
    }
}

#[test]
fn old_modification_time_and_missing_time_are_valid_and_metadata_is_not_identity() {
    let metadata = FileMetadata::new(0, Some(at(1)));
    assert_eq!(metadata.modified_at(), Some(at(1)));
    let mut file = CanonicalFile::new(
        CanonicalFileId::new("id").unwrap(),
        location("old", "original.pdf"),
        Some(evidence(1)),
        metadata,
        at(10),
    );
    let supplied = SameFileObservation::new(
        file.location().clone(),
        FileMetadata::new(0, Some(at(2))),
        Some(evidence(1)),
        at(20),
    );
    assert_eq!(
        file.observe(FileObservation::ConfirmedSameFile(supplied))
            .unwrap(),
        FileChange::Modified
    );
    let snapshot = file.clone();
    let conflicting = SameFileObservation::new(
        file.location().clone(),
        file.metadata().clone(),
        Some(evidence(2)),
        at(21),
    );
    assert_eq!(
        file.observe(FileObservation::ConfirmedSameFile(conflicting)),
        Err(DomainError::IdentityMismatch)
    );
    assert_eq!(file, snapshot);
    assert_eq!(FileMetadata::new(u64::MAX, None).size(), u64::MAX);
}

#[test]
fn errors_display_categories_without_sensitive_values() {
    for error in [
        DomainError::InvalidRecordId,
        DomainError::InvalidLocation,
        DomainError::InvalidIdentityEvidence,
        DomainError::InvalidTimestamp,
        DomainError::InvalidAliasName(AliasNameError::DeviceName),
        DomainError::DuplicateAliasName,
        DomainError::DuplicateAliasId,
        DomainError::AliasNotFound,
        DomainError::IdentityMismatch,
        DomainError::UnresolvedReplacement,
    ] {
        assert!(!error.to_string().is_empty());
        let as_error: &dyn std::error::Error = &error;
        assert!(as_error.source().is_none());
    }
}

#[test]
fn recovery_preserves_all_alias_fields_across_every_location_and_metadata_combination() {
    for replacement in [false, true] {
        for (parent, filename, location_change) in [
            ("old", "original.pdf", LocationChange::Unchanged),
            ("old", "renamed.pdf", LocationChange::Renamed),
            ("new", "original.pdf", LocationChange::Moved),
            ("new", "renamed.pdf", LocationChange::RenamedAndMoved),
        ] {
            for metadata_changed in [false, true] {
                let mut file = populated();
                let original = file.clone();
                let unresolved = if replacement {
                    FileObservation::PossibleReplacement {
                        observed_at: at(15),
                    }
                } else {
                    FileObservation::Unavailable {
                        reason: UnavailableReason::NotFound,
                        observed_at: at(15),
                    }
                };
                file.observe(unresolved).unwrap();
                let observation = same(
                    parent,
                    filename,
                    u64::from(metadata_changed),
                    Some(evidence(1)),
                    20,
                );
                let change = if replacement {
                    file.recover_verified_same_file(observation).unwrap()
                } else {
                    file.observe(FileObservation::ConfirmedSameFile(observation))
                        .unwrap()
                };
                assert_eq!(
                    change,
                    FileChange::Recovered {
                        location_change,
                        metadata_changed
                    }
                );
                assert_eq!(
                    file.status(),
                    if metadata_changed {
                        FileStatus::Modified
                    } else {
                        FileStatus::Available
                    }
                );
                assert_eq!(file.id(), original.id());
                assert_eq!(file.aliases(), original.aliases());
                assert_eq!(file.identity(), original.identity());
                assert_eq!(file.registered_at(), original.registered_at());
            }
        }
    }
}
