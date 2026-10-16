//! Application-owned startup only. No reset, repair, schema changes or user file access.
use crate::transport::{Code, CommandErrorDto};
use filenym_infra::SqliteRepository;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

const SENTINEL: &[u8] = b"FileNym registry initialization v1\n";
fn io_error(_: std::io::Error) -> CommandErrorDto {
    CommandErrorDto::new(Code::DatabaseUnavailable)
}

pub fn open(root: &Path, allow_first_creation: bool) -> Result<SqliteRepository, CommandErrorDto> {
    filenym_infra::registration::validate_storage_path(root)
        .map_err(|_| CommandErrorDto::new(Code::DatabaseUnavailable))?;
    fs::create_dir_all(root).map_err(io_error)?;
    let sentinel = root.join("registry-initialization-v1");
    let database = root.join("filenym.sqlite3");
    for path in [&sentinel, &database] {
        if let Ok(m) = fs::symlink_metadata(path) {
            if !m.is_file() || m.file_type().is_symlink() {
                return Err(CommandErrorDto::new(Code::DatabaseUnavailable));
            }
            filenym_infra::registration::validate_storage_path(path)
                .map_err(|_| CommandErrorDto::new(Code::DatabaseUnavailable))?;
        }
    }
    if fs::metadata(&sentinel).is_ok_and(|m| m.len() != SENTINEL.len() as u64) {
        return Err(CommandErrorDto::new(Code::IncompatibleDatabase));
    }
    let marker = match fs::read(&sentinel) {
        Ok(value) => {
            if value != SENTINEL {
                return Err(CommandErrorDto::new(Code::IncompatibleDatabase));
            }
            true
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => return Err(io_error(e)),
    };
    let exists = match fs::symlink_metadata(&database) {
        Ok(_) => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => return Err(io_error(e)),
    };
    if exists {
        let repository = SqliteRepository::open_existing(&database)?;
        if !marker {
            write_sentinel(&sentinel)?;
        }
        return Ok(repository);
    }
    if marker || !allow_first_creation {
        return Err(CommandErrorDto::new(Code::DatabaseUnavailable));
    }
    write_sentinel(&sentinel)?;
    Ok(SqliteRepository::create_new(database)?)
}
fn write_sentinel(path: &Path) -> Result<(), CommandErrorDto> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(io_error)?;
    file.write_all(SENTINEL).map_err(io_error)?;
    file.sync_all().map_err(io_error)
}
