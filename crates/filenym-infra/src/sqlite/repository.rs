use super::{codec, connection, map_error};
use filenym_core::{
    AliasId, AliasName, AliasSnapshot, CanonicalFile, CanonicalFileId, CanonicalFileSnapshot,
    FileIdentity, FileMetadata,
};
use filenym_ports::{
    CanonicalFileRepository, PersistenceError as Error, PersistenceRevision, StoredCanonicalFile,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use std::{path::Path, time::Duration};

/// Serial synchronous adapter. No raw SQL/connection access is public.
pub struct SqliteRepository {
    conn: Option<Connection>,
}

impl SqliteRepository {
    pub fn create_new(path: impl AsRef<Path>) -> Result<Self, Error> {
        Self::create_new_with_timeout(path, Duration::from_secs(5))
    }
    pub fn open_existing(path: impl AsRef<Path>) -> Result<Self, Error> {
        Self::open_existing_with_timeout(path, Duration::from_secs(5))
    }
    /// Only lock wait is configurable; durability/foreign keys cannot be weakened.
    pub fn create_new_with_timeout(
        path: impl AsRef<Path>,
        timeout: Duration,
    ) -> Result<Self, Error> {
        Ok(Self {
            conn: Some(connection::open(path.as_ref(), true, timeout)?),
        })
    }
    pub fn open_existing_with_timeout(
        path: impl AsRef<Path>,
        timeout: Duration,
    ) -> Result<Self, Error> {
        Ok(Self {
            conn: Some(connection::open(path.as_ref(), false, timeout)?),
        })
    }
    fn conn(&mut self) -> Result<&mut Connection, Error> {
        self.conn.as_mut().ok_or(Error::UncertainOutcome)
    }
    fn finish<T>(&mut self, result: Result<T, Error>) -> Result<T, Error> {
        if matches!(result, Err(Error::UncertainOutcome)) {
            self.conn.take();
        }
        result
    }
}

// Own bound values are prepared before transactions; no lossy native codecs.
struct EncodedFile {
    path_tag: &'static str,
    path: Vec<u8>,
    modified: Option<(i64, i64)>,
    registered: (i64, i64),
    observed: (i64, i64),
    aliases: Vec<EncodedAlias>,
}
struct EncodedAlias {
    created: (i64, i64),
    updated: (i64, i64),
    position: i64,
}
impl EncodedFile {
    fn new(file: &CanonicalFile) -> Result<Self, Error> {
        let (path_tag, path) = codec::encode_path(file.location())?;
        Ok(Self {
            path_tag,
            path,
            modified: file
                .metadata()
                .modified_at()
                .map(codec::encode_time)
                .transpose()?,
            registered: codec::encode_time(file.registered_at())?,
            observed: codec::encode_time(file.last_observed_at())?,
            aliases: file
                .aliases()
                .iter()
                .enumerate()
                .map(|(index, alias)| {
                    Ok(EncodedAlias {
                        created: codec::encode_time(alias.created_at())?,
                        updated: codec::encode_time(alias.updated_at())?,
                        position: i64::try_from(index).map_err(|_| Error::UnrepresentableValue)?,
                    })
                })
                .collect::<Result<_, Error>>()?,
        })
    }
}

const INSERT_PARENT:&str="INSERT INTO canonical_files(id,path_encoding,path_value,identity_scheme,identity_value,size_u64_be,modified_seconds,modified_nanoseconds,registered_seconds,registered_nanoseconds,observed_seconds,observed_nanoseconds,status,unavailable_reason,revision) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)";
const UPDATE_PARENT:&str="UPDATE canonical_files SET path_encoding=?2,path_value=?3,identity_scheme=?4,identity_value=?5,size_u64_be=?6,modified_seconds=?7,modified_nanoseconds=?8,registered_seconds=?9,registered_nanoseconds=?10,observed_seconds=?11,observed_nanoseconds=?12,status=?13,unavailable_reason=?14,revision=?15 WHERE id=?1 AND revision=?16";

fn parent(
    tx: &Transaction<'_>,
    file: &CanonicalFile,
    encoded: &EncodedFile,
    revision: PersistenceRevision,
    expected: Option<PersistenceRevision>,
) -> Result<(), Error> {
    let (status, reason) = codec::encode_status(file.status());
    let mut values: Vec<rusqlite::types::Value> = vec![
        file.id().as_str().to_owned().into(),
        encoded.path_tag.to_owned().into(),
        encoded.path.clone().into(),
        file.identity()
            .map(|i| i.scheme().to_owned().into())
            .unwrap_or(rusqlite::types::Value::Null),
        file.identity()
            .map(|i| i.value().to_vec().into())
            .unwrap_or(rusqlite::types::Value::Null),
        file.metadata().size().to_be_bytes().to_vec().into(),
        encoded
            .modified
            .map(|p| p.0.into())
            .unwrap_or(rusqlite::types::Value::Null),
        encoded
            .modified
            .map(|p| p.1.into())
            .unwrap_or(rusqlite::types::Value::Null),
        encoded.registered.0.into(),
        encoded.registered.1.into(),
        encoded.observed.0.into(),
        encoded.observed.1.into(),
        status.to_owned().into(),
        reason
            .map(|r| r.to_owned().into())
            .unwrap_or(rusqlite::types::Value::Null),
        revision.value().into(),
    ];
    let sql = if let Some(expected) = expected {
        values.push(expected.value().into());
        UPDATE_PARENT
    } else {
        INSERT_PARENT
    };
    let count = tx
        .execute(sql, rusqlite::params_from_iter(values))
        .map_err(|e| {
            if let rusqlite::Error::SqliteFailure(code, _) = &e {
                if code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY
                    && expected.is_none()
                {
                    return Error::DuplicateCanonicalId;
                }
            }
            map_error(e)
        })?;
    if count != 1 {
        return Err(Error::Conflict);
    }
    Ok(())
}

fn aliases(tx: &Transaction<'_>, file: &CanonicalFile, encoded: &EncodedFile) -> Result<(), Error> {
    for (alias, value) in file.aliases().iter().zip(&encoded.aliases) {
        tx.execute("INSERT INTO aliases(canonical_file_id,id,name,name_key,position,created_seconds,created_nanoseconds,updated_seconds,updated_nanoseconds) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![file.id().as_str(),alias.id().as_str(),alias.name().as_str(),alias.name().comparison_key().as_bytes(),value.position,value.created.0,value.created.1,value.updated.0,value.updated.1]).map_err(|e| {
            if let rusqlite::Error::SqliteFailure(code,Some(message))=&e {
                if code.extended_code==rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY { return Error::DuplicateAliasId; }
                if code.extended_code==rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE && message.contains("name_key") { return Error::DuplicateAliasName; }
            } map_error(e)
        })?;
    }
    Ok(())
}

fn complete<T>(tx: Transaction<'_>, result: Result<T, Error>) -> Result<T, Error> {
    match result {
        Ok(value) => match tx.execute_batch("COMMIT") {
            Ok(()) => Ok(value),
            Err(error) => {
                let error = map_error(error);
                let rolled_back = tx.rollback().is_ok();
                if rolled_back && matches!(error, Error::Busy | Error::InvalidStoredData) {
                    Err(error)
                } else {
                    Err(Error::UncertainOutcome)
                }
            }
        },
        Err(error) => {
            if tx.rollback().is_ok() {
                Err(error)
            } else {
                Err(Error::UncertainOutcome)
            }
        }
    }
}

struct CanonicalRow {
    path_tag: String,
    path: Vec<u8>,
    identity_scheme: Option<String>,
    identity_bytes: Option<Vec<u8>>,
    size: Vec<u8>,
    modified: (Option<i64>, Option<i64>),
    registered: (i64, i64),
    observed: (i64, i64),
    status: String,
    reason: Option<String>,
    revision: i64,
}

impl CanonicalRow {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            path_tag: row.get(0)?,
            path: row.get(1)?,
            identity_scheme: row.get(2)?,
            identity_bytes: row.get(3)?,
            size: row.get(4)?,
            modified: (row.get(5)?, row.get(6)?),
            registered: (row.get(7)?, row.get(8)?),
            observed: (row.get(9)?, row.get(10)?),
            status: row.get(11)?,
            reason: row.get(12)?,
            revision: row.get(13)?,
        })
    }
}

struct AliasRow {
    id: String,
    name: String,
    key: Vec<u8>,
    position: i64,
    created: (i64, i64),
    updated: (i64, i64),
}
impl AliasRow {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            key: row.get(2)?,
            position: row.get(3)?,
            created: (row.get(4)?, row.get(5)?),
            updated: (row.get(6)?, row.get(7)?),
        })
    }
}

fn read(conn: &Connection, id: &CanonicalFileId) -> Result<Option<StoredCanonicalFile>, Error> {
    let raw = conn
        .query_row(
            "SELECT path_encoding, path_value, identity_scheme, identity_value,
                size_u64_be, modified_seconds, modified_nanoseconds,
                registered_seconds, registered_nanoseconds,
                observed_seconds, observed_nanoseconds,
                status, unavailable_reason, revision
         FROM canonical_files WHERE id=?1",
            [id.as_str()],
            CanonicalRow::from_row,
        )
        .optional()
        .map_err(map_error)?;
    let Some(raw) = raw else { return Ok(None) };
    let location = codec::decode_path(&raw.path_tag, raw.path)?;
    let identity = match (raw.identity_scheme, raw.identity_bytes) {
        (None, None) => None,
        (Some(scheme), Some(bytes)) => {
            Some(FileIdentity::new(scheme, bytes).map_err(|_| Error::InvalidStoredData)?)
        }
        _ => return Err(Error::InvalidStoredData),
    };
    let size = u64::from_be_bytes(raw.size.try_into().map_err(|_| Error::InvalidStoredData)?);
    let metadata = FileMetadata::new(
        size,
        codec::decode_optional_time(raw.modified.0, raw.modified.1)?,
    );
    let canonical = CanonicalFile::new(
        id.clone(),
        location,
        identity,
        metadata,
        codec::decode_time(raw.registered.0, raw.registered.1)?,
    );
    let observed = codec::decode_time(raw.observed.0, raw.observed.1)?;
    let status = codec::decode_status(&raw.status, raw.reason.as_deref())?;
    let revision = PersistenceRevision::new(raw.revision)?;
    let mut statement = conn
        .prepare(
            "SELECT id, name, name_key, position, created_seconds, created_nanoseconds,
                updated_seconds, updated_nanoseconds
         FROM aliases WHERE canonical_file_id=?1 ORDER BY position",
        )
        .map_err(map_error)?;
    let rows = statement
        .query_map([id.as_str()], AliasRow::from_row)
        .map_err(map_error)?;
    let mut aliases = Vec::new();
    for (index, row) in rows.enumerate() {
        let row = row.map_err(map_error)?;
        let name = AliasName::new(row.name).map_err(|_| Error::InvalidStoredData)?;
        if name.comparison_key().as_bytes() != row.key
            || i64::try_from(index).map_err(|_| Error::InvalidStoredData)? != row.position
        {
            return Err(Error::InvalidStoredData);
        }
        aliases.push(
            AliasSnapshot::new(
                AliasId::new(row.id).map_err(|_| Error::InvalidStoredData)?,
                id.clone(),
                name,
                codec::decode_time(row.created.0, row.created.1)?,
                codec::decode_time(row.updated.0, row.updated.1)?,
            )
            .map_err(|_| Error::InvalidStoredData)?,
        );
    }
    let snapshot = CanonicalFileSnapshot::new(canonical, status, observed, aliases)
        .map_err(|_| Error::InvalidStoredData)?;
    Ok(Some(StoredCanonicalFile::new(
        CanonicalFile::restore(snapshot).map_err(|_| Error::InvalidStoredData)?,
        revision,
    )))
}

impl CanonicalFileRepository for SqliteRepository {
    fn insert(&mut self, file: &CanonicalFile) -> Result<PersistenceRevision, Error> {
        let encoded = EncodedFile::new(file)?;
        let tx = self
            .conn()?
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_error)?;
        let result = (|| {
            let revision = PersistenceRevision::new(0)?;
            parent(&tx, file, &encoded, revision, None)?;
            aliases(&tx, file, &encoded)?;
            Ok(revision)
        })();
        let result = complete(tx, result);
        self.finish(result)
    }
    fn load(&mut self, id: &CanonicalFileId) -> Result<Option<StoredCanonicalFile>, Error> {
        let tx = self.conn()?.transaction().map_err(map_error)?;
        let result = read(&tx, id);
        let result = complete(tx, result);
        self.finish(result)
    }
    fn update(
        &mut self,
        file: &CanonicalFile,
        expected: PersistenceRevision,
    ) -> Result<PersistenceRevision, Error> {
        let encoded = EncodedFile::new(file)?;
        let tx = self
            .conn()?
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_error)?;
        let result = (|| {
            let stored = read(&tx, file.id())?.ok_or(Error::NotFound)?;
            if stored.revision() != expected {
                return Err(Error::Conflict);
            }
            if stored.file().registered_at() != file.registered_at()
                || stored
                    .file()
                    .identity()
                    .is_some_and(|old| Some(old) != file.identity())
            {
                return Err(Error::InvalidAggregateChange);
            }
            let revision = expected.next()?;
            parent(&tx, file, &encoded, revision, Some(expected))?;
            tx.execute(
                "DELETE FROM aliases WHERE canonical_file_id=?1",
                [file.id().as_str()],
            )
            .map_err(map_error)?;
            aliases(&tx, file, &encoded)?;
            Ok(revision)
        })();
        let result = complete(tx, result);
        self.finish(result)
    }
    fn list_ids(&mut self) -> Result<Vec<CanonicalFileId>, Error> {
        let mut stmt = self
            .conn()?
            .prepare("SELECT id FROM canonical_files ORDER BY id COLLATE BINARY")
            .map_err(map_error)?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(map_error)?;
        rows.map(|row| {
            CanonicalFileId::new(row.map_err(map_error)?).map_err(|_| Error::InvalidStoredData)
        })
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{fixture, OwnedDirectory};
    #[test]
    fn every_connection_enforces_the_reviewed_safety_settings() {
        let directory = OwnedDirectory::new();
        for create in [true, false] {
            let connection =
                connection::open(&directory.db(), create, Duration::from_secs(5)).unwrap();
            for (pragma, expected) in [
                ("foreign_keys", 1),
                ("synchronous", 2),
                ("busy_timeout", 5000),
                ("trusted_schema", 0),
            ] {
                assert_eq!(
                    connection
                        .pragma_query_value(None, pragma, |row| row.get::<_, i64>(0))
                        .unwrap(),
                    expected
                );
            }
            assert_eq!(
                connection
                    .pragma_query_value(None, "journal_mode", |row| row.get::<_, String>(0))
                    .unwrap(),
                "wal"
            );
        }
    }
    #[test]
    fn unexpected_commit_state_retires_the_adapter() {
        let directory = OwnedDirectory::new();
        let mut repository = SqliteRepository::create_new(directory.db()).unwrap();
        let transaction = repository.conn().unwrap().transaction().unwrap();
        // Test-only disruption exercises the uncertain-outcome disposition.
        // It does not simulate or claim an actual hardware I/O failure.
        transaction.execute_batch("ROLLBACK").unwrap();
        let result = complete(transaction, Ok(()));
        assert_eq!(repository.finish(result), Err(Error::UncertainOutcome));
        assert_eq!(
            repository.insert(&fixture("one")),
            Err(Error::UncertainOutcome)
        );
        let mut reopened = SqliteRepository::open_existing(directory.db()).unwrap();
        assert!(reopened.list_ids().unwrap().is_empty());
    }
}
