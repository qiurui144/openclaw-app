#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod system_check;
mod session_state;
mod platform_config;
mod deploy;
mod clash_proxy;

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

#[tauri::command]
async fn clash_test(subscription_url: String) -> clash_proxy::ClashTestResult {
    // G2 fix: 先启动 Mihomo 获取代理地址，再测试代理连通性
    match clash_proxy::start(&subscription_url).await {
        Ok(proxy_addr) => clash_proxy::test_proxy(&proxy_addr).await,
        Err(e) => clash_proxy::ClashTestResult {
            success: false,
            latency_ms: None,
            error: Some(e.to_string()),
        },
    }
}

#[tauri::command]
async fn clash_start(subscription_url: String) -> Result<String, String> {
    clash_proxy::start(&subscription_url).await.map_err(|e| e.to_string())
}

#[tauri::command]
fn clash_stop() -> Result<(), String> {
    clash_proxy::stop().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            run_system_check,
            start_deploy,
            health_check,
            clash_test,
            clash_start,
            clash_stop
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
