use crate::{FileIdentity, FileMetadata, KnownFileLocation};
use std::time::SystemTime;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnavailableReason {
    NotFound,
    PermissionDenied,
    Unknown,
}

/// Modified describes changed metadata in the latest accepted observation,
/// not proof of changed bytes. Rename/move are change results, not statuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileStatus {
    Available,
    Modified,
    Unavailable(UnavailableReason),
    PossibleReplacement,
}

/// Trusted classification supplied by the caller after external verification.
/// Construction does NOT verify identity. Never infer this from a path, metadata,
/// watcher signal, or opaque evidence equality alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SameFileObservation {
    location: KnownFileLocation,
    metadata: FileMetadata,
    identity: Option<FileIdentity>,
    observed_at: SystemTime,
}

impl SameFileObservation {
    pub fn new(
        location: KnownFileLocation,
        metadata: FileMetadata,
        identity: Option<FileIdentity>,
        observed_at: SystemTime,
    ) -> Self {
        Self {
            location,
            metadata,
            identity,
            observed_at,
        }
    }
    pub fn location(&self) -> &KnownFileLocation {
        &self.location
    }
    pub fn metadata(&self) -> &FileMetadata {
        &self.metadata
    }
    pub fn identity(&self) -> Option<&FileIdentity> {
        self.identity.as_ref()
    }
    pub fn observed_at(&self) -> SystemTime {
        self.observed_at
    }
}

/// Classified inputs; domain core performs no filesystem observation itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileObservation {
    ConfirmedSameFile(SameFileObservation),
    Unavailable {
        reason: UnavailableReason,
        observed_at: SystemTime,
    },
    PossibleReplacement {
        observed_at: SystemTime,
    },
}

/// Lexical differences in externally supplied locations, not detected OS events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocationChange {
    Unchanged,
    Renamed,
    Moved,
    RenamedAndMoved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileChange {
    Unchanged,
    Modified,
    Renamed {
        metadata_changed: bool,
    },
    Moved {
        metadata_changed: bool,
    },
    RenamedAndMoved {
        metadata_changed: bool,
    },
    Unavailable(UnavailableReason),
    PossibleReplacement,
    Recovered {
        location_change: LocationChange,
        metadata_changed: bool,
    },
}
