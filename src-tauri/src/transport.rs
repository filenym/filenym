//! Transport values only; domain entities are never serialized directly.
use filenym_app::ApplicationError;
use filenym_core::{AliasNameError, DomainError, FileStatus, UnavailableReason};
use filenym_ports::{PersistenceError, PersistenceRevision, StoredCanonicalFile};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Code {
    InvalidCanonicalId,
    InvalidAliasId,
    InvalidRequest,
    InvalidAliasName,
    DuplicateAliasName,
    DuplicateAliasId,
    DuplicateCanonicalId,
    AliasNotFound,
    CanonicalNotFound,
    Conflict,
    Busy,
    DatabaseUnavailable,
    PermissionDenied,
    CorruptDatabase,
    InvalidStoredData,
    IncompatibleDatabase,
    UnsupportedPathEncoding,
    UnrepresentableValue,
    UncertainOutcome,
    StorageFailure,
    InvalidAggregateChange,
    MigrationFailed,
    DatabaseAlreadyExists,
    DomainFailure,
    InvalidTimestamp,
    SelectionNotFound,
    SelectionPermissionDenied,
    SelectionUnsupported,
    SelectionMetadataUnavailable,
    SelectionInvalid,
    SelectionChanged,
    PickerBusy,
    DuplicatePath,
    StaleSession,
    UnsupportedPlatform,
    IdGenerationFailed,
    InternalFailure,
    TransportUnknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Attempted {
    pub canonical_id: Option<String>,
    pub alias_id: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Field {
    CanonicalId,
    AliasId,
    Name,
    ExpectedRevision,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ErrorReason {
    Empty,
    Whitespace,
    TraversalOrSeparator,
    ForbiddenCharacter,
    DeviceName,
    TrailingPeriod,
    TooLong,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandErrorDto {
    pub code: Code,
    pub field: Option<Field>,
    pub reason: Option<ErrorReason>,
    pub candidate_ids: Vec<String>,
    pub attempted: Option<Attempted>,
    pub storage_epoch: Option<String>,
}
impl CommandErrorDto {
    pub fn new(code: Code) -> Self {
        Self {
            code,
            field: None,
            reason: None,
            candidate_ids: vec![],
            attempted: None,
            storage_epoch: None,
        }
    }
}
impl From<PersistenceError> for CommandErrorDto {
    fn from(e: PersistenceError) -> Self {
        use PersistenceError as P;
        Self::new(match e {
            P::DuplicateCanonicalId => Code::DuplicateCanonicalId,
            P::DuplicateAliasId => Code::DuplicateAliasId,
            P::DuplicateAliasName => Code::DuplicateAliasName,
            P::NotFound => Code::CanonicalNotFound,
            P::Conflict => Code::Conflict,
            P::InvalidAggregateChange => Code::InvalidAggregateChange,
            P::InvalidStoredData => Code::InvalidStoredData,
            P::CorruptDatabase => Code::CorruptDatabase,
            P::IncompatibleDatabase => Code::IncompatibleDatabase,
            P::DatabaseAlreadyExists => Code::DatabaseAlreadyExists,
            P::UnsupportedPathEncoding => Code::UnsupportedPathEncoding,
            P::UnrepresentableValue => Code::UnrepresentableValue,
            P::Busy => Code::Busy,
            P::PermissionDenied => Code::PermissionDenied,
            P::Unavailable => Code::DatabaseUnavailable,
            P::MigrationFailed => Code::MigrationFailed,
            P::StorageFailure => Code::StorageFailure,
            P::UncertainOutcome => Code::UncertainOutcome,
        })
    }
}
impl From<ApplicationError> for CommandErrorDto {
    fn from(e: ApplicationError) -> Self {
        use ApplicationError as A;
        match e {
            A::Storage(e) => e.into(),
            A::Domain(DomainError::InvalidAliasName(reason)) => {
                let mut e = Self::new(Code::InvalidAliasName);
                e.field = Some(Field::Name);
                e.reason = Some(match reason {
                    AliasNameError::Empty => ErrorReason::Empty,
                    AliasNameError::Whitespace => ErrorReason::Whitespace,
                    AliasNameError::TraversalOrSeparator => ErrorReason::TraversalOrSeparator,
                    AliasNameError::ForbiddenCharacter => ErrorReason::ForbiddenCharacter,
                    AliasNameError::DeviceName => ErrorReason::DeviceName,
                    AliasNameError::TrailingPeriod => ErrorReason::TrailingPeriod,
                    AliasNameError::TooLong => ErrorReason::TooLong,
                });
                e
            }
            A::Domain(DomainError::InvalidTimestamp) => Self::new(Code::InvalidTimestamp),
            A::Domain(_) => Self::new(Code::DomainFailure),
            A::InvalidCanonicalId => Self::new(Code::InvalidCanonicalId),
            A::InvalidAliasId => Self::new(Code::InvalidAliasId),
            A::CanonicalNotFound => Self::new(Code::CanonicalNotFound),
            A::AliasNotFound => Self::new(Code::AliasNotFound),
            A::DuplicateCanonicalId => Self::new(Code::DuplicateCanonicalId),
            A::DuplicateAliasId => Self::new(Code::DuplicateAliasId),
            A::DuplicateAliasName => Self::new(Code::DuplicateAliasName),
            A::Conflict => Self::new(Code::Conflict),
        }
    }
}
impl From<filenym_infra::registration::RegistrationError> for CommandErrorDto {
    fn from(e: filenym_infra::registration::RegistrationError) -> Self {
        use filenym_infra::registration::RegistrationError as E;
        Self::new(match e {
            E::NotFound => Code::SelectionNotFound,
            E::PermissionDenied => Code::SelectionPermissionDenied,
            E::Unsupported => Code::SelectionUnsupported,
            E::MetadataUnavailable => Code::SelectionMetadataUnavailable,
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reply<T> {
    pub storage_epoch: Option<String>,
    pub value: T,
}
pub type CommandResult<T> = Result<Reply<T>, CommandErrorDto>;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionRequestDto {
    pub storage_epoch: String,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetFileRequestDto {
    pub storage_epoch: String,
    pub canonical_id: String,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddAliasRequestDto {
    pub storage_epoch: String,
    pub canonical_id: String,
    pub expected_revision: String,
    pub name: String,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameAliasRequestDto {
    pub storage_epoch: String,
    pub canonical_id: String,
    pub expected_revision: String,
    pub alias_id: String,
    pub name: String,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteAliasRequestDto {
    pub storage_epoch: String,
    pub canonical_id: String,
    pub expected_revision: String,
    pub alias_id: String,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegisterSelectionRequestDto {
    pub storage_epoch: String,
    pub selection_token: String,
    pub decision: Decision,
    pub candidate_id: Option<String>,
}
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Decision {
    Cancel,
    ViewExisting,
    AddSeparately,
}

pub fn revision(value: &str) -> Result<PersistenceRevision, CommandErrorDto> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(CommandErrorDto::new(Code::InvalidRequest));
    }
    value
        .parse::<i64>()
        .ok()
        .and_then(|v| PersistenceRevision::new(v).ok())
        .ok_or_else(|| CommandErrorDto::new(Code::InvalidRequest))
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimestampDto {
    pub seconds: String,
    pub nanoseconds: u32,
}
pub fn timestamp(value: SystemTime) -> Result<TimestampDto, CommandErrorDto> {
    let total = match value.duration_since(UNIX_EPOCH) {
        Ok(d) => i128::from(d.as_secs()) * 1_000_000_000 + i128::from(d.subsec_nanos()),
        Err(e) => {
            -(i128::from(e.duration().as_secs()) * 1_000_000_000
                + i128::from(e.duration().subsec_nanos()))
        }
    };
    let seconds = i64::try_from(total.div_euclid(1_000_000_000))
        .map_err(|_| CommandErrorDto::new(Code::UnrepresentableValue))?;
    Ok(TimestampDto {
        seconds: seconds.to_string(),
        nanoseconds: u32::try_from(total.rem_euclid(1_000_000_000))
            .map_err(|_| CommandErrorDto::new(Code::UnrepresentableValue))?,
    })
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NativeTextDto {
    pub encoding: String,
    pub units: Vec<u16>,
    pub display: String,
    pub escaped: bool,
}
impl NativeTextDto {
    pub fn new(value: &OsStr) -> Result<Self, CommandErrorDto> {
        #[cfg(not(windows))]
        {
            let _ = value;
            Err(CommandErrorDto::new(Code::UnsupportedPlatform))
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            let units = value.encode_wide().collect::<Vec<_>>();
            let mut escaped = false;
            let mut display = String::new();
            for item in char::decode_utf16(units.iter().copied()) {
                match item {
                    Ok(c) if !c.is_control() => display.push(c),
                    Ok(c) => {
                        escaped = true;
                        display.push_str(&format!("\\u{{{:04X}}}", u32::from(c)));
                    }
                    Err(e) => {
                        escaped = true;
                        display.push_str(&format!("\\u{{{:04X}}}", e.unpaired_surrogate()));
                    }
                }
            }
            Ok(Self {
                encoding: "windowsUtf16".into(),
                units,
                display,
                escaped,
            })
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StatusDto {
    Available,
    Modified,
    Unavailable { reason: ReasonDto },
    PossibleReplacement,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ReasonDto {
    NotFound,
    PermissionDenied,
    Unknown,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AliasDto {
    pub id: String,
    pub canonical_file_id: String,
    pub name: String,
    pub created_at: TimestampDto,
    pub updated_at: TimestampDto,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalFileDto {
    pub id: String,
    pub revision: String,
    pub filename: NativeTextDto,
    pub saved_path: NativeTextDto,
    pub size_bytes: String,
    pub modified_at: Option<TimestampDto>,
    pub registered_at: TimestampDto,
    pub last_observed_at: TimestampDto,
    pub status: StatusDto,
    pub aliases: Vec<AliasDto>,
}
impl CanonicalFileDto {
    /// A failed receipt conversion after commit must never imply a rejected write.
    pub fn from_committed(stored: StoredCanonicalFile) -> Result<Self, CommandErrorDto> {
        Self::from_stored(stored).map_err(|_| CommandErrorDto::new(Code::UncertainOutcome))
    }
    pub fn from_stored(stored: StoredCanonicalFile) -> Result<Self, CommandErrorDto> {
        let f = stored.file();
        Ok(Self {
            id: f.id().as_str().into(),
            revision: stored.revision().value().to_string(),
            filename: NativeTextDto::new(f.location().filename())?,
            saved_path: NativeTextDto::new(f.location().as_path().as_os_str())?,
            size_bytes: f.metadata().size().to_string(),
            modified_at: f.metadata().modified_at().map(timestamp).transpose()?,
            registered_at: timestamp(f.registered_at())?,
            last_observed_at: timestamp(f.last_observed_at())?,
            status: match f.status() {
                FileStatus::Available => StatusDto::Available,
                FileStatus::Modified => StatusDto::Modified,
                FileStatus::PossibleReplacement => StatusDto::PossibleReplacement,
                FileStatus::Unavailable(r) => StatusDto::Unavailable {
                    reason: match r {
                        UnavailableReason::NotFound => ReasonDto::NotFound,
                        UnavailableReason::PermissionDenied => ReasonDto::PermissionDenied,
                        UnavailableReason::Unknown => ReasonDto::Unknown,
                    },
                },
            },
            aliases: f
                .aliases()
                .iter()
                .map(|a| {
                    Ok(AliasDto {
                        id: a.id().as_str().into(),
                        canonical_file_id: a.canonical_file_id().as_str().into(),
                        name: a.name().as_str().into(),
                        created_at: timestamp(a.created_at())?,
                        updated_at: timestamp(a.updated_at())?,
                    })
                })
                .collect::<Result<_, CommandErrorDto>>()?,
        })
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RegistryEntryDto {
    File { file: Box<CanonicalFileDto> },
    Unreadable { id: String, code: Code },
    Missing { id: String },
}
#[derive(Clone, Debug, Serialize)]
pub struct RegistryDto {
    pub entries: Vec<RegistryEntryDto>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SelectionOutcomeDto {
    Cancelled,
    Added {
        file: Box<CanonicalFileDto>,
    },
    DuplicateDecision {
        selection_token: String,
        filename: NativeTextDto,
        duplicate_candidate_ids: Vec<String>,
    },
}
#[derive(Clone, Debug, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DuplicateDecisionOutcomeDto {
    Cancelled,
    ViewExisting { file: Box<CanonicalFileDto> },
    Added { file: Box<CanonicalFileDto> },
}
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Initializing,
    Ready,
    Blocked,
    NeedsReopen,
    Reopening,
    NeedsReload,
    Closing,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryStatusDto {
    pub phase: Phase,
    pub code: Option<Code>,
    pub affected_canonical_ids: Vec<String>,
}
