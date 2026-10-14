use crate::DomainError;
use std::{
    ffi::OsStr,
    path::{Component, Path, PathBuf},
};

/// Supplied host-native last known path. Checks are lexical only: no existence,
/// permissions, containment, symlink resolution, or physical identity guarantee.
/// The future caller is responsible for resolving paths safely.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnownFileLocation(PathBuf);

impl KnownFileLocation {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, DomainError> {
        let path = path.into();
        // Components normalizes interior dots; inspect encoded separators first.
        // ASCII separators cannot split a non-ASCII encoded character.
        let bytes = path.as_os_str().as_encoded_bytes();
        let without_prefix = match path.components().next() {
            Some(Component::Prefix(prefix)) => {
                &bytes[prefix.as_os_str().as_encoded_bytes().len()..]
            }
            _ => bytes,
        };
        let separator = |b: &u8| *b == b'/' || (cfg!(windows) && *b == b'\\');
        if bytes.is_empty()
            || bytes.contains(&0)
            || bytes.last().is_some_and(separator)
            || without_prefix
                .split(separator)
                .any(|part| part == b"." || part == b"..")
            || path.file_name().is_none()
            || path
                .components()
                .any(|part| matches!(part, Component::CurDir | Component::ParentDir))
        {
            return Err(DomainError::InvalidLocation);
        }
        Ok(Self(path))
    }
    pub fn as_path(&self) -> &Path {
        &self.0
    }
    pub fn filename(&self) -> &OsStr {
        self.0.file_name().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn locations_are_structural_not_filesystem_observations() {
        for path in ["", ".", "..", "a/./b", "a/../b", "a/", "/", "a\0b"] {
            assert_eq!(
                KnownFileLocation::new(path),
                Err(DomainError::InvalidLocation)
            );
        }
        #[cfg(windows)]
        for path in [
            "C:\\",
            "C:\\a\\.\\b",
            "C:\\a\\..\\b",
            "C:\\a\\",
            "C:.\\a",
            "C:..\\a",
        ] {
            assert!(KnownFileLocation::new(path).is_err(), "{path:?}");
        }
        let path = PathBuf::from("synthetic").join("no-file.pdf");
        let location = KnownFileLocation::new(&path).unwrap();
        assert_eq!(location.as_path(), path);
        assert_eq!(location.filename(), "no-file.pdf");
    }
}
