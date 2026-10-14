use std::time::SystemTime;

/// Supplied metadata, not a content hash or physical identity proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileMetadata {
    size: u64,
    modified_at: Option<SystemTime>,
}

impl FileMetadata {
    pub fn new(size: u64, modified_at: Option<SystemTime>) -> Self {
        Self { size, modified_at }
    }
    pub fn size(&self) -> u64 {
        self.size
    }
    pub fn modified_at(&self) -> Option<SystemTime> {
        self.modified_at
    }
}
