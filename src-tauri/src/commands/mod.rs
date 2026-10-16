//! Thin commands exposed to the frontend.
pub mod registry;
pub mod selection;

#[tauri::command]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
