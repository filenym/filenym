//! Desktop composition root and Tauri transport wiring.

mod commands;
mod ids;
mod registry;
mod selection;
mod state;
mod storage;
mod transport;
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let root = app
                .path()
                .app_local_data_dir()
                .ok()
                .map(|p| p.join("registry"));
            let state = state::DesktopState::new(root);
            app.manage(state.clone());
            tauri::async_runtime::spawn(async move {
                let _ = state::blocking(state, |s| s.initialize(false)).await;
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let state = window.state::<state::DesktopState>().inner().clone();
                if !state
                    .closing
                    .swap(true, std::sync::atomic::Ordering::SeqCst)
                {
                    let _ = window.emit_to("main", "registry-closing", ());
                    let app = window.app_handle().clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        let _ = state.with(|s| {
                            state.publish(s, transport::Phase::Closing, None);
                            s.repository = None;
                            s.pending = None;
                            Ok(())
                        });
                        app.exit(0);
                    });
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::version,
            commands::registry::registry_status,
            commands::registry::reopen_registry,
            commands::registry::get_canonical_file,
            commands::registry::list_registered_files,
            commands::registry::add_alias,
            commands::registry::rename_alias,
            commands::registry::delete_alias,
            commands::selection::select_existing_file,
            commands::selection::register_selected_file
        ])
        .run(tauri::generate_context!())
        .expect("failed to run FileNym desktop application");
}

#[cfg(test)]
mod tests;
