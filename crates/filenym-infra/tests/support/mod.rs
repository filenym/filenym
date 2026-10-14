#![allow(dead_code)]
use filenym_core::*;
use rusqlite::Connection;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
pub struct OwnedDirectory {
    pub path: PathBuf,
    root: PathBuf,
}
impl OwnedDirectory {
    pub fn new() -> Self {
        let root = std::env::temp_dir()
            .canonicalize()
            .expect("resolve temporary root");
        loop {
            let path = root.join(format!(
                "filenym-tests-{}-{}",
                std::process::id(),
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self { path, root },
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("cannot create isolated test directory: {e}"),
            }
        }
    }
    pub fn db(&self) -> PathBuf {
        self.path.join("registry.sqlite3")
    }
    pub fn raw(&self) -> Connection {
        let conn = Connection::open(self.db()).unwrap();
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        conn
    }
}
impl Drop for OwnedDirectory {
    fn drop(&mut self) {
        let resolved = self
            .path
            .canonicalize()
            .expect("resolve owned test directory");
        assert_eq!(
            resolved, self.path,
            "test directory must not redirect cleanup"
        );
        assert_eq!(
            resolved.parent(),
            Some(self.root.as_path()),
            "cleanup must stay in the owned temporary namespace"
        );
        fs::remove_dir_all(resolved)
            .expect("owned test directory cleanup failed; close handles first");
    }
}
pub fn time(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::new(seconds, 123)
}
pub fn id(value: &str) -> CanonicalFileId {
    CanonicalFileId::new(value).unwrap()
}
pub fn alias_id(value: &str) -> AliasId {
    AliasId::new(value).unwrap()
}
pub fn name(value: &str) -> AliasName {
    AliasName::new(value).unwrap()
}
pub fn fixture(value: &str) -> CanonicalFile {
    let mut file = CanonicalFile::new(
        id(value),
        KnownFileLocation::new("nonexistent/é.pdf").unwrap(),
        Some(FileIdentity::new("opaque' scheme", vec![0, 255, 1]).unwrap()),
        FileMetadata::new(u64::MAX, Some(UNIX_EPOCH - Duration::from_nanos(123))),
        time(10),
    );
    for (index, value) in [
        "Report.PDF",
        "É.pdf",
        "é.pdf",
        "e\u{301}.pdf",
        "context.exe",
        ".dotfile",
        "noextension",
        "internal space.tar.gz",
    ]
    .into_iter()
    .enumerate()
    {
        file.create_alias(alias_id(&index.to_string()), name(value), time(11))
            .unwrap();
    }
    file.rename_alias(&alias_id("0"), name("report.PDF"), time(12))
        .unwrap();
    file
}
