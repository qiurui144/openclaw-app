#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod system_check;
mod session_state;
mod platform_config;
mod deploy;

#[tauri::command]
async fn run_system_check() -> Vec<system_check::CheckItem> {
    system_check::run_all_checks().await
}

#[tauri::command]
async fn start_deploy(
    config: deploy::DeployConfigDto,
    window: tauri::Window
) -> Result<(), String> {
    let cfg = deploy::DeployConfig::from(config);
    deploy::start_deploy(cfg, window).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn health_check(port: u16) -> Result<(), String> {
    deploy::health_check(port).await.map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            run_system_check,
            start_deploy,
            health_check
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
