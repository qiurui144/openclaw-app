#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod system_check;
mod session_state;
mod platform_config;
mod deploy;
mod clash_proxy;
mod skills_manager;
mod updater;

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

#[tauri::command]
fn list_skills(install_path: String) -> Vec<skills_manager::SkillInfo> {
    skills_manager::list_installed(&install_path)
}

#[tauri::command]
async fn update_skills(
    install_path: String,
    skill_names: Vec<String>,
    proxy_url: Option<String>,
) -> Result<(), String> {
    for skill in &skill_names {
        let version = skills_manager::fetch_latest_version(
            skill, proxy_url.as_deref()
        ).await.map_err(|e| e.to_string())?;
        skills_manager::update_skill(
            &install_path, skill, &version, proxy_url.as_deref()
        ).await.map_err(|e| e.to_string())?;
    }
    #[cfg(unix)]
    skills_manager::send_reload_signal(&install_path)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn check_openclaw_update(proxy_url: Option<String>)
    -> Result<Option<updater::UpdateInfo>, String>
{
    updater::check_update(proxy_url.as_deref()).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn apply_openclaw_update(
    install_path: String,
    download_url: String,
    sha256: Option<String>,
    proxy_url: Option<String>,
    window: tauri::Window,
) -> Result<(), String> {
    updater::apply_update(
        &install_path, &download_url,
        sha256.as_deref(), proxy_url.as_deref(), &window
    ).await.map_err(|e| e.to_string())
}

#[tauri::command]
fn load_session() -> Option<session_state::SessionState> {
    session_state::load()
}

#[tauri::command]
fn clear_session(install_path: Option<String>) -> Result<(), String> {
    session_state::clear(install_path.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn write_platform_config(
    install_path: String,
    platforms: Vec<platform_config::PlatformEntry>,
) -> Result<(), String> {
    platform_config::write_platform_config(&install_path, &platforms)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_platform_doc_url(platform: platform_config::Platform) -> String {
    platform.doc_url().to_string()
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_deploy_meta() -> Option<serde_json::Value> {
    let path = dirs::home_dir()?.join(".openclaw/deploy_meta.json");
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

#[tauri::command]
fn get_default_install_path() -> String {
    let elevated = system_check::is_elevated();
    #[cfg(target_os = "linux")]
    return if elevated {
        "/opt/openclaw".to_string()
    } else {
        format!("{}/openclaw",
            dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "~".to_string()))
    };
    #[cfg(target_os = "windows")]
    return if elevated {
        r"C:\Program Files\openclaw".to_string()
    } else {
        format!("{}\\openclaw",
            std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\Users\\Public".to_string()))
    };
    #[cfg(target_os = "macos")]
    return format!("{}/openclaw",
        dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "~".to_string()));
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    return "/opt/openclaw".to_string();
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
            clash_stop,
            list_skills,
            update_skills,
            check_openclaw_update,
            apply_openclaw_update,
            load_session,
            clear_session,
            write_platform_config,
            get_platform_doc_url,
            open_url,
            read_deploy_meta,
            get_default_install_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
