use super::{map_error, migrations};
use filenym_ports::PersistenceError as Error;
use rusqlite::{Connection, OpenFlags};
use std::{fs::OpenOptions, path::Path, time::Duration};

pub(super) fn open(path: &Path, create: bool, timeout: Duration) -> Result<Connection, Error> {
    if !path.is_absolute() || path.file_name().is_none() {
        return Err(Error::Unavailable);
    }
    if path.as_os_str().to_str().is_none_or(|s| s.contains('\0')) {
        return Err(Error::UnsupportedPathEncoding);
    }
    if timeout > Duration::from_secs(5) {
        return Err(Error::UnrepresentableValue);
    }
    if create {
        drop(
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(|e| match e.kind() {
                    std::io::ErrorKind::AlreadyExists => Error::DatabaseAlreadyExists,
                    std::io::ErrorKind::PermissionDenied => Error::PermissionDenied,
                    std::io::ErrorKind::NotFound => Error::Unavailable,
                    _ => Error::StorageFailure,
                })?,
        );
    }
    let mut conn =
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(map_error)?;
    if conn.is_readonly("main").map_err(map_error)? {
        return Err(Error::PermissionDenied);
    }
    conn.busy_timeout(timeout).map_err(map_error)?;
    conn.pragma_update(None, "foreign_keys", true)
        .map_err(map_error)?;
    if conn
        .pragma_query_value(None, "foreign_keys", |r| r.get::<_, i64>(0))
        .map_err(map_error)?
        != 1
    {
        return Err(Error::StorageFailure);
    }
    conn.pragma_update(None, "trusted_schema", false)
        .map_err(map_error)?;
    // Reject foreign/newer/damaged databases before persistent journal changes.
    if create {
        if migrations::identity(&conn)? != (0, 0) {
            return Err(Error::IncompatibleDatabase);
        }
    } else {
        migrations::verify(&conn)?;
    }
    let mode: String = conn
        .query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))
        .map_err(map_error)?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(Error::StorageFailure);
    }
    conn.pragma_update(None, "synchronous", "FULL")
        .map_err(map_error)?;
    if conn
        .pragma_query_value(None, "synchronous", |r| r.get::<_, i64>(0))
        .map_err(map_error)?
        != 2
    {
        return Err(Error::StorageFailure);
    }
    if create {
        migrations::initialize(&mut conn)?;
    } else {
        migrations::verify(&conn)?;
    }
    Ok(conn)
}
