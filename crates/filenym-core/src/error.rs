use std::fmt;

/// Reasons a logical filename fails the conservative, host-independent policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AliasNameError {
    Empty,
    Whitespace,
    TraversalOrSeparator,
    ForbiddenCharacter,
    DeviceName,
    TrailingPeriod,
    TooLong,
}

/// Domain failures contain no user paths, names, or identity tokens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DomainError {
    InvalidRecordId,
    InvalidLocation,
    InvalidIdentityEvidence,
    InvalidTimestamp,
    InvalidAliasName(AliasNameError),
    DuplicateAliasName,
    DuplicateAliasId,
    AliasNotFound,
    IdentityMismatch,
    UnresolvedReplacement,
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAliasName(reason) => write!(f, "invalid alias name: {reason:?}"),
            _ => f.write_str(match self {
                Self::InvalidRecordId => "invalid record ID",
                Self::InvalidLocation => "invalid known file location",
                Self::InvalidIdentityEvidence => "invalid identity evidence",
                Self::InvalidTimestamp => "timestamp precedes record creation",
                Self::DuplicateAliasName => "duplicate alias name",
                Self::DuplicateAliasId => "duplicate alias ID",
                Self::AliasNotFound => "alias not found",
                Self::IdentityMismatch => "identity evidence conflicts",
                Self::UnresolvedReplacement => {
                    "possible replacement requires explicit same-file recovery"
                }
                Self::InvalidAliasName(_) => unreachable!(),
            }),
        }
    }
}

impl std::error::Error for DomainError {}
