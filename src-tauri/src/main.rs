#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod system_check;

#[tauri::command]
async fn run_system_check() -> Vec<system_check::CheckItem> {
    system_check::run_all_checks().await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![run_system_check])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
