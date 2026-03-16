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

/// 完整卸载（逐步执行 + 验证），返回每步结果
#[tauri::command]
async fn run_uninstall(install_path: String, service_port: Option<u16>) -> Result<Vec<UninstallStepResult>, String> {
    use std::path::PathBuf;

    let mut results = Vec::new();
    let port = service_port.unwrap_or(18789);

    // 1. 停止服务
    match service_ctrl::stop() {
        Ok(()) => results.push(UninstallStepResult { step: "停止服务".into(), success: true, detail: "已停止".into() }),
        Err(e) => results.push(UninstallStepResult { step: "停止服务".into(), success: false, detail: e.to_string() }),
    }
    // 等待端口释放（轮询，最多 10 秒）
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let still_open = tokio::time::timeout(
            std::time::Duration::from_millis(300),
            tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port)),
        ).await.map(|r| r.is_ok()).unwrap_or(false);
        if !still_open { break; }
    }

    // 2. 移除服务注册
    let svc_result = remove_service_registration();
    results.push(UninstallStepResult {
        step: "移除服务注册".into(),
        success: svc_result.is_ok(),
        detail: svc_result.unwrap_or_else(|e| e.to_string()),
    });

    // 3. 删除安装目录（安全校验：确认是 OpenClaw 安装目录）
    let install = PathBuf::from(&install_path);
    if install.exists() {
        let is_openclaw_dir = install.join("openclaw_pkg").join("package").join("openclaw.mjs").exists()
            || install.join("openclaw_pkg").exists()
            || install.join("uninstall.sh").exists()
            || install.join("uninstall.bat").exists();
        if !is_openclaw_dir {
            results.push(UninstallStepResult {
                step: "删除安装目录".into(),
                success: false,
                detail: format!("安全校验失败：{} 不是有效的 OpenClaw 安装目录（缺少特征文件）", install_path),
            });
        } else {
            match std::fs::remove_dir_all(&install) {
                Ok(()) => results.push(UninstallStepResult { step: "删除安装目录".into(), success: true, detail: format!("{} 已删除", install_path) }),
                Err(e) => results.push(UninstallStepResult { step: "删除安装目录".into(), success: false, detail: e.to_string() }),
            }
        }
    } else {
        results.push(UninstallStepResult { step: "删除安装目录".into(), success: true, detail: "目录不存在，跳过".into() });
    }

    // 4. 删除用户配置目录 ~/.openclaw
    if let Some(home) = dirs::home_dir() {
        let config_dir = home.join(".openclaw");
        if config_dir.exists() {
            match std::fs::remove_dir_all(&config_dir) {
                Ok(()) => results.push(UninstallStepResult { step: "删除配置目录".into(), success: true, detail: "~/.openclaw 已删除".into() }),
                Err(e) => results.push(UninstallStepResult { step: "删除配置目录".into(), success: false, detail: e.to_string() }),
            }
        } else {
            results.push(UninstallStepResult { step: "删除配置目录".into(), success: true, detail: "不存在，跳过".into() });
        }
    }

    // 5. 验证：目录不存在 + 端口已释放
    let dir_gone = !PathBuf::from(&install_path).exists();
    let port_free = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port)),
    ).await.map(|r| r.is_err()).unwrap_or(true);

    results.push(UninstallStepResult {
        step: "验证卸载".into(),
        success: dir_gone && port_free,
        detail: format!(
            "安装目录{}，端口 {} {}",
            if dir_gone { "已清除" } else { "仍存在" },
            port,
            if port_free { "已释放" } else { "仍被占用" }
        ),
    });

    Ok(results)
}

#[derive(Debug, Clone, serde::Serialize)]
struct UninstallStepResult {
    step: String,
    success: bool,
    detail: String,
}

/// 移除系统服务注册（systemd / launchd / schtasks），返回实际操作结果
fn remove_service_registration() -> Result<String, String> {
    let mut issues: Vec<String> = Vec::new();

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        let elevated = system_check::is_elevated();
        if elevated {
            if let Err(e) = Command::new("systemctl").args(["disable", "openclaw.service"]).status() {
                issues.push(format!("systemctl disable 失败: {e}"));
            }
            let unit_path = std::path::PathBuf::from("/etc/systemd/system/openclaw.service");
            if unit_path.exists() {
                if let Err(e) = std::fs::remove_file(&unit_path) {
                    issues.push(format!("删除 unit 文件失败: {e}"));
                }
            }
            let _ = Command::new("systemctl").args(["daemon-reload"]).status();
        } else {
            let _ = Command::new("timeout").args(["10", "systemctl", "--user", "disable", "openclaw.service"]).status();
            if let Some(home) = dirs::home_dir() {
                let unit_path = home.join(".config/systemd/user/openclaw.service");
                if unit_path.exists() {
                    if let Err(e) = std::fs::remove_file(&unit_path) {
                        issues.push(format!("删除 unit 文件失败: {e}"));
                    }
                }
            }
            let _ = Command::new("timeout").args(["10", "systemctl", "--user", "daemon-reload"]).status();
        }
        return if issues.is_empty() {
            Ok("systemd 服务已移除".into())
        } else {
            Err(format!("systemd 服务移除部分失败: {}", issues.join("; ")))
        };
    }
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let elevated = system_check::is_elevated();
        let plist = if elevated {
            std::path::PathBuf::from("/Library/LaunchDaemons/com.openclaw.gateway.plist")
        } else {
            dirs::home_dir().unwrap_or_default().join("Library/LaunchAgents/com.openclaw.gateway.plist")
        };
        if plist.exists() {
            let unload_ok = Command::new("launchctl").args(["unload", "-w", plist.to_str().unwrap_or("")])
                .status().map(|s| s.success()).unwrap_or(false);
            if !unload_ok { issues.push("launchctl unload 失败".into()); }
            if let Err(e) = std::fs::remove_file(&plist) {
                issues.push(format!("删除 plist 失败: {e}"));
            }
        }
        return if issues.is_empty() {
            Ok("launchd 服务已移除".into())
        } else {
            Err(format!("launchd 服务移除部分失败: {}", issues.join("; ")))
        };
    }
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let ok = Command::new("schtasks").args(["/Delete", "/TN", "OpenClaw Gateway", "/F"])
            .status().map(|s| s.success()).unwrap_or(false);
        return if ok {
            Ok("计划任务已移除".into())
        } else {
            Err("计划任务移除失败（可能不存在或权限不足）".into())
        };
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    Err("不支持的平台".into())
}

#[tauri::command]
fn read_deploy_meta() -> Option<serde_json::Value> {
    let path = dirs::home_dir()?.join(".openclaw/deploy_meta.json");
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

/// 校验安装路径（黑名单 + 可写 + 磁盘空间）
#[tauri::command]
fn validate_install_path(path: String) -> Result<(), String> {
    deploy::validate_install_path(&path).map_err(|e| e.to_string())
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

// ── 简化配置页面命令（读写 openclaw.json）──────────────────────────────────

/// 读取 ~/.openclaw/openclaw.json 完整内容
#[tauri::command]
fn read_openclaw_config() -> Result<serde_json::Value, String> {
    let path = dirs::home_dir()
        .ok_or("无法获取 home 目录")?
        .join(".openclaw/openclaw.json");
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| format!("配置文件格式错误: {e}"))
}

/// 写入 ~/.openclaw/openclaw.json（深度合并，不覆盖未传字段）
#[tauri::command]
fn write_openclaw_config(patch: serde_json::Value) -> Result<(), String> {
    let dir = dirs::home_dir()
        .ok_or("无法获取 home 目录")?
        .join(".openclaw");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("openclaw.json");

    let mut current: serde_json::Value = if path.exists() {
        let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    json_deep_merge(&mut current, &patch);

    std::fs::write(&path, serde_json::to_string_pretty(&current).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

/// 获取 Gateway 综合状态（健康、已配置的平台/AI 信息）
#[tauri::command]
async fn get_gateway_status() -> serde_json::Value {
    let meta = service_ctrl::read_meta();
    let port = meta.as_ref().map(|m| m.service_port).unwrap_or(18789);

    // 检测服务是否运行
    let status = if let Some(ref m) = meta {
        service_ctrl::check_status(m.service_port).await
    } else {
        service_ctrl::ServiceStatus::Stopped
    };
    let running = status == service_ctrl::ServiceStatus::Running;

    // 读取配置文件获取已配置的平台和 AI 信息
    let config = dirs::home_dir()
        .and_then(|h| std::fs::read_to_string(h.join(".openclaw/openclaw.json")).ok())
        .and_then(|d| serde_json::from_str::<serde_json::Value>(&d).ok())
        .unwrap_or(serde_json::json!({}));

    let ai_provider = config.get("ai").and_then(|a| a.get("provider")).and_then(|v| v.as_str()).unwrap_or("");
    let ai_model = config.get("ai").and_then(|a| a.get("model")).and_then(|v| v.as_str()).unwrap_or("");
    let has_ai = !ai_provider.is_empty();

    // 检测已配置的聊天平台
    let channels = config.get("channels").cloned().unwrap_or(serde_json::json!({}));

    serde_json::json!({
        "running": running,
        "port": port,
        "has_meta": meta.is_some(),
        "version": meta.as_ref().map(|m| m.version.as_str()).unwrap_or(""),
        "install_path": meta.as_ref().map(|m| m.install_path.as_str()).unwrap_or(""),
        "ai": {
            "configured": has_ai,
            "provider": ai_provider,
            "model": ai_model,
        },
        "channels": channels,
    })
}

/// JSON 深度合并（patch 中的值覆盖 target 对应 key，递归合并对象）
fn json_deep_merge(target: &mut serde_json::Value, patch: &serde_json::Value) {
    if let (Some(target_obj), Some(patch_obj)) = (target.as_object_mut(), patch.as_object()) {
        for (key, value) in patch_obj {
            if value.is_null() {
                target_obj.remove(key);
            } else if value.is_object() && target_obj.get(key).map_or(false, |v| v.is_object()) {
                json_deep_merge(target_obj.get_mut(key).unwrap(), value);
            } else {
                target_obj.insert(key.clone(), value.clone());
            }
        }
    } else {
        *target = patch.clone();
    }
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
                match skills_registry::install_paid_skill(&install_path, slug, &jwt).await {
                    Ok(()) => refreshed.push(slug.clone()),
                    Err(e) => eprintln!("[refresh] {} 刷新失败: {e}", slug),
                }
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
            validate_install_path,
            get_default_install_path,
            read_openclaw_config,
            write_openclaw_config,
            get_gateway_status,
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
