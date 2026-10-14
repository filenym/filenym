use crate::DomainError;

fn validate(value: &str) -> Result<(), DomainError> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return Err(DomainError::InvalidRecordId);
    }
    Ok(())
}

/// Stable caller-supplied application record ID, unrelated to path or OS evidence.
///
/// ```compile_fail
/// use filenym_core::{AliasId, CanonicalFileId};
/// let canonical: CanonicalFileId = AliasId::new("alias").unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CanonicalFileId(String);

impl CanonicalFileId {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        validate(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Caller-supplied alias record ID. Global allocation belongs outside core.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AliasId(String);

impl AliasId {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        validate(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_are_validated_without_imposing_a_storage_format() {
        for value in ["", " ", "\u{2003}", " leading", "trailing ", "a\n", "a\0b"] {
            assert_eq!(
                CanonicalFileId::new(value),
                Err(DomainError::InvalidRecordId)
            );
            assert_eq!(AliasId::new(value), Err(DomainError::InvalidRecordId));
        }
        assert_eq!(
            CanonicalFileId::new("opaque:α/1").unwrap().as_str(),
            "opaque:α/1"
        );
        assert_eq!(AliasId::new("a").unwrap().as_str(), "a");
    }
}
