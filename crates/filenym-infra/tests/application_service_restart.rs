mod support;
use filenym_app::{AddAlias, DeleteAlias, FileRegistryService, RegisterCanonicalFile, RenameAlias};
use filenym_core::*;
use filenym_infra::SqliteRepository;
use std::{
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use support::*;

fn expected(after_mutation: bool) -> CanonicalFile {
    let mut file = CanonicalFile::new(
        id("process"),
        KnownFileLocation::new("nonexistent/process.pdf").unwrap(),
        None,
        FileMetadata::new(0, None),
        time(10),
    );
    file.create_alias(alias_id("0"), name("Report.PDF"), time(11))
        .unwrap();
    file.create_alias(alias_id("1"), name("Remove.pdf"), time(11))
        .unwrap();
    file.rename_alias(&alias_id("0"), name("Context.exe"), time(12))
        .unwrap();
    file.delete_alias(&alias_id("1")).unwrap();
    if after_mutation {
        file.rename_alias(&alias_id("0"), name("Next.txt"), time(13))
            .unwrap();
    }
    file
}

#[test]
fn application_service_restart_worker() {
    let Some(path) = std::env::var_os("FILENYM_SERVICE_TEST_DB") else {
        return;
    };
    let mode = std::env::var("FILENYM_SERVICE_TEST_MODE").unwrap();
    if mode == "missing" {
        assert!(matches!(
            SqliteRepository::open_existing(&path),
            Err(filenym_ports::PersistenceError::Unavailable)
        ));
        assert!(!Path::new(&path).exists());
        return;
    }
    let mut repo = SqliteRepository::open_existing(path).unwrap();
    let mut service = FileRegistryService::new(&mut repo);
    match mode.as_str() {
        "write" => {
            let current = service
                .register_canonical_file(RegisterCanonicalFile {
                    canonical_id: "process".into(),
                    location: KnownFileLocation::new("nonexistent/process.pdf").unwrap(),
                    identity: None,
                    metadata: FileMetadata::new(0, None),
                    registered_at: time(10),
                })
                .unwrap();
            let current = service
                .add_alias(AddAlias {
                    canonical_id: "process".into(),
                    expected_revision: current.revision(),
                    alias_id: "0".into(),
                    name: "Report.PDF".into(),
                    created_at: time(11),
                })
                .unwrap();
            let current = service
                .add_alias(AddAlias {
                    canonical_id: "process".into(),
                    expected_revision: current.revision(),
                    alias_id: "1".into(),
                    name: "Remove.pdf".into(),
                    created_at: time(11),
                })
                .unwrap();
            let current = service
                .rename_alias(RenameAlias {
                    canonical_id: "process".into(),
                    expected_revision: current.revision(),
                    alias_id: "0".into(),
                    name: "Context.exe".into(),
                    updated_at: time(12),
                })
                .unwrap();
            let current = service
                .delete_alias(DeleteAlias {
                    canonical_id: "process".into(),
                    expected_revision: current.revision(),
                    alias_id: "1".into(),
                })
                .unwrap();
            assert_eq!(current.file(), &expected(false));
            assert_eq!(current.revision().value(), 4);
        }
        "read" => {
            let current = service.get_canonical_file("process").unwrap();
            assert_eq!(current.file(), &expected(false));
            assert_eq!(current.revision().value(), 4);
            assert_eq!(
                service.list_registered_file_ids().unwrap(),
                vec![id("process")]
            );
        }
        "mutate" => {
            let current = service.get_canonical_file("process").unwrap();
            assert_eq!(current.file(), &expected(false));
            assert_eq!(current.revision().value(), 4);
            let next = service
                .rename_alias(RenameAlias {
                    canonical_id: "process".into(),
                    expected_revision: current.revision(),
                    alias_id: "0".into(),
                    name: "Next.txt".into(),
                    updated_at: time(13),
                })
                .unwrap();
            assert_eq!(next.file(), &expected(true));
            assert_eq!(next.revision().value(), 5);
        }
        _ => panic!("unknown service worker mode"),
    }
}

struct Worker(Child);
impl Drop for Worker {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}
fn run(mode: &str, path: &Path) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "application_service_restart_worker",
            "--nocapture",
        ])
        .env("FILENYM_SERVICE_TEST_DB", path)
        .env("FILENYM_SERVICE_TEST_MODE", mode)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut worker = Worker(command.spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = worker.0.try_wait().unwrap() {
            assert!(status.success(), "service worker {mode} failed");
            break;
        }
        assert!(Instant::now() < deadline, "service worker {mode} timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn fresh_process_service_journey_preserves_committed_state_and_missing_db_safety() {
    let dir = OwnedDirectory::new();
    run("missing", &dir.db());
    assert!(!dir.db().exists());
    drop(SqliteRepository::create_new(dir.db()).unwrap());
    for mode in ["write", "read", "mutate"] {
        run(mode, &dir.db());
    }
    let mut repo = SqliteRepository::open_existing(dir.db()).unwrap();
    let current = FileRegistryService::new(&mut repo)
        .get_canonical_file("process")
        .unwrap();
    assert_eq!(current.file(), &expected(true));
    assert_eq!(current.revision().value(), 5);
}
