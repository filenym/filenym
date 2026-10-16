//! Metadata-only supported-input checks. No physical identity or continuity proof.
use filenym_core::{FileMetadata, KnownFileLocation};
use std::{fs::File, path::Path};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationError {
    NotFound,
    PermissionDenied,
    Unsupported,
    MetadataUnavailable,
}

pub struct RegistrationFacts {
    pub location: KnownFileLocation,
    pub metadata: FileMetadata,
    // Keep the attribute-only handle alive through registration, without denying sharing.
    _handle: File,
}

fn io_error(error: std::io::Error) -> RegistrationError {
    match error.kind() {
        std::io::ErrorKind::NotFound => RegistrationError::NotFound,
        std::io::ErrorKind::PermissionDenied => RegistrationError::PermissionDenied,
        _ => RegistrationError::MetadataUnavailable,
    }
}

fn modification_time(
    result: Result<std::time::SystemTime, std::io::Error>,
) -> Result<Option<std::time::SystemTime>, RegistrationError> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == std::io::ErrorKind::Unsupported => Ok(None),
        Err(error) => Err(io_error(error)),
    }
}

/// Reject cloud reparse objects until provider-safe handling is verified. This
/// intentionally implements the approved fail-closed fallback, never a data open.
pub fn inspect_registration_candidate(path: &Path) -> Result<RegistrationFacts, RegistrationError> {
    #[cfg(windows)]
    {
        windows::inspect(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err(RegistrationError::Unsupported)
    }
}

/// The application-owned SQLite directory must not be remote or redirected.
pub fn validate_storage_path(path: &Path) -> Result<(), RegistrationError> {
    #[cfg(windows)]
    {
        windows::storage_path(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err(RegistrationError::Unsupported)
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::{
        ffi::OsString,
        fs::OpenOptions,
        os::windows::{
            ffi::{OsStrExt, OsStringExt},
            fs::{MetadataExt, OpenOptionsExt},
            io::AsRawHandle,
        },
        path::{Component, PathBuf, Prefix},
    };
    use windows_sys::Win32::{
        Foundation::{HANDLE, INVALID_HANDLE_VALUE},
        Storage::FileSystem::*,
        System::WindowsProgramming::{DRIVE_FIXED, DRIVE_REMOVABLE},
    };

    fn local_drive(path: &Path) -> Result<(), RegistrationError> {
        let drive = match path.components().next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::Disk(d) | Prefix::VerbatimDisk(d) => d,
                _ => return Err(RegistrationError::Unsupported),
            },
            _ => return Err(RegistrationError::Unsupported),
        };
        if !path.is_absolute() {
            return Err(RegistrationError::Unsupported);
        }
        let root = [u16::from(drive), 58, 92, 0];
        // SAFETY: root is a NUL-terminated drive root, valid for the call.
        match unsafe { GetDriveTypeW(root.as_ptr()) } {
            DRIVE_FIXED | DRIVE_REMOVABLE => Ok(()),
            _ => Err(RegistrationError::Unsupported),
        }
    }

    pub(super) fn storage_path(path: &Path) -> Result<(), RegistrationError> {
        local_drive(path)?;
        for ancestor in path.ancestors() {
            match std::fs::symlink_metadata(ancestor) {
                Ok(m)
                    if m.file_attributes()
                        & (FILE_ATTRIBUTE_REPARSE_POINT
                            | FILE_ATTRIBUTE_OFFLINE
                            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS
                            | FILE_ATTRIBUTE_RECALL_ON_OPEN)
                        != 0 =>
                {
                    return Err(RegistrationError::Unsupported)
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(io_error(e)),
            }
        }
        Ok(())
    }

    fn supported_attributes(attributes: u32) -> bool {
        attributes
            & (FILE_ATTRIBUTE_DIRECTORY
                | FILE_ATTRIBUTE_DEVICE
                | FILE_ATTRIBUTE_REPARSE_POINT
                | FILE_ATTRIBUTE_OFFLINE
                | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
            == 0
    }

    fn query(path: &Path) -> Result<u32, RegistrationError> {
        let units: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut data: WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };
        // SAFETY: input is NUL-terminated and output storage is valid. Query one exact name.
        let handle = unsafe { FindFirstFileW(units.as_ptr(), &mut data) };
        if handle == INVALID_HANDLE_VALUE {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        // SAFETY: handle is owned by this call and closed exactly once.
        unsafe {
            FindClose(handle);
        }
        if data.dwFileAttributes & FILE_ATTRIBUTE_RECALL_ON_OPEN != 0 {
            return Err(RegistrationError::Unsupported);
        }
        Ok(data.dwFileAttributes)
    }

    pub(super) fn inspect(path: &Path) -> Result<RegistrationFacts, RegistrationError> {
        let location = KnownFileLocation::new(path).map_err(|_| RegistrationError::Unsupported)?;
        local_drive(path)?;
        let components: Vec<_> = path.components().collect();
        for component in components.iter().skip(1) {
            if let Component::Normal(value) = component {
                if value.encode_wide().any(|u| u == 58 || u == 42 || u == 63) {
                    return Err(RegistrationError::Unsupported);
                }
            }
        }
        // Only known local directory redirects are followed. read_link supports
        // junctions/symlinks and fails closed for unknown/cloud reparse tags.
        for ancestor in path.ancestors().skip(1).filter(|p| p.parent().is_some()) {
            let metadata = std::fs::symlink_metadata(ancestor).map_err(io_error)?;
            if metadata.file_attributes()
                & (FILE_ATTRIBUTE_OFFLINE
                    | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS
                    | FILE_ATTRIBUTE_RECALL_ON_OPEN)
                != 0
            {
                return Err(RegistrationError::Unsupported);
            }
            if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                let target =
                    std::fs::read_link(ancestor).map_err(|_| RegistrationError::Unsupported)?;
                let target = if target.is_absolute() {
                    target
                } else {
                    ancestor
                        .parent()
                        .ok_or(RegistrationError::Unsupported)?
                        .join(target)
                };
                // Reject remote/cloud/unknown redirect destinations before opening
                // through them. Nested redirect targets remain conservatively unsupported.
                storage_path(&target)?;
                if !std::fs::metadata(ancestor).map_err(io_error)?.is_dir() {
                    return Err(RegistrationError::Unsupported);
                }
            }
        }
        let attributes = query(path)?;
        if !supported_attributes(attributes) {
            return Err(RegistrationError::Unsupported);
        }
        // No read/write/create/truncate permission is requested, only file attributes.
        let handle = OpenOptions::new()
            .access_mode(FILE_READ_ATTRIBUTES)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)
            .map_err(io_error)?;
        inspect_opened(path, location, handle)
    }

    fn inspect_opened(
        path: &Path,
        location: KnownFileLocation,
        handle: File,
    ) -> Result<RegistrationFacts, RegistrationError> {
        let metadata = handle.metadata().map_err(io_error)?;
        if !metadata.is_file() || !supported_attributes(metadata.file_attributes()) {
            return Err(RegistrationError::Unsupported);
        }
        let mut destination = vec![0u16; 32768];
        // SAFETY: borrowed file handle remains alive; output capacity matches supplied length.
        let length = unsafe {
            GetFinalPathNameByHandleW(
                handle.as_raw_handle() as HANDLE,
                destination.as_mut_ptr(),
                destination.len() as u32,
                VOLUME_NAME_DOS,
            )
        } as usize;
        if length == 0 || length >= destination.len() {
            return Err(RegistrationError::Unsupported);
        }
        local_drive(&PathBuf::from(OsString::from_wide(&destination[..length])))?;
        if !supported_attributes(query(path)?) {
            return Err(RegistrationError::Unsupported);
        }
        let modified_at = modification_time(metadata.modified())?;
        Ok(RegistrationFacts {
            location,
            metadata: FileMetadata::new(metadata.len(), modified_at),
            _handle: handle,
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn reparse_offline_and_recall_are_never_data_open_candidates() {
            assert!(supported_attributes(FILE_ATTRIBUTE_NORMAL));
            for bit in [
                FILE_ATTRIBUTE_DIRECTORY,
                FILE_ATTRIBUTE_DEVICE,
                FILE_ATTRIBUTE_REPARSE_POINT,
                FILE_ATTRIBUTE_OFFLINE,
                FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
            ] {
                assert!(!supported_attributes(bit));
            }
        }

        #[test]
        fn deterministic_directory_reparse_substitution_and_disappearance_are_rejected() {
            let directory = crate::test_support::OwnedDirectory::new();
            let target = directory.path.join("target");
            std::fs::create_dir(&target).unwrap();
            for replacement in [0, 1, 2] {
                let path = directory.path.join(format!("candidate-{replacement}.txt"));
                std::fs::write(&path, b"owned sentinel").unwrap();
                let location = KnownFileLocation::new(&path).unwrap();
                assert!(supported_attributes(query(&path).unwrap()));
                let handle = OpenOptions::new()
                    .access_mode(FILE_READ_ATTRIBUTES)
                    .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
                    .open(&path)
                    .unwrap();
                std::fs::remove_file(&path).unwrap();
                if replacement == 1 {
                    std::fs::create_dir(&path).unwrap();
                }
                if replacement == 2 {
                    let output = std::process::Command::new("cmd")
                        .args(["/c", "mklink", "/J"])
                        .arg(&path)
                        .arg(&target)
                        .output()
                        .unwrap();
                    assert!(output.status.success());
                }
                assert!(inspect_opened(&path, location, handle).is_err());
                if replacement == 2 {
                    std::fs::remove_dir(&path).unwrap();
                }
            }
        }

        #[test]
        fn only_unsupported_modification_time_becomes_absent() {
            use std::io::{Error, ErrorKind};
            assert_eq!(
                modification_time(Ok(std::time::UNIX_EPOCH)),
                Ok(Some(std::time::UNIX_EPOCH))
            );
            assert_eq!(
                modification_time(Err(Error::from(ErrorKind::Unsupported))),
                Ok(None)
            );
            assert_eq!(
                modification_time(Err(Error::from(ErrorKind::PermissionDenied))),
                Err(RegistrationError::PermissionDenied)
            );
            assert_eq!(
                modification_time(Err(Error::from(ErrorKind::Other))),
                Err(RegistrationError::MetadataUnavailable)
            );
        }

        #[test]
        fn redirect_destination_prefixes_reject_remote_and_device_targets() {
            for path in [
                r"\\server\share\file",
                r"\\?\UNC\server\share\file",
                r"\\.\NUL",
                r"C:relative",
            ] {
                assert_eq!(
                    local_drive(Path::new(path)),
                    Err(RegistrationError::Unsupported)
                );
            }
        }
    }
}
