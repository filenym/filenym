use crate::{
    Alias, AliasId, AliasName, CanonicalFileId, DomainError, FileChange, FileIdentity,
    FileMetadata, FileObservation, FileStatus, KnownFileLocation, LocationChange,
    SameFileObservation,
};
use std::time::SystemTime;

/// In-memory consistency boundary. Equality compares domain snapshots, not
/// physical files. All operations are metadata-only and failure-atomic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalFile {
    id: CanonicalFileId,
    location: KnownFileLocation,
    identity: Option<FileIdentity>,
    metadata: FileMetadata,
    registered_at: SystemTime,
    last_observed_at: SystemTime,
    status: FileStatus,
    aliases: Vec<Alias>,
}

impl CanonicalFile {
    /// Restores validated facts without asserting physical identity, replaying
    /// observations, or clearing a persisted lifecycle status.
    pub fn restore(snapshot: crate::CanonicalFileSnapshot) -> Result<Self, DomainError> {
        let mut file = snapshot.canonical;
        let mut aliases: Vec<Alias> = Vec::with_capacity(snapshot.aliases.len());
        for alias in snapshot.aliases {
            if alias.owner != file.id {
                return Err(DomainError::AliasOwnershipMismatch);
            }
            if alias.created_at < file.registered_at || alias.updated_at < alias.created_at {
                return Err(DomainError::InvalidTimestamp);
            }
            if aliases.iter().any(|existing| existing.id() == &alias.id) {
                return Err(DomainError::DuplicateAliasId);
            }
            if aliases
                .iter()
                .any(|existing| existing.name().key() == alias.name.key())
            {
                return Err(DomainError::DuplicateAliasName);
            }
            let mut restored =
                Alias::new(alias.id, alias.owner, alias.name.clone(), alias.created_at);
            restored.rename(alias.name, alias.updated_at);
            aliases.push(restored);
        }
        file.aliases = aliases;
        file.status = snapshot.status;
        file.last_observed_at = snapshot.observed_at;
        Ok(file)
    }
    /// Initial registration facts are supplied by a trusted caller; this does
    /// not register, inspect, or verify a physical file.
    pub fn new(
        id: CanonicalFileId,
        location: KnownFileLocation,
        identity: Option<FileIdentity>,
        metadata: FileMetadata,
        registered_at: SystemTime,
    ) -> Self {
        Self {
            id,
            location,
            identity,
            metadata,
            registered_at,
            last_observed_at: registered_at,
            status: FileStatus::Available,
            aliases: Vec::new(),
        }
    }
    pub fn id(&self) -> &CanonicalFileId {
        &self.id
    }
    pub fn location(&self) -> &KnownFileLocation {
        &self.location
    }
    pub fn identity(&self) -> Option<&FileIdentity> {
        self.identity.as_ref()
    }
    pub fn metadata(&self) -> &FileMetadata {
        &self.metadata
    }
    pub fn registered_at(&self) -> SystemTime {
        self.registered_at
    }
    pub fn last_observed_at(&self) -> SystemTime {
        self.last_observed_at
    }
    pub fn status(&self) -> FileStatus {
        self.status
    }
    pub fn aliases(&self) -> &[Alias] {
        &self.aliases
    }

    /// Creates metadata only. IDs and ASCII-folded names must be unique within
    /// this aggregate; the supplied timestamp must not precede registration.
    pub fn create_alias(
        &mut self,
        id: AliasId,
        name: AliasName,
        at: SystemTime,
    ) -> Result<(), DomainError> {
        if at < self.registered_at {
            return Err(DomainError::InvalidTimestamp);
        }
        if self.aliases.iter().any(|alias| alias.id() == &id) {
            return Err(DomainError::DuplicateAliasId);
        }
        if self
            .aliases
            .iter()
            .any(|alias| alias.name().key() == name.key())
        {
            return Err(DomainError::DuplicateAliasName);
        }
        self.aliases.push(Alias::new(id, self.id.clone(), name, at));
        Ok(())
    }

    /// Renames metadata only, preserving exact input. Identical spelling is a
    /// no-op; case-only changes update the supplied timestamp. It must not
    /// precede alias creation (wall-clock monotonicity is not assumed).
    pub fn rename_alias(
        &mut self,
        id: &AliasId,
        name: AliasName,
        at: SystemTime,
    ) -> Result<(), DomainError> {
        let index = self
            .aliases
            .iter()
            .position(|alias| alias.id() == id)
            .ok_or(DomainError::AliasNotFound)?;
        if at < self.aliases[index].created_at() {
            return Err(DomainError::InvalidTimestamp);
        }
        if self
            .aliases
            .iter()
            .any(|alias| alias.id() != id && alias.name().key() == name.key())
        {
            return Err(DomainError::DuplicateAliasName);
        }
        if self.aliases[index].name() != &name {
            self.aliases[index].rename(name, at);
        }
        Ok(())
    }

    /// Removes only the selected logical alias, never the canonical record.
    pub fn delete_alias(&mut self, id: &AliasId) -> Result<(), DomainError> {
        let index = self
            .aliases
            .iter()
            .position(|alias| alias.id() == id)
            .ok_or(DomainError::AliasNotFound)?;
        self.aliases.remove(index);
        Ok(())
    }

    /// Applies caller-classified facts, with no physical identity verification.
    /// Unavailable/replacement observations retain all accepted canonical facts
    /// and aliases. Observation timestamps may move backwards with wall clocks.
    pub fn observe(&mut self, observation: FileObservation) -> Result<FileChange, DomainError> {
        match observation {
            FileObservation::ConfirmedSameFile(observation) => {
                if self.status == FileStatus::PossibleReplacement {
                    return Err(DomainError::UnresolvedReplacement);
                }
                self.accept_same_file(observation)
            }
            FileObservation::Unavailable {
                reason,
                observed_at,
            } => {
                self.status = FileStatus::Unavailable(reason);
                self.last_observed_at = observed_at;
                Ok(FileChange::Unavailable(reason))
            }
            FileObservation::PossibleReplacement { observed_at } => {
                self.status = FileStatus::PossibleReplacement;
                self.last_observed_at = observed_at;
                Ok(FileChange::PossibleReplacement)
            }
        }
    }

    /// Explicit same-file recovery after external verification. This is not an
    /// intentional replacement API, and core does not verify the classification.
    pub fn recover_verified_same_file(
        &mut self,
        observation: SameFileObservation,
    ) -> Result<FileChange, DomainError> {
        self.accept_same_file(observation)
    }

    fn accept_same_file(
        &mut self,
        observation: SameFileObservation,
    ) -> Result<FileChange, DomainError> {
        if let (Some(stored), Some(supplied)) = (self.identity(), observation.identity()) {
            if stored != supplied {
                return Err(DomainError::IdentityMismatch);
            }
        }
        let renamed = self.location.filename() != observation.location().filename();
        let moved = self.location.as_path().parent() != observation.location().as_path().parent();
        let location_change = match (renamed, moved) {
            (false, false) => LocationChange::Unchanged,
            (true, false) => LocationChange::Renamed,
            (false, true) => LocationChange::Moved,
            (true, true) => LocationChange::RenamedAndMoved,
        };
        let metadata_changed = self.metadata != *observation.metadata();
        let recovered = matches!(
            self.status,
            FileStatus::Unavailable(_) | FileStatus::PossibleReplacement
        );
        let change = if recovered {
            FileChange::Recovered {
                location_change,
                metadata_changed,
            }
        } else {
            match location_change {
                LocationChange::Renamed => FileChange::Renamed { metadata_changed },
                LocationChange::Moved => FileChange::Moved { metadata_changed },
                LocationChange::RenamedAndMoved => FileChange::RenamedAndMoved { metadata_changed },
                LocationChange::Unchanged if metadata_changed => FileChange::Modified,
                LocationChange::Unchanged => FileChange::Unchanged,
            }
        };
        self.location = observation.location().clone();
        self.metadata = observation.metadata().clone();
        if let Some(identity) = observation.identity() {
            self.identity = Some(identity.clone());
        }
        self.last_observed_at = observation.observed_at();
        self.status = if metadata_changed {
            FileStatus::Modified
        } else {
            FileStatus::Available
        };
        Ok(change)
    }
}
