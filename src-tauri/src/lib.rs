//! Desktop composition root and Tauri transport wiring.

mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::version])
        .run(tauri::generate_context!())
        .expect("failed to run FileNym desktop application");
}
