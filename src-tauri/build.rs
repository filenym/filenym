fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "version",
            "registry_status",
            "reopen_registry",
            "get_canonical_file",
            "list_registered_files",
            "add_alias",
            "rename_alias",
            "delete_alias",
            "select_existing_file",
            "register_selected_file",
        ]),
    ))
    .expect("build FileNym command permissions")
}
