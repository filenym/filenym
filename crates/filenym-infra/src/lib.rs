//! Implementations of FileNym's external ports belong here.
//!
//! SQLite stores registry metadata behind the persistence contract. Filesystem
//! identity adapters and desktop composition remain deferred.

mod sqlite;
pub use sqlite::SqliteRepository;

#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod test_support;
