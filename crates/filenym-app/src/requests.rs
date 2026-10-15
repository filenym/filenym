use filenym_core::{FileIdentity, FileMetadata, KnownFileLocation};
use filenym_ports::PersistenceRevision;
use std::time::SystemTime;

/// Caller-supplied facts; registration does not inspect or verify a physical file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisterCanonicalFile {
    pub canonical_id: String,
    pub location: KnownFileLocation,
    pub identity: Option<FileIdentity>,
    pub metadata: FileMetadata,
    pub registered_at: SystemTime,
}

/// Raw strings are validated by the service before repository access.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddAlias {
    pub canonical_id: String,
    pub expected_revision: PersistenceRevision,
    pub alias_id: String,
    pub name: String,
    pub created_at: SystemTime,
}

/// Even identical spelling requires a conditional write and advances revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenameAlias {
    pub canonical_id: String,
    pub expected_revision: PersistenceRevision,
    pub alias_id: String,
    pub name: String,
    pub updated_at: SystemTime,
}

/// Removes alias metadata only, retaining the canonical record and physical file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeleteAlias {
    pub canonical_id: String,
    pub expected_revision: PersistenceRevision,
    pub alias_id: String,
}
