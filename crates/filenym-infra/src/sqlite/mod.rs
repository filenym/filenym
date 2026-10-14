mod codec;
mod connection;
mod migrations;
mod repository;

pub use repository::SqliteRepository;

use filenym_ports::PersistenceError;

fn map_error(error: rusqlite::Error) -> PersistenceError {
    use rusqlite::{Error as SqlError, ErrorCode};
    match error {
        SqlError::SqliteFailure(error, _) => match error.code {
            ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked => PersistenceError::Busy,
            ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase => {
                PersistenceError::CorruptDatabase
            }
            ErrorCode::PermissionDenied | ErrorCode::ReadOnly => PersistenceError::PermissionDenied,
            ErrorCode::CannotOpen => PersistenceError::Unavailable,
            ErrorCode::ConstraintViolation => PersistenceError::InvalidStoredData,
            _ => PersistenceError::StorageFailure,
        },
        SqlError::InvalidColumnType(..)
        | SqlError::FromSqlConversionFailure(..)
        | SqlError::IntegralValueOutOfRange(..) => PersistenceError::InvalidStoredData,
        _ => PersistenceError::StorageFailure,
    }
}
