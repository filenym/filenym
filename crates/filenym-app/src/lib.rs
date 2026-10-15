//! FileNym use cases and application orchestration belong here.
//!
//! Services depend on domain concepts and ports, never desktop or infrastructure adapters.

mod error;
mod requests;
mod service;

pub use error::ApplicationError;
pub use requests::{AddAlias, DeleteAlias, RegisterCanonicalFile, RenameAlias};
pub use service::FileRegistryService;
