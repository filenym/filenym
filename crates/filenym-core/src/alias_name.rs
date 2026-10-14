use crate::{AliasNameError, DomainError};

/// Exact logical filename, limited to 255 UTF-8 bytes (not an OS limit).
/// Uniqueness folds ASCII case only: non-ASCII case and Unicode normalization
/// forms remain distinct. This does not model filesystem collation.
/// Extensions are preserved exactly, including explicitly different extensions.
///
/// Rejects empty names, edge whitespace, control characters, both path
/// separators, `.`/`..`, `< > : " | ? *`, trailing periods, and case-insensitive
/// Windows device stems (including extensions and superscript COM/LPT digits).
/// Internal spaces, dotfiles, extensionless and multi-period names are accepted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AliasName {
    value: String,
    key: String,
}

impl AliasName {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let fail = |reason| Err(DomainError::InvalidAliasName(reason));
        if value.is_empty() {
            return fail(AliasNameError::Empty);
        }
        if value.chars().any(char::is_control) {
            return fail(AliasNameError::ForbiddenCharacter);
        }
        if value.trim().is_empty() || value.trim() != value {
            return fail(AliasNameError::Whitespace);
        }
        if value == "." || value == ".." || value.contains(['/', '\\']) {
            return fail(AliasNameError::TraversalOrSeparator);
        }
        if value.chars().any(|c| "<>:\"|?*".contains(c)) {
            return fail(AliasNameError::ForbiddenCharacter);
        }
        if value.ends_with('.') {
            return fail(AliasNameError::TrailingPeriod);
        }
        if value.len() > 255 {
            return fail(AliasNameError::TooLong);
        }
        let stem = value
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        let numbered = stem
            .strip_prefix("COM")
            .or_else(|| stem.strip_prefix("LPT"));
        if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || numbered.is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        {
            return fail(AliasNameError::DeviceName);
        }
        let key = value.to_ascii_lowercase();
        Ok(Self { value, key })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
    pub(crate) fn key(&self) -> &str {
        &self.key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepted_names_are_preserved() {
        for value in [
            "Report.PDF",
            "internal space",
            "é.pdf",
            ".gitignore",
            "README",
            "report..pdf",
            "a.tar.gz",
            "different.exe",
            "COM10.txt",
        ] {
            assert_eq!(AliasName::new(value).unwrap().as_str(), value);
        }
    }
    #[test]
    fn invalid_forms_have_typed_reasons() {
        for (values, reason) in [
            (vec![""], AliasNameError::Empty),
            (
                vec![" ", "\u{2003}", " a", "a\u{2003}"],
                AliasNameError::Whitespace,
            ),
            (
                vec![".", "..", "../a", "a/b", "a\\b"],
                AliasNameError::TraversalOrSeparator,
            ),
            (
                vec![
                    "a<",
                    "a>",
                    "a:",
                    "a\"",
                    "a|",
                    "a?",
                    "a*",
                    "a\0",
                    "a\n",
                    "a\u{0085}",
                ],
                AliasNameError::ForbiddenCharacter,
            ),
            (vec!["a."], AliasNameError::TrailingPeriod),
            (
                vec![
                    "con",
                    "PRN.pdf",
                    "aux",
                    "Nul.tar.gz",
                    "COM1",
                    "com9.txt",
                    "LPT1",
                    "lpt9.txt",
                    "COM¹",
                    "LPT².pdf",
                    "COM³.txt",
                ],
                AliasNameError::DeviceName,
            ),
        ] {
            for value in values {
                assert_eq!(
                    AliasName::new(value),
                    Err(DomainError::InvalidAliasName(reason)),
                    "{value:?}"
                );
            }
        }
        for prefix in ["COM", "LPT"] {
            for suffix in ["1", "2", "3", "4", "5", "6", "7", "8", "9", "¹", "²", "³"] {
                for extension in ["", ".pdf", ".tar.gz"] {
                    assert_eq!(
                        AliasName::new(format!("{prefix}{suffix}{extension}")),
                        Err(DomainError::InvalidAliasName(AliasNameError::DeviceName))
                    );
                }
            }
        }
    }
    #[test]
    fn byte_limits_and_unicode_comparison_are_explicit() {
        assert!(AliasName::new("a".repeat(255)).is_ok());
        assert_eq!(
            AliasName::new("a".repeat(256)),
            Err(DomainError::InvalidAliasName(AliasNameError::TooLong))
        );
        assert!(AliasName::new(format!("{}a", "é".repeat(127))).is_ok());
        assert!(AliasName::new("é".repeat(128)).is_err());
        assert_eq!(AliasName::new("Report.PDF").unwrap().key(), "report.pdf");
        assert_ne!(
            AliasName::new("É").unwrap().key(),
            AliasName::new("é").unwrap().key()
        );
        assert_ne!(
            AliasName::new("é").unwrap().key(),
            AliasName::new("e\u{301}").unwrap().key()
        );
    }
}
