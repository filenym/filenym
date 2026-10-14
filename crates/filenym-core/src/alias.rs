use crate::{AliasId, AliasName, CanonicalFileId};
use std::time::SystemTime;

/// Read-only contextual name; mutation is controlled by its canonical aggregate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alias {
    id: AliasId,
    canonical_file_id: CanonicalFileId,
    name: AliasName,
    created_at: SystemTime,
    updated_at: SystemTime,
}

impl Alias {
    pub(crate) fn new(
        id: AliasId,
        canonical_file_id: CanonicalFileId,
        name: AliasName,
        at: SystemTime,
    ) -> Self {
        Self {
            id,
            canonical_file_id,
            name,
            created_at: at,
            updated_at: at,
        }
    }
    pub(crate) fn rename(&mut self, name: AliasName, at: SystemTime) {
        self.name = name;
        self.updated_at = at;
    }
    pub fn id(&self) -> &AliasId {
        &self.id
    }
    pub fn canonical_file_id(&self) -> &CanonicalFileId {
        &self.canonical_file_id
    }
    pub fn name(&self) -> &AliasName {
        &self.name
    }
    pub fn created_at(&self) -> SystemTime {
        self.created_at
    }
    pub fn updated_at(&self) -> SystemTime {
        self.updated_at
    }
}
