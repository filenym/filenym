//! FileNym's domain concepts and rules belong here.
//!
//! This crate stays independent of desktop transport, persistence, and platform APIs.

mod alias;
mod alias_name;
mod canonical_file;
mod error;
mod identity;
mod ids;
mod lifecycle;
mod location;
mod metadata;
mod rehydration;

pub use alias::Alias;
pub use alias_name::AliasName;
pub use canonical_file::CanonicalFile;
pub use error::{AliasNameError, DomainError};
pub use identity::FileIdentity;
pub use ids::{AliasId, CanonicalFileId};
pub use lifecycle::{
    FileChange, FileObservation, FileStatus, LocationChange, SameFileObservation, UnavailableReason,
};
pub use location::KnownFileLocation;
pub use metadata::FileMetadata;
pub use rehydration::{AliasSnapshot, CanonicalFileSnapshot};
