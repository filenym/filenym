use filenym_core::DomainError;
use filenym_ports::PersistenceError;
use std::{error::Error, fmt};

/// Safe categories without request contents or engine-specific diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationError {
    InvalidCanonicalId,
    InvalidAliasId,
    CanonicalNotFound,
    AliasNotFound,
    DuplicateCanonicalId,
    DuplicateAliasId,
    DuplicateAliasName,
    Conflict,
    Domain(DomainError),
    Storage(PersistenceError),
}

impl From<DomainError> for ApplicationError {
    fn from(error: DomainError) -> Self {
        match error {
            DomainError::AliasNotFound => Self::AliasNotFound,
            DomainError::DuplicateAliasId => Self::DuplicateAliasId,
            DomainError::DuplicateAliasName => Self::DuplicateAliasName,
            other => Self::Domain(other),
        }
    }
}

impl From<PersistenceError> for ApplicationError {
    fn from(error: PersistenceError) -> Self {
        match error {
            PersistenceError::NotFound => Self::CanonicalNotFound,
            PersistenceError::DuplicateCanonicalId => Self::DuplicateCanonicalId,
            PersistenceError::DuplicateAliasId => Self::DuplicateAliasId,
            PersistenceError::DuplicateAliasName => Self::DuplicateAliasName,
            PersistenceError::Conflict => Self::Conflict,
            other => Self::Storage(other),
        }
    }
}

impl fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidCanonicalId => "invalid canonical ID",
            Self::InvalidAliasId => "invalid alias ID",
            Self::CanonicalNotFound => "canonical record not found",
            Self::AliasNotFound => "alias not found",
            Self::DuplicateCanonicalId => "canonical ID already stored",
            Self::DuplicateAliasId => "duplicate alias ID",
            Self::DuplicateAliasName => "duplicate alias name",
            Self::Conflict => "storage revision conflict; reload required",
            Self::Domain(error) => return error.fmt(f),
            Self::Storage(error) => return error.fmt(f),
        })
    }
}

impl Error for ApplicationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Domain(error) => Some(error),
            Self::Storage(error) => Some(error),
            _ => None,
        }
    }
}
