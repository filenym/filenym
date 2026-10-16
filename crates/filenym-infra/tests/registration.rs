#![cfg(windows)]
mod support;
use filenym_infra::registration::{inspect_registration_candidate, RegistrationError};
use std::{fs, os::windows::fs::OpenOptionsExt, process::Command};
#[test]
fn regular_zero_unicode_readonly_and_extended_paths_are_metadata_only() {
    let directory = support::OwnedDirectory::new();
    for (name, bytes) in [("empty.txt", b"".as_slice()), ("résumé.txt", b"sentinel")] {
        let path = directory.path.join(name);
        fs::write(&path, bytes).unwrap();
        let before = fs::metadata(&path).unwrap();
        let facts = inspect_registration_candidate(&path).unwrap();
        assert_eq!(facts.location.as_path(), path);
        assert_eq!(facts.metadata.size(), bytes.len() as u64);
        drop(facts);
        let extended = path.canonicalize().unwrap();
        assert!(inspect_registration_candidate(&extended).is_ok());
        let mut permissions = before.permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&path, permissions).unwrap();
        assert!(inspect_registration_candidate(&path).is_ok());
        fs::set_permissions(&path, before.permissions()).unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(
            fs::metadata(&path).unwrap().modified().unwrap(),
            before.modified().unwrap()
        );
    }
}
#[test]
fn unsupported_inputs_fail_and_data_sharing_requires_no_data_access() {
    let directory = support::OwnedDirectory::new();
    assert!(matches!(
        inspect_registration_candidate(&directory.path),
        Err(RegistrationError::Unsupported)
    ));
    for path in [
        directory.path.join("absent"),
        r"\\server\share\file".into(),
        r"\\.\NUL".into(),
        r"C:relative.txt".into(),
        directory.path.join("file:stream"),
    ] {
        assert!(inspect_registration_candidate(&path).is_err());
    }
    let file = directory.path.join("locked.txt");
    fs::write(&file, b"safe").unwrap();
    let handle = fs::OpenOptions::new()
        .write(true)
        .share_mode(0)
        .open(&file)
        .unwrap();
    // FILE_READ_ATTRIBUTES is independent of data-sharing permissions.
    assert!(inspect_registration_candidate(&file).is_ok());
    drop(handle);
}
#[test]
fn ordinary_local_junctions_work_broken_and_final_redirects_fail() {
    let directory = support::OwnedDirectory::new();
    let target = directory.path.join("target");
    fs::create_dir(&target).unwrap();
    let file = target.join("file.txt");
    fs::write(&file, b"safe").unwrap();
    let redirect = directory.path.join("junction");
    let output = Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&redirect)
        .arg(&target)
        .output()
        .unwrap();
    assert!(output.status.success(), "junction fixture creation failed");
    assert!(inspect_registration_candidate(&redirect.join("file.txt")).is_ok());
    assert!(inspect_registration_candidate(&redirect).is_err());
    fs::remove_file(&file).unwrap();
    assert!(inspect_registration_candidate(&redirect.join("file.txt")).is_err());
    fs::remove_dir(&target).unwrap();
    assert!(inspect_registration_candidate(&redirect.join("file.txt")).is_err());
    fs::remove_dir(&redirect).unwrap();
}
#[test]
fn native_unpaired_filename_is_inspected_without_lossy_conversion() {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt};
    let directory = support::OwnedDirectory::new();
    let name = OsString::from_wide(&[65, 0xd800, 46, 116, 120, 116]);
    let path = directory.path.join(&name);
    fs::write(&path, b"native sentinel").unwrap();
    let facts = inspect_registration_candidate(&path).unwrap();
    assert_eq!(facts.location.filename(), name);
    assert_eq!(facts.metadata.size(), 15);
}
#[test]
fn redirect_loop_is_rejected_without_opening_file_data() {
    let directory = support::OwnedDirectory::new();
    let loop_path = directory.path.join("loop");
    let output = Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&loop_path)
        .arg(&loop_path)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(inspect_registration_candidate(&loop_path.join("file.txt")).is_err());
    fs::remove_dir(loop_path).unwrap();
}
#[test]
fn denied_attribute_permission_returns_typed_error_and_never_reads_contents() {
    let directory = support::OwnedDirectory::new();
    let private = directory.path.join("private");
    fs::create_dir(&private).unwrap();
    let path = private.join("private.txt");
    fs::write(&path, b"private sentinel").unwrap();
    let who = Command::new("whoami").output().unwrap();
    assert!(who.status.success());
    let account = String::from_utf8(who.stdout).unwrap().trim().to_owned();
    let deny = Command::new("icacls")
        .arg(&path)
        .args(["/deny", &format!("{account}:(RA)")])
        .output()
        .unwrap();
    assert!(
        deny.status.success(),
        "cannot establish owned deny-attributes fixture"
    );
    let deny_directory = Command::new("icacls")
        .arg(&private)
        .args(["/deny", &format!("{account}:(RD)")])
        .output()
        .unwrap();
    assert!(deny_directory.status.success());
    let result = inspect_registration_candidate(&path);
    let restore_directory = Command::new("icacls")
        .arg(&private)
        .args(["/remove:d", &account])
        .output()
        .unwrap();
    let restore = Command::new("icacls")
        .arg(&path)
        .args(["/remove:d", &account])
        .output()
        .unwrap();
    assert!(restore_directory.status.success());
    assert!(restore.status.success(), "cannot restore owned fixture ACL");
    assert!(
        matches!(result, Err(RegistrationError::PermissionDenied)),
        "actual result: {:?}",
        result.as_ref().map(|_| ())
    );
    assert_eq!(fs::read(path).unwrap(), b"private sentinel");
}
