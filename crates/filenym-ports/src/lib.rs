//! Interfaces for capabilities required by FileNym application services belong here.
//!
//! The aggregate persistence contract is defined here without engine types.
//! Additional ports will be introduced only with reviewed use cases.

mod persistence;
pub use persistence::{
    CanonicalFileRepository, PersistenceError, PersistenceRevision, StoredCanonicalFile,
};
