//! Thin commands exposed to the frontend.

#[tauri::command]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
