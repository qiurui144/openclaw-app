use anyhow::Result;
use secrecy::Secret;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{Emitter, Window};

use crate::platform_config::{DingtalkConfig, FeishuConfig, PlatformConfigs, QqConfig, WecomConfig};
#[allow(unused_imports)]
use crate::session_state::{SessionState, DownloadedFile};

// ── IPC DTO：从前端接收，立即转换为内部类型 ──────────────────────
#[derive(Clone, Deserialize)]
pub struct DeployConfigDto {
    pub install_path: String,
    pub service_port: u16,
    pub admin_password: String,   // 仅在 IPC boundary 使用
    pub domain_name: Option<String>,
    pub install_service: bool,
    pub start_on_boot: bool,
    pub source_mode: SourceModeDto,
    pub wecom_config: Option<WecomConfig>,
    pub dingtalk_config: Option<DingtalkConfig>,
    pub feishu_config: Option<FeishuConfig>,
    pub qq_config: Option<QqConfig>,
    pub ai_config: Option<AiConfigDto>,
}

/// AI 模型接入配置（OpenAI 兼容接口）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AiConfigDto {
    pub provider: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SourceModeDto {
    Bundled,
    Online { proxy_url: Option<String> },
    LocalZip { zip_path: String },
}

// ── 内部安全类型：admin_password 使用 Secret<String> ─────────────
pub struct DeployConfig {
    pub install_path: String,
    pub service_port: u16,
    pub admin_password: Secret<String>,
    pub domain_name: Option<String>,
    pub install_service: bool,
    pub start_on_boot: bool,
    pub source_mode: SourceMode,
    pub wecom_config: Option<WecomConfig>,
    pub dingtalk_config: Option<DingtalkConfig>,
    pub feishu_config: Option<FeishuConfig>,
    pub qq_config: Option<QqConfig>,
    pub ai_config: Option<AiConfigDto>,
}

pub enum SourceMode {
    Bundled,
    /// proxy_url = "http://127.0.0.1:{clash_port}"（Clash 启动后填入）
    Online { proxy_url: Option<String> },
    LocalZip(PathBuf),
}

impl From<DeployConfigDto> for DeployConfig {
    fn from(dto: DeployConfigDto) -> Self {
        DeployConfig {
            install_path: dto.install_path,
            service_port: dto.service_port,
            admin_password: Secret::new(dto.admin_password),
            domain_name: dto.domain_name,
            install_service: dto.install_service,
            start_on_boot: dto.start_on_boot,
            source_mode: match dto.source_mode {
                SourceModeDto::Bundled => SourceMode::Bundled,
                SourceModeDto::Online { proxy_url } =>
                    SourceMode::Online { proxy_url },
                SourceModeDto::LocalZip { zip_path } =>
                    SourceMode::LocalZip(PathBuf::from(zip_path)),
            },
            wecom_config: dto.wecom_config,
            dingtalk_config: dto.dingtalk_config,
            feishu_config: dto.feishu_config,
            qq_config: dto.qq_config,
            ai_config: dto.ai_config,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct DeployProgress {
    pub step: u32,
    pub total: u32,
    pub percent: u32,
    pub message: String,
}

/// 向前端发送进度事件
fn emit_progress(window: &Window, step: u32, total: u32, msg: &str) {
    let _ = window.emit("deploy:progress", DeployProgress {
        step,
        total,
        percent: (step * 100) / total,
        message: msg.to_string(),
    });
}

/// 异步命令直接调用，不 spawn 独立任务（确保事件在同一 async 上下文发出）
pub async fn do_deploy_direct(config: DeployConfig, window: &Window) -> Result<()> {
    do_deploy(&config, window).await
}

async fn do_deploy(config: &DeployConfig, window: &Window) -> Result<()> {
    const TOTAL: u32 = 11;
    eprintln!("[deploy] do_deploy START, install_path={}", config.install_path);

    // Step 1: 创建安装目录
    eprintln!("[deploy] step 1: emit_progress...");
    emit_progress(window, 1, TOTAL, "创建安装目录…");
    eprintln!("[deploy] step 1: emit_progress done");
    let _ = window.emit("deploy:log", format!("安装路径: {}", config.install_path));
    std::fs::create_dir_all(&config.install_path)?;
    let _ = window.emit("deploy:log", "目录创建完成");

    // Online 模式预检：自动探测网络环境
    if let SourceMode::Online { ref proxy_url } = config.source_mode {
        let _ = window.emit("deploy:log", "检测网络连通性…");
        let probe_ok = probe_connectivity(proxy_url.as_deref()).await;
        if probe_ok {
            let _ = window.emit("deploy:log", "网络连通，开始下载资源");
        } else if proxy_url.is_some() {
            let _ = window.emit("deploy:log", "⚠ 通过代理仍无法连通，下载可能失败");
        } else {
            let _ = window.emit("deploy:log", "⚠ 无法直连 npm/nodejs.org，如需代理请返回配置 Clash");
        }
    }

    // Step 2-4: 获取并解包资源（因 SourceMode 不同行为差异）
    emit_progress(window, 2, TOTAL, "获取 Node.js 运行时…");
    acquire_node(config, window).await?;

    emit_progress(window, 3, TOTAL, "获取 OpenClaw 安装包…");
    acquire_openclaw_package(config, window).await?;

    emit_progress(window, 4, TOTAL, "解包 OpenClaw（含所有依赖，请稍候）…");
    extract_openclaw(config, window)?;

    // Step 5: 写入主配置
    emit_progress(window, 5, TOTAL, "写入配置文件…");
    write_main_config(config)?;
    let _ = window.emit("deploy:log", format!("配置写入 ~/.openclaw/openclaw.json（端口 {}）", config.service_port));

    // Step 6: 写入平台集成配置
    emit_progress(window, 6, TOTAL, "写入平台集成配置…");
    crate::platform_config::write_platform_config(
        &config.install_path,
        PlatformConfigs {
            wecom: config.wecom_config.as_ref(),
            dingtalk: config.dingtalk_config.as_ref(),
            feishu: config.feishu_config.as_ref(),
            qq: config.qq_config.as_ref(),
        },
    )?;

    // Step 7: 注册系统服务（失败不阻断部署，仅记录警告）
    if config.install_service {
        emit_progress(window, 7, TOTAL, "注册系统服务…");
        let _ = window.emit("deploy:log", "尝试注册 systemd/launchd/schtasks 服务…");
        if let Err(e) = install_service(config) {
            let _ = window.emit("deploy:progress", DeployProgress {
                step: 7, total: TOTAL, percent: 63,
                message: format!("系统服务注册失败（{}），将直接启动进程", e),
            });
        }
    }

    // Step 8: 启动服务
    emit_progress(window, 8, TOTAL, "启动 OpenClaw 服务…");
    let _ = window.emit("deploy:log", format!("启动 node {} gateway --port {}",
        PathBuf::from(&config.install_path).join("openclaw_pkg/package/openclaw.mjs").display(),
        config.service_port));
    start_service(config)?;

    // Step 9: 健康检查（每 2 秒更新进度）
    for i in 0..15u32 {
        let pct = 82 + i * (95 - 82) / 15;
        let _ = window.emit("deploy:progress", DeployProgress {
            step: 9, total: TOTAL, percent: pct,
            message: format!("等待服务就绪… ({}/15)", i + 1),
        });
        let ok = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            tokio::net::TcpStream::connect(format!("127.0.0.1:{}", config.service_port))
        ).await.map(|r| r.is_ok()).unwrap_or(false);
        if ok && reqwest::get(format!("http://127.0.0.1:{}/health", config.service_port))
            .await.map(|r| r.status().is_success()).unwrap_or(false)
        {
            break;
        }
        if i == 14 {
            anyhow::bail!("服务在 30 秒内未能正常启动，请查看日志");
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    // Step 10: 生成卸载脚本
    emit_progress(window, 10, TOTAL, "生成卸载脚本…");
    write_uninstall_script(config)?;

    // Step 11: 写入安装记录
    emit_progress(window, 11, TOTAL, "完成！");
    write_deploy_meta(config)?;
    crate::session_state::clear(Some(&config.install_path))?;

    let _ = window.emit("deploy:done", ());
    Ok(())
}

// ── 各步骤实现 ────────────────────────────────────────────────────

async fn acquire_node(config: &DeployConfig, window: &Window) -> Result<()> {
    let node_dir = PathBuf::from(&config.install_path).join("node");
    std::fs::create_dir_all(&node_dir)?;
    let dest = node_dir.join(if cfg!(windows) { "node.exe" } else { "node" });

    match &config.source_mode {
        SourceMode::Bundled => {
            #[cfg(feature = "bundled")]
            {
                #[cfg(target_os = "linux")]
                let data = include_bytes!("../../resources/binaries/linux/node");
                #[cfg(target_os = "windows")]
                let data = include_bytes!("..\\..\\resources\\binaries\\windows\\node.exe");
                std::fs::write(&dest, &data[..])?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&dest,
                        std::fs::Permissions::from_mode(0o755))?;
                }
            }
            #[cfg(not(feature = "bundled"))]
            anyhow::bail!("Bundled 模式需要使用 --features bundled 编译（资源文件未内嵌）");
        }
        SourceMode::Online { proxy_url } => {
            download_node_binary(&dest, proxy_url.as_deref(), window).await?;
        }
        SourceMode::LocalZip(zip_path) => {
            extract_from_zip(zip_path, "node", &dest)?;
        }
    }
    Ok(())
}

async fn acquire_openclaw_package(config: &DeployConfig, window: &Window) -> Result<()> {
    let dest = PathBuf::from(&config.install_path).join("openclaw.tgz");
    match &config.source_mode {
        SourceMode::Bundled => {
            #[cfg(feature = "bundled")]
            {
                #[cfg(target_os = "linux")]
                let data = include_bytes!("../../resources/binaries/linux/openclaw.tgz");
                #[cfg(target_os = "windows")]
                let data = include_bytes!("..\\..\\resources\\binaries\\windows\\openclaw.tgz");
                std::fs::write(&dest, &data[..])?;
            }
            #[cfg(not(feature = "bundled"))]
            anyhow::bail!("Bundled 模式需要使用 --features bundled 编译");
        }
        SourceMode::Online { proxy_url } => {
            download_openclaw_package(&dest, proxy_url.as_deref(), window).await?;
        }
        SourceMode::LocalZip(zip_path) => {
            extract_from_zip(zip_path, "openclaw.tgz", &dest)?;
        }
    }
    Ok(())
}

fn extract_openclaw(config: &DeployConfig, window: &Window) -> Result<()> {
    let tgz = PathBuf::from(&config.install_path).join("openclaw.tgz");
    let dest = PathBuf::from(&config.install_path).join("openclaw_pkg");
    std::fs::create_dir_all(&dest)?;

    let file = std::fs::File::open(&tgz)?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);

    // 逐条解包并向前端发送文件名日志
    for (i, entry) in archive.entries()?.enumerate() {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().to_string();
        entry.unpack_in(&dest)?;
        // 每 50 个文件更新一次日志，避免消息过多
        if i % 50 == 0 {
            let _ = window.emit("deploy:log", format!("解包 {}", path));
        }
    }
    std::fs::remove_file(&tgz)?;
    Ok(())
}

fn write_main_config(config: &DeployConfig) -> Result<()> {
    use secrecy::ExposeSecret;
    let config_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw");
    std::fs::create_dir_all(&config_dir)?;

    let password_hash = bcrypt::hash(
        config.admin_password.expose_secret(), bcrypt::DEFAULT_COST)?;

    let mut gateway = serde_json::json!({
        "port": config.service_port,
        "auth": {
            "mode": "password",
            "passwordHash": password_hash
        }
    });
    if let Some(domain) = &config.domain_name {
        gateway["publicUrl"] = serde_json::json!(format!("https://{}", domain));
    }

    let mut cfg = serde_json::json!({ "gateway": gateway });

    if let Some(ai) = &config.ai_config {
        if !ai.api_key.is_empty() {
            cfg["ai"] = serde_json::json!({
                "provider": ai.provider,
                "baseUrl": ai.base_url,
                "apiKey": ai.api_key,
                "model": ai.model,
            });
        }
    }

    std::fs::write(
        config_dir.join("openclaw.json"),
        serde_json::to_string_pretty(&cfg)?
    )?;
    Ok(())
}

fn install_service(config: &DeployConfig) -> Result<()> {
    let node_bin = PathBuf::from(&config.install_path)
        .join("node")
        .join(if cfg!(windows) { "node.exe" } else { "node" });
    // npm tarball 解压后入口在 openclaw_pkg/package/openclaw.mjs
    let script = PathBuf::from(&config.install_path)
        .join("openclaw_pkg")
        .join("package")
        .join("openclaw.mjs");
    let port = config.service_port;
    let elevated = crate::system_check::is_elevated();

    #[cfg(target_os = "linux")]
    {
        let unit_content = format!(
            "[Unit]\nDescription=OpenClaw Gateway\nAfter=network.target\n\n\
             [Service]\nType=simple\n\
             ExecStart={} {} gateway --port {}\n\
             Restart=on-failure\nRestartSec=10\n\
             Environment=NODE_ENV=production\n\
             StandardOutput=journal\nStandardError=journal\n\n\
             [Install]\nWantedBy={}\n",
            node_bin.display(), script.display(), port,
            if elevated { "multi-user.target" } else { "default.target" }
        );

        if elevated {
            // root：写入系统级 unit，无需 D-Bus session
            let unit_dir = PathBuf::from("/etc/systemd/system");
            std::fs::create_dir_all(&unit_dir)?;
            std::fs::write(unit_dir.join("openclaw.service"), unit_content)?;
            std::process::Command::new("systemctl").args(["daemon-reload"]).status()?;
            if config.start_on_boot {
                std::process::Command::new("systemctl")
                    .args(["enable", "openclaw.service"]).status()?;
            }
        } else {
            // 普通用户：写入用户级 unit，需要 D-Bus session
            let unit_dir = dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config/systemd/user");
            std::fs::create_dir_all(&unit_dir)?;
            std::fs::write(unit_dir.join("openclaw.service"), unit_content)?;
            std::process::Command::new("timeout")
                .args(["10", "systemctl", "--user", "daemon-reload"]).status()?;
            if config.start_on_boot {
                std::process::Command::new("timeout")
                    .args(["10", "systemctl", "--user", "enable", "openclaw.service"]).status()?;
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let plist = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
             \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
             <plist version=\"1.0\"><dict>\n\
             <key>Label</key><string>com.openclaw.gateway</string>\n\
             <key>ProgramArguments</key><array>\n\
             <string>{node}</string><string>{script}</string>\
             <string>gateway</string><string>--port</string><string>{port}</string>\n\
             </array>\n\
             <key>EnvironmentVariables</key><dict>\
             <key>NODE_ENV</key><string>production</string></dict>\n\
             <key>RunAtLoad</key><{boot}/>\n\
             <key>KeepAlive</key><true/>\n\
             <key>StandardOutPath</key><string>/tmp/openclaw.log</string>\n\
             <key>StandardErrorPath</key><string>/tmp/openclaw.log</string>\n\
             </dict></plist>\n",
            node = node_bin.display(), script = script.display(), port = port,
            boot = if config.start_on_boot { "true" } else { "false" },
        );
        let plist_dir = if elevated {
            PathBuf::from("/Library/LaunchDaemons")
        } else {
            dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
                .join("Library/LaunchAgents")
        };
        std::fs::create_dir_all(&plist_dir)?;
        std::fs::write(plist_dir.join("com.openclaw.gateway.plist"), plist)?;
    }
    #[cfg(target_os = "windows")]
    {
        let cmd = format!("\"{}\" \"{}\" gateway --port {}",
            node_bin.display(), script.display(), port);
        let mut args = vec!["/Create", "/F", "/SC", "ONLOGON",
                            "/TN", "OpenClaw Gateway", "/TR", &cmd];
        // 管理员：以 SYSTEM 身份运行，普通用户：以当前用户身份运行
        if elevated { args.extend(["/RU", "SYSTEM"]); }
        std::process::Command::new("schtasks").args(&args).status()?;
    }
    Ok(())
}

fn start_service(config: &DeployConfig) -> Result<()> {
    let node_bin = PathBuf::from(&config.install_path)
        .join("node").join(if cfg!(windows) { "node.exe" } else { "node" });
    // npm tarball 解压后入口在 openclaw_pkg/package/openclaw.mjs
    let script = PathBuf::from(&config.install_path)
        .join("openclaw_pkg").join("package").join("openclaw.mjs");

    if !node_bin.exists() {
        anyhow::bail!("Node.js 不存在: {}，解压步骤可能失败", node_bin.display());
    }
    if !script.exists() {
        anyhow::bail!("入口脚本不存在: {}，解压步骤可能失败", script.display());
    }

    #[cfg(target_os = "linux")]
    {
        // 只有安装了 unit 文件才走 systemctl，否则直接 spawn
        let elevated = crate::system_check::is_elevated();
        let unit_exists = if elevated {
            PathBuf::from("/etc/systemd/system/openclaw.service").exists()
        } else {
            dirs::home_dir()
                .map(|h| h.join(".config/systemd/user/openclaw.service").exists())
                .unwrap_or(false)
        };

        if config.install_service && unit_exists {
            let ok = if elevated {
                std::process::Command::new("systemctl")
                    .args(["start", "openclaw.service"])
                    .status().map(|s| s.success()).unwrap_or(false)
            } else {
                std::process::Command::new("timeout")
                    .args(["10", "systemctl", "--user", "start", "openclaw.service"])
                    .status().map(|s| s.success()).unwrap_or(false)
            };
            if ok { return Ok(()); }
        }

        // 直接启动进程（未选 systemd 或 systemctl 失败时）
        std::process::Command::new(&node_bin)
            .args([script.to_str().unwrap(), "gateway",
                   "--port", &config.service_port.to_string()])
            .env("NODE_ENV", "production")
            .spawn()
            .map_err(|e| anyhow::anyhow!("启动 OpenClaw 失败: {e}\n节点: {}", node_bin.display()))?;
    }
    #[cfg(target_os = "windows")]
    {
        if config.install_service {
            let ran = std::process::Command::new("schtasks")
                .args(["/Run", "/TN", "OpenClaw Gateway"])
                .status().map(|s| s.success()).unwrap_or(false);
            if ran { return Ok(()); }
        }
        std::process::Command::new(&node_bin)
            .args([script.to_str().unwrap(), "gateway",
                   "--port", &config.service_port.to_string()])
            .env("NODE_ENV", "production")
            .spawn()
            .map_err(|e| anyhow::anyhow!("启动 OpenClaw 失败: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        let elevated = crate::system_check::is_elevated();
        let plist_path = if elevated {
            PathBuf::from("/Library/LaunchDaemons/com.openclaw.gateway.plist")
        } else {
            dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
                .join("Library/LaunchAgents/com.openclaw.gateway.plist")
        };

        if config.install_service && plist_path.exists() {
            let ok = std::process::Command::new("launchctl")
                .args(["load", "-w", plist_path.to_str().unwrap_or("")])
                .status().map(|s| s.success()).unwrap_or(false);
            if ok { return Ok(()); }
        }

        // 直接启动进程（未选 launchd 或 launchctl 失败时）
        std::process::Command::new(&node_bin)
            .args([script.to_str().unwrap(), "gateway",
                   "--port", &config.service_port.to_string()])
            .env("NODE_ENV", "production")
            .spawn()
            .map_err(|e| anyhow::anyhow!("启动 OpenClaw 失败: {e}\n节点: {}", node_bin.display()))?;
    }
    Ok(())
}

pub async fn health_check(port: u16) -> Result<()> {
    let url = format!("http://127.0.0.1:{}/health", port);
    for i in 0..15 {
        let has_listener = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
        ).await.map(|r| r.is_ok()).unwrap_or(false);

        if has_listener && reqwest::get(&url).await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
        {
            return Ok(());
        }
        if i < 14 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
    let has_listener = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
    ).await.map(|r| r.is_ok()).unwrap_or(false);

    if has_listener {
        use sysinfo::System;
        let mut sys = System::new();
        sys.refresh_processes();
        let occupant = sys.processes().values()
            .find(|p| p.name().contains("openclaw"))
            .map(|p| format!("PID {}", p.pid()))
            .unwrap_or_else(|| "其他进程".into());
        anyhow::bail!(
            "端口 {} 被 {} 占用但未响应健康检查，请检查端口冲突", port, occupant
        )
    }
    anyhow::bail!("服务在 30 秒内未能正常启动，请查看日志")
}

fn write_uninstall_script(config: &DeployConfig) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let elevated = crate::system_check::is_elevated();
        let service_cmds = if elevated {
            "systemctl stop openclaw.service 2>/dev/null || true\n\
             systemctl disable openclaw.service 2>/dev/null || true\n\
             rm -f /etc/systemd/system/openclaw.service\n\
             systemctl daemon-reload\n".to_string()
        } else {
            "systemctl --user stop openclaw.service 2>/dev/null || true\n\
             systemctl --user disable openclaw.service 2>/dev/null || true\n\
             rm -f ~/.config/systemd/user/openclaw.service\n\
             systemctl --user daemon-reload 2>/dev/null || true\n".to_string()
        };
        let script = format!(
            "#!/bin/bash\nset -e\n\
             {}\
             rm -rf \"{}\"\n\
             rm -rf ~/.openclaw\n\
             echo '✓ OpenClaw 已卸载'\n",
            service_cmds, config.install_path
        );
        let path = PathBuf::from(&config.install_path).join("uninstall.sh");
        std::fs::write(&path, &script)?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(target_os = "macos")]
    {
        let elevated = crate::system_check::is_elevated();
        let plist = if elevated {
            "/Library/LaunchDaemons/com.openclaw.gateway.plist".to_string()
        } else {
            format!("{}/Library/LaunchAgents/com.openclaw.gateway.plist",
                dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default())
        };
        let script = format!(
            "#!/bin/bash\nset -e\n\
             launchctl unload -w \"{plist}\" 2>/dev/null || true\n\
             rm -f \"{plist}\"\n\
             rm -rf \"{dir}\"\n\
             rm -rf ~/.openclaw\n\
             echo '✓ OpenClaw 已卸载'\n",
            plist = plist, dir = config.install_path
        );
        let path = PathBuf::from(&config.install_path).join("uninstall.sh");
        std::fs::write(&path, &script)?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(windows)]
    {
        let script = format!(
            "@echo off\r\n\
             schtasks /End /TN \"OpenClaw Gateway\" 2>nul\r\n\
             schtasks /Delete /TN \"OpenClaw Gateway\" /F 2>nul\r\n\
             rmdir /S /Q \"{}\"\r\n\
             rmdir /S /Q \"%USERPROFILE%\\.openclaw\"\r\n\
             echo OpenClaw 已卸载\r\n",
            config.install_path
        );
        std::fs::write(
            PathBuf::from(&config.install_path).join("uninstall.bat"),
            script
        )?;
    }
    Ok(())
}

fn write_deploy_meta(config: &DeployConfig) -> Result<()> {
    let meta_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw");
    let meta = serde_json::json!({
        "install_path": config.install_path,
        "version": env!("CARGO_PKG_VERSION"),
        "deployed_at": chrono::Utc::now().to_rfc3339(),
        "service_port": config.service_port,
    });
    std::fs::write(
        meta_dir.join("deploy_meta.json"),
        serde_json::to_string_pretty(&meta)?
    )?;
    Ok(())
}

// ── 网络下载辅助（Online 模式）────────────────────────────────────

async fn download_node_binary(dest: &PathBuf, proxy: Option<&str>, window: &Window) -> Result<()> {
    let version = "22.17.0";
    #[cfg(target_os = "linux")]
    let url = format!("https://nodejs.org/dist/v{}/node-v{}-linux-x64.tar.xz", version, version);
    #[cfg(target_os = "windows")]
    let url = format!("https://nodejs.org/dist/v{}/node-v{}-win-x64.zip", version, version);
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let url = format!("https://nodejs.org/dist/v{}/node-v{}-darwin-arm64.tar.gz", version, version);
    #[cfg(all(target_os = "macos", not(target_arch = "aarch64")))]
    let url = format!("https://nodejs.org/dist/v{}/node-v{}-darwin-x64.tar.gz", version, version);
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    let url = { let _ = (dest, proxy, window); anyhow::bail!("当前平台不支持在线下载 Node.js") };

    let archive_tmp = dest.with_file_name("node_archive.tmp");
    download_with_progress(&url, &archive_tmp, proxy, "Node.js", window).await?;
    extract_node_from_archive(&archive_tmp, dest)?;
    std::fs::remove_file(&archive_tmp).ok();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dest, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

fn extract_node_from_archive(archive: &PathBuf, dest: &PathBuf) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let data = std::fs::read(archive)?;
        let xz = xz2::read::XzDecoder::new(std::io::Cursor::new(data));
        let mut tar = tar::Archive::new(xz);
        for entry in tar.entries()? {
            let mut e = entry?;
            let path = e.path()?.into_owned();
            if path.ends_with("bin/node") {
                e.unpack(dest)?;
                return Ok(());
            }
        }
        anyhow::bail!("Node.js 压缩包中未找到 bin/node");
    }
    #[cfg(target_os = "windows")]
    {
        let data = std::fs::read(archive)?;
        let cursor = std::io::Cursor::new(data);
        let mut zip = zip::ZipArchive::new(cursor)?;
        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            if file.name().ends_with("node.exe") {
                let mut out = std::fs::File::create(dest)?;
                std::io::copy(&mut file, &mut out)?;
                return Ok(());
            }
        }
        anyhow::bail!("Node.js 压缩包中未找到 node.exe");
    }
    #[cfg(target_os = "macos")]
    {
        // macOS 发行版为 .tar.gz
        let file = std::fs::File::open(archive)?;
        let gz = flate2::read::GzDecoder::new(file);
        let mut tar = tar::Archive::new(gz);
        for entry in tar.entries()? {
            let mut e = entry?;
            let path = e.path()?.into_owned();
            if path.ends_with("bin/node") {
                e.unpack(dest)?;
                return Ok(());
            }
        }
        anyhow::bail!("Node.js 压缩包中未找到 bin/node");
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    anyhow::bail!("当前平台不支持自动提取 Node.js 二进制");
}

async fn download_openclaw_package(dest: &PathBuf, proxy: Option<&str>, window: &Window) -> Result<()> {
    // 从 npm registry 查询最新版本的 tarball 地址（npmmirror 优先，回退 npmjs）
    let tarball_url = fetch_npm_tarball_url("openclaw", proxy).await?;
    download_with_progress(&tarball_url, dest, proxy, "OpenClaw", window).await
}

async fn fetch_npm_tarball_url(pkg: &str, proxy: Option<&str>) -> Result<String> {
    let registries = [
        format!("https://registry.npmmirror.com/{}/latest", pkg),
        format!("https://registry.npmjs.org/{}/latest", pkg),
    ];
    let mut last_err = anyhow::anyhow!("所有 npm 镜像均不可达");
    for url in &registries {
        let client = build_client(proxy)?;
        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let info: serde_json::Value = resp.json().await?;
                let tarball = info["dist"]["tarball"].as_str()
                    .ok_or_else(|| anyhow::anyhow!("npm registry 响应缺少 dist.tarball 字段"))?
                    .to_string();
                return Ok(tarball);
            }
            Ok(resp) => {
                last_err = anyhow::anyhow!("npm registry {} 返回 {}", url, resp.status());
            }
            Err(e) => {
                last_err = anyhow::anyhow!("连接 npm registry {} 失败: {}", url, e);
            }
        }
    }
    Err(last_err)
}

/// 流式下载并实时向前端发送下载进度（每 512KB 一条日志）
async fn download_with_progress(
    url: &str, dest: &PathBuf, proxy: Option<&str>, label: &str, window: &Window,
) -> Result<()> {
    use std::io::Write;
    let client = build_client(proxy)?;
    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("下载 {} 失败，HTTP {}: {}", label, resp.status(), url);
    }
    let total = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut last_logged: u64 = 0;
    let log_interval: u64 = 512 * 1024; // 每 512KB 记一条

    let mut file = std::fs::File::create(dest)?;
    let mut stream = resp;
    while let Some(chunk) = stream.chunk().await? {
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        if downloaded - last_logged >= log_interval || downloaded == total {
            last_logged = downloaded;
            let msg = if total > 0 {
                format!("下载 {} {:.1}/{:.1} MB ({:.0}%)",
                    label,
                    downloaded as f64 / 1_048_576.0,
                    total as f64 / 1_048_576.0,
                    downloaded as f64 * 100.0 / total as f64)
            } else {
                format!("下载 {} {:.1} MB", label, downloaded as f64 / 1_048_576.0)
            };
            let _ = window.emit("deploy:log", msg);
        }
    }
    Ok(())
}

fn build_client(proxy: Option<&str>) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300));
    if let Some(proxy_url) = proxy {
        builder = builder.proxy(reqwest::Proxy::all(proxy_url)?);
    }
    Ok(builder.build()?)
}

/// 探测 npm registry 和 nodejs.org 连通性（3 秒超时，不阻塞部署）
async fn probe_connectivity(proxy: Option<&str>) -> bool {
    let client = match build_client(proxy) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let probe = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap_or(client);
    probe.head("https://registry.npmmirror.com/openclaw")
        .send().await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

fn extract_from_zip(zip_path: &PathBuf, entry_name: &str, dest: &PathBuf) -> Result<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.name().ends_with(entry_name) {
            let mut data = Vec::new();
            std::io::copy(&mut entry, &mut data)?;
            std::fs::write(dest, data)?;
            return Ok(());
        }
    }
    anyhow::bail!("ZIP 中未找到 {}", entry_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    #[test]
    fn test_dto_to_config_conversion() {
        let dto = DeployConfigDto {
            install_path: "/opt/openclaw".into(),
            service_port: 18789,
            admin_password: "Secret123".into(),
            domain_name: None,
            install_service: true,
            start_on_boot: true,
            source_mode: SourceModeDto::Bundled,
            wecom_config: None,
            dingtalk_config: None,
            feishu_config: None,
            qq_config: None,
            ai_config: None,
        };
        let config = DeployConfig::from(dto);
        // 密码已包装为 Secret，不可直接读取（需 expose_secret()）
        assert_eq!(config.admin_password.expose_secret(), "Secret123");
        assert_eq!(config.service_port, 18789);
    }

    #[test]
    fn test_deploy_progress_serializable() {
        let p = DeployProgress {
            step: 1,
            total: 11,
            percent: 9,
            message: "创建安装目录".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("创建安装目录"));
        assert!(json.contains("\"percent\":9"));
    }

    #[test]
    fn test_source_mode_local_zip_path() {
        let dto = DeployConfigDto {
            install_path: "/opt/openclaw".into(),
            service_port: 18789,
            admin_password: "Secret123".into(),
            domain_name: None,
            install_service: false,
            start_on_boot: false,
            source_mode: SourceModeDto::LocalZip {
                zip_path: "/tmp/openclaw.zip".into()
            },
            wecom_config: None,
            dingtalk_config: None,
            feishu_config: None,
            qq_config: None,
            ai_config: None,
        };
        let config = DeployConfig::from(dto);
        assert!(matches!(config.source_mode, SourceMode::LocalZip(_)));
    }

    /// 验证前端真实发出的 JSON 格式能被正确反序列化。
    /// 这些 JSON 字符串与前端 buildSourceMode() 的输出完全一致。
    #[test]
    fn test_source_mode_json_deserialization() {
        // bundled 模式
        let v: SourceModeDto = serde_json::from_str(r#"{"type":"bundled"}"#).unwrap();
        assert!(matches!(v, SourceModeDto::Bundled));

        // online 模式（无代理）
        let v: SourceModeDto = serde_json::from_str(r#"{"type":"online","proxy_url":null}"#).unwrap();
        assert!(matches!(v, SourceModeDto::Online { proxy_url: None }));

        // online 模式（带代理）
        let v: SourceModeDto = serde_json::from_str(
            r#"{"type":"online","proxy_url":"http://127.0.0.1:7890"}"#
        ).unwrap();
        assert!(matches!(v, SourceModeDto::Online { proxy_url: Some(_) }));

        // local_zip 模式
        let v: SourceModeDto = serde_json::from_str(
            r#"{"type":"local_zip","zip_path":"/tmp/openclaw.zip"}"#
        ).unwrap();
        assert!(matches!(v, SourceModeDto::LocalZip { .. }));
    }
}
