//! Validated reconstruction inputs, independent of persistence formats.
use crate::{AliasId, AliasName, CanonicalFile, CanonicalFileId, DomainError, FileStatus};
use std::time::SystemTime;

/// Immutable reconstruction input, not a mutable alias entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AliasSnapshot {
    pub(crate) id: AliasId,
    pub(crate) owner: CanonicalFileId,
    pub(crate) name: AliasName,
    pub(crate) created_at: SystemTime,
    pub(crate) updated_at: SystemTime,
}

impl AliasSnapshot {
    pub fn new(
        id: AliasId,
        owner: CanonicalFileId,
        name: AliasName,
        created_at: SystemTime,
        updated_at: SystemTime,
    ) -> Result<Self, DomainError> {
        if updated_at < created_at {
            return Err(DomainError::InvalidTimestamp);
        }
        Ok(Self {
            id,
            owner,
            name,
            created_at,
            updated_at,
        })
    }
}

/// Reconstruction of canonical facts, status/time and a complete ordered alias
/// collection. `canonical` supplies facts/registration; its aliases/status/time
/// are explicitly replaced by these inputs after validation, without events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalFileSnapshot {
    pub(crate) canonical: CanonicalFile,
    pub(crate) status: FileStatus,
    pub(crate) observed_at: SystemTime,
    pub(crate) aliases: Vec<AliasSnapshot>,
}

impl CanonicalFileSnapshot {
    pub fn new(
        canonical: CanonicalFile,
        status: FileStatus,
        observed_at: SystemTime,
        aliases: Vec<AliasSnapshot>,
    ) -> Result<Self, DomainError> {
        let snapshot = Self {
            canonical,
            status,
            observed_at,
            aliases,
        };
        CanonicalFile::restore(snapshot.clone())?;
        Ok(snapshot)
    }
}

impl From<&CanonicalFile> for CanonicalFileSnapshot {
    fn from(file: &CanonicalFile) -> Self {
        Self {
            canonical: file.clone(),
            status: file.status(),
            observed_at: file.last_observed_at(),
            aliases: file
                .aliases()
                .iter()
                .map(|alias| AliasSnapshot {
                    id: alias.id().clone(),
                    owner: alias.canonical_file_id().clone(),
                    name: alias.name().clone(),
                    created_at: alias.created_at(),
                    updated_at: alias.updated_at(),
                })
                .collect(),
        }
    }
}
