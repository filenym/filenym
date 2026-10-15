use crate::{AddAlias, ApplicationError, DeleteAlias, RegisterCanonicalFile, RenameAlias};
use filenym_core::{AliasId, AliasName, CanonicalFile, CanonicalFileId};
use filenym_ports::{CanonicalFileRepository, PersistenceRevision, StoredCanonicalFile};

/// Serial, metadata-only orchestration. The caller owns repository setup and recovery.
/// Responses are detached committed snapshots; no mutation is automatically retried.
pub struct FileRegistryService<'a, R: CanonicalFileRepository + ?Sized> {
    repository: &'a mut R,
}

impl<'a, R: CanonicalFileRepository + ?Sized> FileRegistryService<'a, R> {
    pub fn new(repository: &'a mut R) -> Self {
        Self { repository }
    }

    /// Stores supplied facts without filesystem inspection or identity verification.
    pub fn register_canonical_file(
        &mut self,
        request: RegisterCanonicalFile,
    ) -> Result<StoredCanonicalFile, ApplicationError> {
        let id = canonical_id(request.canonical_id)?;
        let file = CanonicalFile::new(
            id,
            request.location,
            request.identity,
            request.metadata,
            request.registered_at,
        );
        let revision = self.repository.insert(&file)?;
        Ok(StoredCanonicalFile::new(file, revision))
    }

    pub fn add_alias(
        &mut self,
        request: AddAlias,
    ) -> Result<StoredCanonicalFile, ApplicationError> {
        let id = canonical_id(request.canonical_id)?;
        let alias = alias_id(request.alias_id)?;
        let name = AliasName::new(request.name)?;
        let stored = self.load_current(&id, request.expected_revision)?;
        let revision = stored.revision();
        let mut file = stored.into_file();
        file.create_alias(alias, name, request.created_at)?;
        self.save(file, revision)
    }

    /// Identical spelling preserves alias fields but still performs a revision-checked write.
    pub fn rename_alias(
        &mut self,
        request: RenameAlias,
    ) -> Result<StoredCanonicalFile, ApplicationError> {
        let id = canonical_id(request.canonical_id)?;
        let alias = alias_id(request.alias_id)?;
        let name = AliasName::new(request.name)?;
        let stored = self.load_current(&id, request.expected_revision)?;
        let revision = stored.revision();
        let mut file = stored.into_file();
        file.rename_alias(&alias, name, request.updated_at)?;
        self.save(file, revision)
    }

    pub fn delete_alias(
        &mut self,
        request: DeleteAlias,
    ) -> Result<StoredCanonicalFile, ApplicationError> {
        let id = canonical_id(request.canonical_id)?;
        let alias = alias_id(request.alias_id)?;
        let stored = self.load_current(&id, request.expected_revision)?;
        let revision = stored.revision();
        let mut file = stored.into_file();
        file.delete_alias(&alias)?;
        self.save(file, revision)
    }

    pub fn get_canonical_file(
        &mut self,
        id: &str,
    ) -> Result<StoredCanonicalFile, ApplicationError> {
        let id = canonical_id(id)?;
        self.repository
            .load(&id)?
            .ok_or(ApplicationError::CanonicalNotFound)
    }

    /// Inventory only, including unavailable records; get validates complete stored state.
    pub fn list_registered_file_ids(&mut self) -> Result<Vec<CanonicalFileId>, ApplicationError> {
        Ok(self.repository.list_ids()?)
    }

    fn load_current(
        &mut self,
        id: &CanonicalFileId,
        expected: PersistenceRevision,
    ) -> Result<StoredCanonicalFile, ApplicationError> {
        let stored = self
            .repository
            .load(id)?
            .ok_or(ApplicationError::CanonicalNotFound)?;
        if stored.revision() != expected {
            return Err(ApplicationError::Conflict);
        }
        Ok(stored)
    }

    fn save(
        &mut self,
        file: CanonicalFile,
        expected: PersistenceRevision,
    ) -> Result<StoredCanonicalFile, ApplicationError> {
        let revision = self.repository.update(&file, expected)?;
        Ok(StoredCanonicalFile::new(file, revision))
    }
}

fn canonical_id(value: impl Into<String>) -> Result<CanonicalFileId, ApplicationError> {
    CanonicalFileId::new(value).map_err(|_| ApplicationError::InvalidCanonicalId)
}

fn alias_id(value: String) -> Result<AliasId, ApplicationError> {
    AliasId::new(value).map_err(|_| ApplicationError::InvalidAliasId)
}
