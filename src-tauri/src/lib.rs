#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![ping])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Health-check command used to verify the Rust <-> JS bridge during Phase 0.
#[tauri::command]
fn ping() -> &'static str {
    "pong"
}
