use filenym_app::{AddAlias, DeleteAlias, RegisterCanonicalFile, RenameAlias};
use filenym_core::*;
use filenym_ports::*;
use std::{
    collections::VecDeque,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}
pub fn rev(value: i64) -> PersistenceRevision {
    PersistenceRevision::new(value).unwrap()
}
pub fn registration() -> RegisterCanonicalFile {
    RegisterCanonicalFile {
        canonical_id: "canonical".into(),
        location: KnownFileLocation::new("nonexistent/private.pdf").unwrap(),
        identity: None,
        metadata: FileMetadata::new(0, None),
        registered_at: at(10),
    }
}
pub fn file() -> CanonicalFile {
    let request = registration();
    CanonicalFile::new(
        CanonicalFileId::new(request.canonical_id).unwrap(),
        request.location,
        request.identity,
        request.metadata,
        request.registered_at,
    )
}
pub fn populated() -> CanonicalFile {
    let mut file = file();
    file.create_alias(
        AliasId::new("alias").unwrap(),
        AliasName::new("Report.PDF").unwrap(),
        at(11),
    )
    .unwrap();
    file
}
pub fn add() -> AddAlias {
    AddAlias {
        canonical_id: "canonical".into(),
        expected_revision: rev(3),
        alias_id: "other".into(),
        name: "Other.txt".into(),
        created_at: at(12),
    }
}
pub fn rename() -> RenameAlias {
    RenameAlias {
        canonical_id: "canonical".into(),
        expected_revision: rev(3),
        alias_id: "alias".into(),
        name: "Changed.exe".into(),
        updated_at: at(13),
    }
}
pub fn delete() -> DeleteAlias {
    DeleteAlias {
        canonical_id: "canonical".into(),
        expected_revision: rev(3),
        alias_id: "alias".into(),
    }
}

pub enum Step {
    Insert(CanonicalFile, Result<PersistenceRevision, PersistenceError>),
    Load(
        String,
        Result<Option<StoredCanonicalFile>, PersistenceError>,
    ),
    Update(
        CanonicalFile,
        PersistenceRevision,
        Result<PersistenceRevision, PersistenceError>,
    ),
    List(Result<Vec<CanonicalFileId>, PersistenceError>),
}
pub struct Spy {
    steps: VecDeque<Step>,
}
impl Spy {
    pub fn new(steps: impl IntoIterator<Item = Step>) -> Self {
        Self {
            steps: steps.into_iter().collect(),
        }
    }
    pub fn done(&self) {
        assert!(
            self.steps.is_empty(),
            "expected repository calls were not consumed"
        );
    }
}
impl CanonicalFileRepository for Spy {
    fn insert(&mut self, file: &CanonicalFile) -> Result<PersistenceRevision, PersistenceError> {
        match self.steps.pop_front().expect("unexpected insert") {
            Step::Insert(expected, result) => {
                assert_eq!(file, &expected);
                result
            }
            _ => panic!("incorrect call order: insert"),
        }
    }
    fn load(
        &mut self,
        id: &CanonicalFileId,
    ) -> Result<Option<StoredCanonicalFile>, PersistenceError> {
        match self.steps.pop_front().expect("unexpected load") {
            Step::Load(expected, result) => {
                assert_eq!(id.as_str(), expected);
                result
            }
            _ => panic!("incorrect call order: load"),
        }
    }
    fn update(
        &mut self,
        file: &CanonicalFile,
        revision: PersistenceRevision,
    ) -> Result<PersistenceRevision, PersistenceError> {
        match self.steps.pop_front().expect("unexpected update") {
            Step::Update(expected, expected_revision, result) => {
                assert_eq!(file, &expected);
                assert_eq!(revision, expected_revision);
                result
            }
            _ => panic!("incorrect call order: update"),
        }
    }
    fn list_ids(&mut self) -> Result<Vec<CanonicalFileId>, PersistenceError> {
        match self.steps.pop_front().expect("unexpected list") {
            Step::List(result) => result,
            _ => panic!("incorrect call order: list"),
        }
    }
}
pub fn loaded(file: CanonicalFile) -> Step {
    Step::Load(
        "canonical".into(),
        Ok(Some(StoredCanonicalFile::new(file, rev(3)))),
    )
}
