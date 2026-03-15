#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod system_check;
mod session_state;
mod platform_config;
mod deploy;
mod clash_proxy;
mod skills_manager;
mod updater;
mod service_ctrl;
mod tray;
mod license;
mod skills_registry;
mod activation;

// ── 向导命令 ──────────────────────────────────────────────────────────────────

#[tauri::command]
async fn run_system_check() -> Vec<system_check::CheckItem> {
    system_check::run_all_checks().await
}

#[tauri::command]
async fn start_deploy(
    config: deploy::DeployConfigDto,
    window: tauri::Window,
) -> Result<(), String> {
    let cfg = deploy::DeployConfig::from(config);
    deploy::do_deploy_direct(cfg, &window).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn health_check(port: u16) -> Result<(), String> {
    deploy::health_check(port).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn clash_test(subscription_url: String) -> clash_proxy::ClashTestResult {
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
    wecom_config: Option<platform_config::WecomConfig>,
    dingtalk_config: Option<platform_config::DingtalkConfig>,
    feishu_config: Option<platform_config::FeishuConfig>,
    qq_config: Option<platform_config::QqConfig>,
) -> Result<(), String> {
    platform_config::write_platform_config(
        &install_path,
        platform_config::PlatformConfigs {
            wecom: wecom_config.as_ref(),
            dingtalk: dingtalk_config.as_ref(),
            feishu: feishu_config.as_ref(),
            qq: qq_config.as_ref(),
        },
    ).map_err(|e| e.to_string())
}

#[tauri::command]
async fn open_url(url: String) -> Result<(), String> {
    // 在独立线程执行，避免某些系统上 xdg-open 阻塞 IPC
    tokio::task::spawn_blocking(move || {
        open::that(&url).map_err(|e| e.to_string())
    }).await.unwrap_or_else(|e| Err(e.to_string()))
}

#[tauri::command]
fn run_uninstall(install_path: String) -> Result<(), String> {
    use std::path::PathBuf;
    use std::process::Command;

    #[cfg(unix)]
    let script = PathBuf::from(&install_path).join("uninstall.sh");
    #[cfg(windows)]
    let script = PathBuf::from(&install_path).join("uninstall.bat");
    #[cfg(not(any(unix, windows)))]
    return Err("不支持的操作系统".to_string());

    if !script.exists() {
        return Err(format!(
            "卸载脚本不存在：{}，请手动删除安装目录",
            script.display()
        ));
    }

    #[cfg(unix)]
    let status = Command::new("bash").arg(&script).status();
    #[cfg(windows)]
    let status = Command::new("cmd").args(["/C", &script.to_string_lossy()]).status();

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("卸载脚本退出码：{}", s.code().unwrap_or(-1))),
        Err(e) => Err(format!("执行卸载脚本失败：{e}")),
    }
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

// ── 服务控制命令（供托盘和 FinishPage 使用）──────────────────────────────────

/// 查询 OpenClaw 服务当前状态（"running" / "stopped" / "unknown"）
#[tauri::command]
async fn service_status() -> String {
    match service_ctrl::read_meta() {
        Some(m) => match service_ctrl::check_status(m.service_port).await {
            service_ctrl::ServiceStatus::Running => "running".to_string(),
            service_ctrl::ServiceStatus::Stopped => "stopped".to_string(),
        },
        None => "unknown".to_string(),
    }
}

/// 启动 OpenClaw 服务
#[tauri::command]
fn service_start() -> Result<(), String> {
    let meta = service_ctrl::read_meta().ok_or("未找到安装记录，请先安装 OpenClaw")?;
    service_ctrl::start(&meta).map_err(|e| e.to_string())
}

/// 停止 OpenClaw 服务
#[tauri::command]
fn service_stop() -> Result<(), String> {
    service_ctrl::stop().map_err(|e| e.to_string())
}

/// 部署完成后通知托盘刷新（加载新 meta、更新状态图标）
#[tauri::command]
async fn notify_deploy_done(app: tauri::AppHandle) {
    use tauri::Manager;
    if let Some(meta) = service_ctrl::read_meta() {
        if let Some(state) = app.try_state::<tray::TrayMeta>() {
            if let Ok(mut guard) = state.0.lock() {
                *guard = Some(meta);
            }
        }
    }
    tray::refresh_tray_icon(&app).await;
}

// ── 许可证命令 ──────────────────────────────────────────────────────────────

/// 获取当前许可证状态
#[tauri::command]
fn get_license_status() -> license::LicenseStatus {
    license::load_license()
}

/// 获取机器指纹
#[tauri::command]
fn get_machine_id() -> String {
    license::machine_id()
}

/// 发送登录验证码
#[tauri::command]
async fn send_login_code(phone: String) -> Result<(), String> {
    license::send_sms_code(&phone).await.map_err(|e| e.to_string())
}

/// 手机号 + 验证码登录
#[tauri::command]
async fn license_login(phone: String, code: String) -> Result<license::LicenseStatus, String> {
    license::login(&phone, &code).await.map_err(|e| e.to_string())
}

/// 刷新许可证令牌
#[tauri::command]
async fn refresh_license() -> Result<license::LicenseStatus, String> {
    license::refresh_token().await.map_err(|e| e.to_string())
}

/// 使用授权码激活
#[tauri::command]
async fn redeem_activation_code(code: String) -> Result<license::LicenseStatus, String> {
    license::redeem_code(&code).await.map_err(|e| e.to_string())
}

/// 登出
#[tauri::command]
fn license_logout() {
    license::logout();
}

// ── Skills 商店命令 ──────────────────────────────────────────────────────────

/// 获取 Skill 索引（免费 + 付费混合列表）
#[tauri::command]
async fn fetch_skill_index() -> Result<Vec<skills_registry::SkillEntry>, String> {
    skills_registry::fetch_skill_index().await.map_err(|e| e.to_string())
}

/// 安装付费 Skill
#[tauri::command]
async fn install_paid_skill(
    install_path: String,
    slug: String,
) -> Result<(), String> {
    let jwt = license::current_jwt().ok_or("未登录，请先登录".to_string())?;
    if !license::can_access_skill(&slug) {
        return Err("无权访问此 Skill，请升级订阅".to_string());
    }
    skills_registry::install_paid_skill(&install_path, &slug, &jwt)
        .await
        .map_err(|e| e.to_string())
}

/// 卸载付费 Skill
#[tauri::command]
fn uninstall_paid_skill(slug: String) -> Result<(), String> {
    skills_registry::uninstall_paid_skill(&slug).map_err(|e| e.to_string())
}

/// 列出已安装的付费 Skills
#[tauri::command]
fn list_paid_skills() -> Vec<String> {
    skills_registry::list_installed_paid_skills()
}

/// 刷新过期的付费 Skill 缓存（重新从 API 拉取内容）
#[tauri::command]
async fn refresh_expired_skills(install_path: String) -> Result<Vec<String>, String> {
    let jwt = match license::current_jwt() {
        Some(j) => j,
        None => return Ok(vec![]),
    };
    let slugs = skills_registry::list_installed_paid_skills();
    let mut refreshed = vec![];
    for slug in &slugs {
        if skills_registry::is_skill_cache_expired(slug) {
            if license::can_access_skill(slug) {
                skills_registry::install_paid_skill(&install_path, slug, &jwt)
                    .await
                    .map_err(|e| e.to_string())?;
                refreshed.push(slug.clone());
            }
        }
    }
    Ok(refreshed)
}

/// 创建支付订单
#[tauri::command]
async fn create_payment(
    plan: String,
    skill_slug: Option<String>,
) -> Result<skills_registry::PaymentOrder, String> {
    let jwt = license::current_jwt().ok_or("未登录，请先登录".to_string())?;
    skills_registry::create_payment_order(&jwt, &plan, skill_slug.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// 查询支付状态
#[tauri::command]
async fn check_payment(order_id: String) -> Result<String, String> {
    let jwt = license::current_jwt().ok_or("未登录，请先登录".to_string())?;
    skills_registry::check_payment_status(&jwt, &order_id)
        .await
        .map_err(|e| e.to_string())
}

// ── 公众号激活命令 ────────────────────────────────────────────────────────────

/// 检查是否已通过公众号验证
#[tauri::command]
fn check_activation_status() -> bool {
    activation::is_activated()
}

/// 请求公众号带参二维码
#[tauri::command]
async fn request_activation_qr() -> Result<activation::ActivationQr, String> {
    activation::request_qrcode().await.map_err(|e| e.to_string())
}

/// 轮询公众号关注状态
#[tauri::command]
async fn poll_activation(ticket: String) -> Result<activation::ActivationResult, String> {
    activation::check_activation(&ticket).await.map_err(|e| e.to_string())
}

/// 获取客户端 ID（品牌标识）
#[tauri::command]
fn get_client_id() -> String {
    activation::get_client_id().to_string()
}

// ── 应用入口 ──────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            tray::setup_tray(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // 有已安装的 OpenClaw 时：隐藏到托盘而非退出
                // 无安装记录时：正常关闭，避免用户无法退出向导
                use tauri::Manager;
                let has_meta = window
                    .try_state::<tray::TrayMeta>()
                    .map(|s| s.0.lock().ok().map(|g| g.is_some()).unwrap_or(false))
                    .unwrap_or(false);
                if has_meta {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
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
            open_url,
            run_uninstall,
            read_deploy_meta,
            get_default_install_path,
            service_status,
            service_start,
            service_stop,
            notify_deploy_done,
            // 许可证
            get_license_status,
            get_machine_id,
            send_login_code,
            license_login,
            refresh_license,
            redeem_activation_code,
            license_logout,
            // Skills 商店
            fetch_skill_index,
            install_paid_skill,
            uninstall_paid_skill,
            list_paid_skills,
            create_payment,
            check_payment,
            refresh_expired_skills,
            // 公众号激活
            check_activation_status,
            request_activation_qr,
            poll_activation,
            get_client_id,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
