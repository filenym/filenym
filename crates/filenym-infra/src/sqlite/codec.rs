use filenym_core::{FileStatus, KnownFileLocation, UnavailableReason};
use filenym_ports::PersistenceError as Error;
use std::{
    ffi::OsString,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub(super) fn encode_time(time: SystemTime) -> Result<(i64, i64), Error> {
    let total = match time.duration_since(UNIX_EPOCH) {
        Ok(d) => i128::from(d.as_secs()) * 1_000_000_000 + i128::from(d.subsec_nanos()),
        Err(e) => {
            -(i128::from(e.duration().as_secs()) * 1_000_000_000
                + i128::from(e.duration().subsec_nanos()))
        }
    };
    Ok((
        i64::try_from(total.div_euclid(1_000_000_000)).map_err(|_| Error::UnrepresentableValue)?,
        i64::try_from(total.rem_euclid(1_000_000_000)).map_err(|_| Error::UnrepresentableValue)?,
    ))
}

pub(super) fn decode_time(seconds: i64, nanos: i64) -> Result<SystemTime, Error> {
    if !(0..1_000_000_000).contains(&nanos) {
        return Err(Error::InvalidStoredData);
    }
    let total = i128::from(seconds) * 1_000_000_000 + i128::from(nanos);
    let magnitude = total.unsigned_abs();
    let duration = Duration::new(
        u64::try_from(magnitude / 1_000_000_000).map_err(|_| Error::InvalidStoredData)?,
        u32::try_from(magnitude % 1_000_000_000).map_err(|_| Error::InvalidStoredData)?,
    );
    let time = if total < 0 {
        UNIX_EPOCH.checked_sub(duration)
    } else {
        UNIX_EPOCH.checked_add(duration)
    }
    .ok_or(Error::InvalidStoredData)?;
    // Some hosts have coarser SystemTime precision (Windows: 100 ns).
    // Reject any conversion that would silently round a stored timestamp.
    if encode_time(time).map_err(|_| Error::InvalidStoredData)? != (seconds, nanos) {
        return Err(Error::InvalidStoredData);
    }
    Ok(time)
}

pub(super) fn decode_optional_time(
    seconds: Option<i64>,
    nanos: Option<i64>,
) -> Result<Option<SystemTime>, Error> {
    match (seconds, nanos) {
        (None, None) => Ok(None),
        (Some(s), Some(n)) => decode_time(s, n).map(Some),
        _ => Err(Error::InvalidStoredData),
    }
}

pub(super) fn encode_path(location: &KnownFileLocation) -> Result<(&'static str, Vec<u8>), Error> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        Ok((
            "windows_utf16le_v1",
            location
                .as_path()
                .as_os_str()
                .encode_wide()
                .flat_map(u16::to_le_bytes)
                .collect(),
        ))
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        Ok((
            "unix_bytes_v1",
            location.as_path().as_os_str().as_bytes().to_vec(),
        ))
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = location;
        Err(Error::UnsupportedPathEncoding)
    }
}

pub(super) fn decode_path(tag: &str, bytes: Vec<u8>) -> Result<KnownFileLocation, Error> {
    if bytes.is_empty() {
        return Err(Error::InvalidStoredData);
    }
    #[cfg(windows)]
    let path = {
        use std::os::windows::ffi::OsStringExt;
        if tag != "windows_utf16le_v1" {
            return Err(Error::UnsupportedPathEncoding);
        }
        if !bytes.len().is_multiple_of(2) {
            return Err(Error::InvalidStoredData);
        }
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|part| u16::from_le_bytes([part[0], part[1]]))
            .collect();
        PathBuf::from(OsString::from_wide(&units))
    };
    #[cfg(unix)]
    let path = {
        use std::os::unix::ffi::OsStringExt;
        if tag != "unix_bytes_v1" {
            return Err(Error::UnsupportedPathEncoding);
        }
        PathBuf::from(OsString::from_vec(bytes))
    };
    #[cfg(not(any(windows, unix)))]
    {
        let _ = (tag, bytes);
        return Err(Error::UnsupportedPathEncoding);
    }
    #[cfg(any(windows, unix))]
    KnownFileLocation::new(path).map_err(|_| Error::InvalidStoredData)
}

pub(super) fn encode_status(status: FileStatus) -> (&'static str, Option<&'static str>) {
    match status {
        FileStatus::Available => ("available", None),
        FileStatus::Modified => ("modified", None),
        FileStatus::PossibleReplacement => ("possible_replacement", None),
        FileStatus::Unavailable(reason) => (
            "unavailable",
            Some(match reason {
                UnavailableReason::NotFound => "not_found",
                UnavailableReason::PermissionDenied => "permission_denied",
                UnavailableReason::Unknown => "unknown",
            }),
        ),
    }
}
pub(super) fn decode_status(status: &str, reason: Option<&str>) -> Result<FileStatus, Error> {
    match (status, reason) {
        ("available", None) => Ok(FileStatus::Available),
        ("modified", None) => Ok(FileStatus::Modified),
        ("possible_replacement", None) => Ok(FileStatus::PossibleReplacement),
        ("unavailable", Some(reason)) => Ok(FileStatus::Unavailable(match reason {
            "not_found" => UnavailableReason::NotFound,
            "permission_denied" => UnavailableReason::PermissionDenied,
            "unknown" => UnavailableReason::Unknown,
            _ => return Err(Error::InvalidStoredData),
        })),
        _ => Err(Error::InvalidStoredData),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_timestamp_boundaries() {
        for delta in [
            Duration::ZERO,
            Duration::new(1, 0),
            Duration::new(0, 1),
            Duration::new(0, 999_999_999),
            Duration::new(100, 555),
        ] {
            for time in [UNIX_EPOCH + delta, UNIX_EPOCH - delta] {
                let pair = encode_time(time).unwrap();
                assert_eq!(decode_time(pair.0, pair.1).unwrap(), time);
            }
        }
        assert_eq!(
            encode_time(UNIX_EPOCH - Duration::from_millis(500)).unwrap(),
            (-1, 500_000_000)
        );
        for nanos in [-1, 1_000_000_000] {
            assert_eq!(decode_time(0, nanos), Err(Error::InvalidStoredData));
        }
        assert_eq!(
            decode_optional_time(None, Some(0)),
            Err(Error::InvalidStoredData)
        );
        assert_eq!(
            decode_optional_time(Some(0), None),
            Err(Error::InvalidStoredData)
        );
        assert_eq!(decode_optional_time(None, None).unwrap(), None);
        #[cfg(windows)]
        {
            assert_eq!(decode_time(0, 1), Err(Error::InvalidStoredData));
            assert_eq!(decode_time(i64::MAX, 0), Err(Error::InvalidStoredData));
        }
        if let Some(time) = UNIX_EPOCH.checked_add(Duration::from_secs(i64::MAX.unsigned_abs() + 1))
        {
            assert_eq!(encode_time(time), Err(Error::UnrepresentableValue));
        }
    }
    #[test]
    fn native_paths_are_exact_and_invalid_encodings_rejected() {
        for path in ["relative/report.pdf", "Unicode-é.pdf"] {
            let location = KnownFileLocation::new(path).unwrap();
            let (tag, bytes) = encode_path(&location).unwrap();
            assert_eq!(
                decode_path(tag, bytes).unwrap().as_path().as_os_str(),
                location.as_path().as_os_str()
            );
        }
        assert_eq!(
            decode_path("future", vec![1, 2]),
            Err(Error::UnsupportedPathEncoding)
        );
        assert_eq!(decode_path("future", vec![]), Err(Error::InvalidStoredData));
        #[cfg(windows)]
        {
            use std::os::windows::ffi::{OsStrExt, OsStringExt};
            for path in [
                "C:\\docs\\é.pdf",
                "\\\\server\\share\\file.pdf",
                "\\\\?\\C:\\docs\\file.pdf",
            ] {
                let location = KnownFileLocation::new(path).unwrap();
                let (tag, bytes) = encode_path(&location).unwrap();
                assert_eq!(decode_path(tag, bytes).unwrap(), location);
            }
            let native = OsString::from_wide(&[0x61, 0xd800, 0x62]);
            let location = KnownFileLocation::new(PathBuf::from(native)).unwrap();
            let (tag, bytes) = encode_path(&location).unwrap();
            assert_eq!(
                decode_path(tag, bytes)
                    .unwrap()
                    .as_path()
                    .as_os_str()
                    .encode_wide()
                    .collect::<Vec<_>>(),
                vec![0x61, 0xd800, 0x62]
            );
            for bytes in [vec![1], vec![0, 0], vec![46, 0, 46, 0]] {
                assert_eq!(
                    decode_path("windows_utf16le_v1", bytes),
                    Err(Error::InvalidStoredData)
                );
            }
            assert_eq!(
                decode_path("unix_bytes_v1", vec![1, 2]),
                Err(Error::UnsupportedPathEncoding)
            );
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            let location =
                KnownFileLocation::new(PathBuf::from(OsString::from_vec(vec![b'a', 255, b'b'])))
                    .unwrap();
            let (tag, bytes) = encode_path(&location).unwrap();
            assert_eq!(decode_path(tag, bytes).unwrap(), location);
            assert_eq!(
                decode_path("windows_utf16le_v1", vec![1, 2]),
                Err(Error::UnsupportedPathEncoding)
            );
        }
    }
    #[test]
    fn status_decoder_rejects_inconsistent_values() {
        for (status, reason) in [
            ("future", None),
            ("available", Some("unknown")),
            ("unavailable", None),
            ("unavailable", Some("future")),
        ] {
            assert_eq!(decode_status(status, reason), Err(Error::InvalidStoredData));
        }
    }
}
