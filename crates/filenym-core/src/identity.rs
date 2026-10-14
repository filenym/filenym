use crate::DomainError;

/// Opaque caller-supplied filesystem evidence. Equality compares evidence only;
/// matching tokens do not prove continuity, uniqueness, or rename/move survival.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileIdentity {
    scheme: String,
    value: Vec<u8>,
}

impl FileIdentity {
    pub fn new(scheme: impl Into<String>, value: impl Into<Vec<u8>>) -> Result<Self, DomainError> {
        let scheme = scheme.into();
        let value = value.into();
        if scheme.trim().is_empty() || value.is_empty() {
            return Err(DomainError::InvalidIdentityEvidence);
        }
        Ok(Self { scheme, value })
    }
    pub fn scheme(&self) -> &str {
        &self.scheme
    }
    pub fn value(&self) -> &[u8] {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn evidence_requires_a_scheme_and_payload() {
        for (scheme, bytes) in [("", vec![1]), (" ", vec![1]), ("synthetic", vec![])] {
            assert_eq!(
                FileIdentity::new(scheme, bytes),
                Err(DomainError::InvalidIdentityEvidence)
            );
        }
        let evidence = FileIdentity::new("synthetic", vec![0, 1]).unwrap();
        assert_eq!(evidence.scheme(), "synthetic");
        assert_eq!(evidence.value(), &[0, 1]);
    }
}
