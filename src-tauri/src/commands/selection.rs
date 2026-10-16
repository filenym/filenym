use crate::{
    selection,
    state::{blocking, DesktopState},
    transport::*,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::State;
use tauri_plugin_dialog::DialogExt;

struct PickerGuard(Arc<AtomicBool>);
impl Drop for PickerGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
#[tauri::command]
pub async fn select_existing_file(
    window: tauri::WebviewWindow,
    state: State<'_, DesktopState>,
    request: SessionRequestDto,
) -> CommandResult<SelectionOutcomeDto> {
    if window.label() != "main" {
        return Err(CommandErrorDto::new(Code::InvalidRequest));
    }
    if state.picker.swap(true, Ordering::SeqCst) {
        return Err(CommandErrorDto::new(Code::PickerBusy));
    }
    let guard = PickerGuard(state.picker.clone());
    let app = window.app_handle().clone();
    blocking(state.inner().clone(), move |s| {
        let _guard = guard;
        s.with(|session| {
            s.check(session, &request.storage_epoch, true)?;
            session.pending = None;
            Ok(())
        })?;
        let picked = app.dialog().file().set_parent(&window).blocking_pick_file();
        match picked {
            Some(path) => selection::selected(
                s,
                request.storage_epoch,
                path.into_path()
                    .map_err(|_| CommandErrorDto::new(Code::SelectionUnsupported))?,
            ),
            None => s.with(|session| {
                s.check(session, &request.storage_epoch, false)?;
                s.finish(session, Ok(SelectionOutcomeDto::Cancelled), None)
            }),
        }
    })
    .await
}
use tauri::Manager;
#[tauri::command]
pub async fn register_selected_file(
    state: State<'_, DesktopState>,
    request: RegisterSelectionRequestDto,
) -> CommandResult<DuplicateDecisionOutcomeDto> {
    blocking(state.inner().clone(), move |s| {
        selection::decide(s, request)
    })
    .await
}
