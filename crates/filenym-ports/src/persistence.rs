use filenym_core::{CanonicalFile, CanonicalFileId};
use std::fmt;

/// Storage coordination only; unrelated to file identity or domain timestamps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersistenceRevision(i64);
impl PersistenceRevision {
    pub fn new(value: i64) -> Result<Self, PersistenceError> {
        if value < 0 {
            return Err(PersistenceError::InvalidStoredData);
        }
        Ok(Self(value))
    }
    pub fn value(self) -> i64 {
        self.0
    }
    pub fn next(self) -> Result<Self, PersistenceError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(PersistenceError::UnrepresentableValue)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredCanonicalFile {
    file: CanonicalFile,
    revision: PersistenceRevision,
}
impl StoredCanonicalFile {
    pub fn new(file: CanonicalFile, revision: PersistenceRevision) -> Self {
        Self { file, revision }
    }
    pub fn file(&self) -> &CanonicalFile {
        &self.file
    }
    pub fn revision(&self) -> PersistenceRevision {
        self.revision
    }
    pub fn into_file(self) -> CanonicalFile {
        self.file
    }
}

/// Safe categories: no engine types, SQL text, paths, or evidence bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersistenceError {
    DuplicateCanonicalId,
    DuplicateAliasId,
    DuplicateAliasName,
    NotFound,
    Conflict,
    InvalidAggregateChange,
    InvalidStoredData,
    CorruptDatabase,
    IncompatibleDatabase,
    DatabaseAlreadyExists,
    UnsupportedPathEncoding,
    UnrepresentableValue,
    Busy,
    PermissionDenied,
    Unavailable,
    MigrationFailed,
    StorageFailure,
    /// Commit/rollback I/O left durability uncertain. Reload via a fresh adapter;
    /// never blindly retry or claim the previous snapshot is intact.
    UncertainOutcome,
}
impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DuplicateCanonicalId => "canonical ID already stored",
            Self::DuplicateAliasId => "alias ID already stored",
            Self::DuplicateAliasName => "alias name already stored",
            Self::NotFound => "canonical record not found",
            Self::Conflict => "storage revision conflict; reload required",
            Self::InvalidAggregateChange => "invalid canonical aggregate change",
            Self::InvalidStoredData => "invalid stored data",
            Self::CorruptDatabase => "database corruption detected",
            Self::IncompatibleDatabase => "incompatible database format",
            Self::DatabaseAlreadyExists => "database already exists",
            Self::UnsupportedPathEncoding => "unsupported native path encoding",
            Self::UnrepresentableValue => "value cannot be represented",
            Self::Busy => "database busy",
            Self::PermissionDenied => "database access denied",
            Self::Unavailable => "database unavailable",
            Self::MigrationFailed => "database migration failed",
            Self::StorageFailure => "storage operation failed",
            Self::UncertainOutcome => "storage outcome uncertain; reopen and reload required",
        })
    }
}
impl std::error::Error for PersistenceError {}

pub trait CanonicalFileRepository {
    fn insert(&mut self, file: &CanonicalFile) -> Result<PersistenceRevision, PersistenceError>;
    fn load(
        &mut self,
        id: &CanonicalFileId,
    ) -> Result<Option<StoredCanonicalFile>, PersistenceError>;
    fn update(
        &mut self,
        file: &CanonicalFile,
        expected: PersistenceRevision,
    ) -> Result<PersistenceRevision, PersistenceError>;
    /// IDs only; full aggregate validation occurs on load. Includes all statuses.
    fn list_ids(&mut self) -> Result<Vec<CanonicalFileId>, PersistenceError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn revisions_are_checked() {
        assert_eq!(
            PersistenceRevision::new(-1),
            Err(PersistenceError::InvalidStoredData)
        );
        assert_eq!(
            PersistenceRevision::new(i64::MAX).unwrap().next(),
            Err(PersistenceError::UnrepresentableValue)
        );
        assert_eq!(
            PersistenceRevision::new(0).unwrap().next().unwrap().value(),
            1
        );
    }
    #[test]
    fn errors_are_safe_categories() {
        assert_eq!(
            PersistenceError::UncertainOutcome.to_string(),
            "storage outcome uncertain; reopen and reload required"
        );
        assert!(std::error::Error::source(&PersistenceError::StorageFailure).is_none());
    }
}
