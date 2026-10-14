use filenym_core::*;
use std::time::{Duration, UNIX_EPOCH};
fn base() -> CanonicalFile {
    CanonicalFile::new(
        CanonicalFileId::new("file").unwrap(),
        KnownFileLocation::new("synthetic.pdf").unwrap(),
        Some(FileIdentity::new("fixture", vec![0, 1]).unwrap()),
        FileMetadata::new(u64::MAX, Some(UNIX_EPOCH - Duration::from_micros(1))),
        UNIX_EPOCH,
    )
}
fn alias(id: &str, owner: &str, name: &str, seconds: u64) -> AliasSnapshot {
    AliasSnapshot::new(
        AliasId::new(id).unwrap(),
        CanonicalFileId::new(owner).unwrap(),
        AliasName::new(name).unwrap(),
        UNIX_EPOCH + Duration::from_secs(seconds),
        UNIX_EPOCH + Duration::from_secs(seconds + 1),
    )
    .unwrap()
}
#[test]
fn complete_snapshots_restore_without_events() {
    for status in [
        FileStatus::Available,
        FileStatus::Modified,
        FileStatus::PossibleReplacement,
        FileStatus::Unavailable(UnavailableReason::NotFound),
        FileStatus::Unavailable(UnavailableReason::PermissionDenied),
        FileStatus::Unavailable(UnavailableReason::Unknown),
    ] {
        let snapshot = CanonicalFileSnapshot::new(
            base(),
            status,
            UNIX_EPOCH - Duration::from_micros(5),
            vec![
                alias("b", "file", "É.pdf", 2),
                alias("a", "file", "é.pdf", 1),
            ],
        )
        .unwrap();
        let file = CanonicalFile::restore(snapshot).unwrap();
        assert_eq!(file.status(), status);
        assert_eq!(
            file.last_observed_at(),
            UNIX_EPOCH - Duration::from_micros(5)
        );
        assert_eq!(file.aliases()[0].id().as_str(), "b");
        assert_eq!(file.aliases()[1].id().as_str(), "a");
        assert_eq!(
            file.aliases()[0].created_at(),
            UNIX_EPOCH + Duration::from_secs(2)
        );
        assert_eq!(
            file.aliases()[0].updated_at(),
            UNIX_EPOCH + Duration::from_secs(3)
        );
        assert_eq!(
            CanonicalFile::restore(CanonicalFileSnapshot::from(&file)).unwrap(),
            file
        );
        assert_eq!(file.location(), base().location());
        assert_eq!(file.identity(), base().identity());
        assert_eq!(file.metadata(), base().metadata());
    }
    assert_eq!(
        CanonicalFile::restore(CanonicalFileSnapshot::from(&base())).unwrap(),
        base()
    );
}
#[test]
fn restoration_rejects_invalid_ownership_duplicates_and_times() {
    for (aliases, error) in [
        (
            vec![alias("a", "other", "report.pdf", 1)],
            DomainError::AliasOwnershipMismatch,
        ),
        (
            vec![
                alias("a", "file", "one.pdf", 1),
                alias("a", "file", "two.pdf", 2),
            ],
            DomainError::DuplicateAliasId,
        ),
        (
            vec![
                alias("a", "file", "Report.pdf", 1),
                alias("b", "file", "REPORT.PDF", 2),
            ],
            DomainError::DuplicateAliasName,
        ),
    ] {
        assert_eq!(
            CanonicalFileSnapshot::new(base(), FileStatus::Available, UNIX_EPOCH, aliases),
            Err(error)
        );
    }
    assert_eq!(
        AliasSnapshot::new(
            AliasId::new("a").unwrap(),
            base().id().clone(),
            AliasName::new("one").unwrap(),
            UNIX_EPOCH,
            UNIX_EPOCH - Duration::from_micros(1)
        ),
        Err(DomainError::InvalidTimestamp)
    );
    let early = AliasSnapshot::new(
        AliasId::new("a").unwrap(),
        base().id().clone(),
        AliasName::new("one").unwrap(),
        UNIX_EPOCH - Duration::from_secs(1),
        UNIX_EPOCH,
    )
    .unwrap();
    assert_eq!(
        CanonicalFileSnapshot::new(base(), FileStatus::Available, UNIX_EPOCH, vec![early]),
        Err(DomainError::InvalidTimestamp)
    );
}
#[test]
fn comparison_key_is_exact_ascii_policy() {
    assert_eq!(
        AliasName::new("REPORT.PDF").unwrap().comparison_key(),
        "report.pdf"
    );
    assert_ne!(
        AliasName::new("É.pdf").unwrap().comparison_key(),
        AliasName::new("é.pdf").unwrap().comparison_key()
    );
    assert_ne!(
        AliasName::new("é.pdf").unwrap().comparison_key(),
        AliasName::new("e\u{301}.pdf").unwrap().comparison_key()
    );
}
