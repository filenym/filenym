use crate::{
    registry,
    state::{blocking, DesktopState},
    transport::*,
};
use tauri::State;

#[tauri::command]
pub fn registry_status(state: State<'_, DesktopState>) -> CommandResult<RegistryStatusDto> {
    state
        .status
        .lock()
        .map(|s| s.clone())
        .map_err(|_| CommandErrorDto::new(Code::InternalFailure))
}
#[tauri::command]
pub async fn reopen_registry(state: State<'_, DesktopState>) -> CommandResult<RegistryStatusDto> {
    blocking(state.inner().clone(), |s| s.initialize(true)).await
}
#[tauri::command]
pub async fn get_canonical_file(
    state: State<'_, DesktopState>,
    request: GetFileRequestDto,
) -> CommandResult<CanonicalFileDto> {
    blocking(state.inner().clone(), move |s| registry::get(s, request)).await
}
#[tauri::command]
pub async fn list_registered_files(
    state: State<'_, DesktopState>,
    request: SessionRequestDto,
) -> CommandResult<RegistryDto> {
    blocking(state.inner().clone(), move |s| registry::list(s, request)).await
}
#[tauri::command]
pub async fn add_alias(
    state: State<'_, DesktopState>,
    request: AddAliasRequestDto,
) -> CommandResult<CanonicalFileDto> {
    blocking(state.inner().clone(), move |s| registry::add(s, request)).await
}
#[tauri::command]
pub async fn rename_alias(
    state: State<'_, DesktopState>,
    request: RenameAliasRequestDto,
) -> CommandResult<CanonicalFileDto> {
    blocking(state.inner().clone(), move |s| registry::rename(s, request)).await
}
#[tauri::command]
pub async fn delete_alias(
    state: State<'_, DesktopState>,
    request: DeleteAliasRequestDto,
) -> CommandResult<CanonicalFileDto> {
    blocking(state.inner().clone(), move |s| registry::delete(s, request)).await
}
