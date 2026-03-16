use anyhow::Result;
use secrecy::Secret;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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

/// node 二进制的平台路径（完整发行版解压后的位置）
pub fn node_bin_path(install_path: &str) -> PathBuf {
    let base = PathBuf::from(install_path).join("node");
    if cfg!(windows) {
        base.join("node.exe")
    } else {
        // 完整发行版: node/bin/node；兼容旧版单文件: node/node
        let full = base.join("bin/node");
        if full.exists() { full } else { base.join("node") }
    }
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

// ── 安装路径校验 ─────────────────────────────────────────────────

/// 系统关键目录黑名单（路径规范化后比较）
const PATH_BLACKLIST: &[&str] = &[
    "/", "/bin", "/sbin", "/usr", "/usr/bin", "/usr/sbin", "/usr/lib",
    "/etc", "/var", "/tmp", "/dev", "/proc", "/sys", "/boot", "/root",
    "/lib", "/lib64",
];

#[cfg(windows)]
const PATH_BLACKLIST_WIN: &[&str] = &[
    "C:\\", "C:\\Windows", "C:\\Windows\\System32",
    "C:\\Program Files", "C:\\Program Files (x86)",
];

/// 规范化路径：解析 . 和 .. 组件（纯逻辑处理，不依赖文件系统存在性）
fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => { result.pop(); }
            Component::CurDir => {}
            _ => result.push(component),
        }
    }
    result
}

/// 校验安装路径：非空、合法字符、不在黑名单、可写、磁盘空间充足（≥500MB）
pub fn validate_install_path(path: &str) -> Result<()> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        anyhow::bail!("安装路径不能为空");
    }

    // 非法字符检测（允许中文路径，但禁止控制字符和部分 shell 特殊字符）
    let invalid_chars = ['<', '>', '|', '"', '\0', '*', '?'];
    if trimmed.chars().any(|c| invalid_chars.contains(&c) || c.is_control()) {
        anyhow::bail!("安装路径包含非法字符（<>|\"*? 或控制字符）");
    }

    // 规范化路径：解析符号链接（已存在时）+ 清理 .. 和 . 组件
    let raw_path = Path::new(trimmed);
    let normalized = if raw_path.exists() {
        // 已存在的路径：canonicalize 解析符号链接
        raw_path.canonicalize().unwrap_or_else(|_| normalize_path(raw_path))
    } else {
        normalize_path(raw_path)
    };

    // 黑名单检测（Unix）
    #[cfg(unix)]
    {
        let path_str = normalized.to_string_lossy();
        for &bl in PATH_BLACKLIST {
            if path_str == bl || path_str == format!("{}/", bl) {
                anyhow::bail!("安装路径不能是系统关键目录：{}", bl);
            }
        }
    }

    // 黑名单检测（Windows）
    #[cfg(windows)]
    {
        let path_upper = normalized.to_string_lossy().to_uppercase();
        for &bl in PATH_BLACKLIST_WIN {
            if path_upper == bl.to_uppercase()
                || path_upper == format!("{}\\", bl).to_uppercase()
            {
                anyhow::bail!("安装路径不能是系统关键目录：{}", bl);
            }
        }
    }

    // 可写检测：尝试在目标（或其最近的已存在祖先）创建临时文件
    let test_dir = if normalized.exists() {
        normalized.clone()
    } else {
        // 找到最近存在的父目录
        let mut parent = normalized.clone();
        loop {
            if parent.exists() { break; }
            if !parent.pop() {
                anyhow::bail!("安装路径无效：无法定位可写的父目录");
            }
        }
        parent
    };
    let probe = test_dir.join(".openclaw_write_probe");
    match std::fs::write(&probe, b"probe") {
        Ok(()) => { let _ = std::fs::remove_file(&probe); }
        Err(e) => anyhow::bail!("安装路径不可写（{}）：{}", test_dir.display(), e),
    }

    // 磁盘空间检测（≥500MB）
    check_disk_space(&test_dir, 500)?;

    Ok(())
}

/// 检查指定路径所在磁盘的可用空间（单位 MB）
fn check_disk_space(path: &Path, required_mb: u64) -> Result<()> {
    use sysinfo::Disks;
    let disks = Disks::new_with_refreshed_list();
    let mut best_match: Option<(usize, u64)> = None; // (mount_point_len, available_bytes)

    for disk in disks.list() {
        let mount = disk.mount_point();
        if path.starts_with(mount) {
            let mount_len = mount.to_string_lossy().len();
            if best_match.map_or(true, |(len, _)| mount_len > len) {
                best_match = Some((mount_len, disk.available_space()));
            }
        }
    }

    if let Some((_, available)) = best_match {
        let available_mb = available / (1024 * 1024);
        if available_mb < required_mb {
            anyhow::bail!(
                "磁盘空间不足：可用 {} MB，至少需要 {} MB",
                available_mb, required_mb
            );
        }
    }
    // 如果没匹配到磁盘（例如网络路径），跳过空间检查
    Ok(())
}

/// 异步命令直接调用，不 spawn 独立任务（确保事件在同一 async 上下文发出）
pub async fn do_deploy_direct(config: DeployConfig, window: &Window) -> Result<()> {
    do_deploy(&config, window).await
}

async fn do_deploy(config: &DeployConfig, window: &Window) -> Result<()> {
    const TOTAL: u32 = 11;

    // Step 0: 安装路径预检
    let _ = window.emit("deploy:log", "校验安装路径…");
    validate_install_path(&config.install_path)?;

    // Step 1: 创建安装目录
    emit_progress(window, 1, TOTAL, "创建安装目录…");
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

    // Step 4.5: 释放预缓存的 Skills
    let _ = window.emit("deploy:log", "安装预缓存的 Skills…");
    install_bundled_skills(config, window);

    // Step 4.6: 安装必需的插件（如 qqbot）
    if config.qq_config.is_some() {
        let _ = window.emit("deploy:log", "安装 QQ Bot 插件（@sliverp/qqbot）…");
        install_plugin(config, "qqbot", "@sliverp/qqbot", window);
    }

    // Step 4.7: 安装所有自带插件的依赖（pnpm 优先）
    let _ = window.emit("deploy:log", "安装插件依赖（pnpm 优先）…");
    install_plugin_dependencies(config, window);

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

    match &config.source_mode {
        SourceMode::Bundled => {
            // 离线模式：写入内嵌的 node 二进制（fat tarball 已含 node_modules，不需要 npm）
            #[cfg(feature = "bundled")]
            {
                let dest = node_dir.join(if cfg!(windows) { "node.exe" } else { "node" });
                #[cfg(target_os = "linux")]
                let data = include_bytes!("../../resources/binaries/linux/node");
                #[cfg(target_os = "windows")]
                let data = include_bytes!("..\\..\\resources\\binaries\\windows\\node.exe");
                #[cfg(target_os = "macos")]
                let data = include_bytes!("../../resources/binaries/macos/node");
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
            // Online 模式：直接下载完整 Node.js 发行版（含 npm）
            download_node_full(&node_dir, proxy_url.as_deref(), window).await?;
        }
        SourceMode::LocalZip(zip_path) => {
            let dest = node_dir.join(if cfg!(windows) { "node.exe" } else { "node" });
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
                #[cfg(target_os = "macos")]
                let data = include_bytes!("../../resources/binaries/macos/openclaw.tgz");
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
        if i % 50 == 0 {
            let _ = window.emit("deploy:log", format!("解包 {}", path));
        }
    }
    std::fs::remove_file(&tgz)?;

    // Bundled 模式的 fat tarball 已含 node_modules，无需 npm install
    // Online / LocalZip 模式的 npm tarball 不含 node_modules，需要安装依赖
    let pkg_dir = dest.join("package");
    let needs_install = !pkg_dir.join("node_modules").exists();
    if needs_install {
        let _ = window.emit("deploy:log", "安装依赖（pnpm 优先，npmmirror 源）…");
        install_npm_dependencies(config, &pkg_dir, window)?;
    } else {
        let _ = window.emit("deploy:log", "node_modules 已就绪（离线包）");
    }

    Ok(())
}

/// 安装 OpenClaw 插件（通过 openclaw plugins install 命令）
fn install_plugin(config: &DeployConfig, name: &str, pkg: &str, window: &Window) {
    let node_bin = node_bin_path(&config.install_path);
    let script = PathBuf::from(&config.install_path)
        .join("openclaw_pkg").join("package").join("openclaw.mjs");
    if !node_bin.exists() || !script.exists() {
        let _ = window.emit("deploy:log", format!("跳过 {} 插件安装（node/openclaw 不可用）", name));
        return;
    }
    let result = std::process::Command::new(&node_bin)
        .args([script.to_str().unwrap_or_default(), "plugins", "install", pkg])
        .env("NODE_ENV", "production")
        .output();
    match result {
        Ok(out) if out.status.success() => {
            let _ = window.emit("deploy:log", format!("{} 插件安装成功", name));
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let _ = window.emit("deploy:log", format!("{} 插件安装失败（非致命）: {}", name, stderr));
        }
        Err(e) => {
            let _ = window.emit("deploy:log", format!("{} 插件安装失败（非致命）: {}", name, e));
        }
    }
}

/// 释放预缓存的官方 Skills 到安装目录（Bundled 模式从内嵌资源，Online 时可选跳过）
fn install_bundled_skills(config: &DeployConfig, window: &Window) {
    use tauri::Manager;

    let skills_dest = PathBuf::from(&config.install_path)
        .join("openclaw_pkg").join("package").join("skills");

    // 如果 skills 目录已存在且非空，跳过（保留用户自定义）
    if skills_dest.exists() && std::fs::read_dir(&skills_dest).map(|mut d| d.next().is_some()).unwrap_or(false) {
        let _ = window.emit("deploy:log", "Skills 目录已存在，跳过预装");
        return;
    }

    // 通过 Tauri resource_dir 查找打包的 Skills 资源
    // CI 构建时 fetch_resources.sh 将 Skills 下载到 resources/skills/，
    // tauri.conf.json 的 bundle.resources 将其打包到应用资源目录
    let app = window.app_handle();
    let candidates: Vec<PathBuf> = vec![
        // 发布构建：Tauri 资源目录下的 skills/
        app.path().resource_dir().ok().map(|p| p.join("skills")),
        // 开发模式回退：项目 resources 目录
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources/skills")),
    ].into_iter().flatten().collect();

    for src_dir in &candidates {
        if src_dir.exists() && src_dir.is_dir() {
            let _ = std::fs::create_dir_all(&skills_dest);
            match copy_dir_contents(src_dir, &skills_dest) {
                Ok(count) if count > 0 => {
                    let _ = window.emit("deploy:log", format!("预装 {} 个 Skills（来源: {}）", count, src_dir.display()));
                    return;
                }
                Ok(_) => continue,
                Err(e) => {
                    let _ = window.emit("deploy:log", format!("Skills 预装失败（非致命）: {}", e));
                }
            }
        }
    }
    let _ = window.emit("deploy:log", "无预缓存 Skills，跳过预装（可在配置页面手动安装）");
}

/// 递归复制目录内容，返回复制的文件数（跳过符号链接，避免循环）
fn copy_dir_contents(src: &Path, dest: &Path) -> Result<usize> {
    let mut count = 0;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        if ft.is_symlink() {
            continue; // 跳过符号链接，避免循环引用
        }
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if ft.is_dir() {
            std::fs::create_dir_all(&dest_path)?;
            count += copy_dir_contents(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path)?;
            count += 1;
        }
    }
    Ok(count)
}

const CHINA_REGISTRY: &str = "https://registry.npmmirror.com";

/// 确保 pnpm 可用：先检测系统 pnpm，否则通过 corepack 或 npm 全局安装
fn ensure_pnpm(node_bin: &Path, window: &Window) -> Option<PathBuf> {
    // 1. 系统已有 pnpm
    if let Ok(out) = std::process::Command::new("pnpm").arg("--version").output() {
        if out.status.success() {
            let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let _ = window.emit("deploy:log", format!("检测到系统 pnpm {}", ver));
            return Some(PathBuf::from("pnpm"));
        }
    }

    // 2. 通过 corepack 启用 pnpm（Node.js 16.9+ 自带 corepack）
    if let Some(parent) = node_bin.parent() {
        let corepack = parent.join("corepack");
        if corepack.exists() {
            let _ = window.emit("deploy:log", "通过 corepack 启用 pnpm…");
            if let Ok(out) = std::process::Command::new(&corepack).args(["enable", "pnpm"]).output() {
                if out.status.success() {
                    let _ = window.emit("deploy:log", "corepack enable pnpm 成功");
                    return Some(PathBuf::from("pnpm"));
                }
            }
        }
    }

    // 3. 通过 npm 全局安装 pnpm
    if let Ok(npm_cli) = find_npm_cli(&node_bin.to_path_buf()) {
        let _ = window.emit("deploy:log", "通过 npm 安装 pnpm…");
        let result = std::process::Command::new(node_bin)
            .args([npm_cli.to_str().unwrap_or_default(),
                   "install", "-g", "pnpm",
                   &format!("--registry={}", CHINA_REGISTRY)])
            .output();
        if let Ok(out) = result {
            if out.status.success() {
                let _ = window.emit("deploy:log", "pnpm 安装成功");
                return Some(PathBuf::from("pnpm"));
            }
        }
    }

    let _ = window.emit("deploy:log", "pnpm 不可用，将使用 npm 回落");
    None
}

/// 使用 pnpm（优先）或 npm 安装 openclaw 的生产依赖
fn install_npm_dependencies(config: &DeployConfig, pkg_dir: &PathBuf, window: &Window) -> Result<()> {
    let node_bin = node_bin_path(&config.install_path);
    let pnpm = ensure_pnpm(&node_bin, window);

    let proxy_env: Vec<(&str, &str)> = match &config.source_mode {
        SourceMode::Online { proxy_url: Some(url) } =>
            vec![("HTTP_PROXY", url.as_str()), ("HTTPS_PROXY", url.as_str())],
        _ => vec![],
    };

    let (output, pkg_mgr_name) = if let Some(pnpm_path) = &pnpm {
        let mut cmd = std::process::Command::new(pnpm_path);
        cmd.args(["install", "--prod", &format!("--registry={}", CHINA_REGISTRY)])
            .current_dir(pkg_dir)
            .env("NODE_ENV", "production");
        for (k, v) in &proxy_env { cmd.env(k, v); }
        (cmd.output(), "pnpm")
    } else {
        let npm_cli = find_npm_cli(&node_bin)?;
        let _ = window.emit("deploy:log", format!("npm cli: {}", npm_cli.display()));
        let mut cmd = std::process::Command::new(&node_bin);
        cmd.arg(&npm_cli)
            .args(["install", "--omit=dev", "--no-audit", "--no-fund",
                   "--no-package-lock",
                   &format!("--registry={}", CHINA_REGISTRY)])
            .current_dir(pkg_dir)
            .env("NODE_ENV", "production")
            .env("npm_config_userconfig", pkg_dir.join(".npmrc_empty").to_str().unwrap_or(""))
            .env("npm_config_cache", PathBuf::from(&config.install_path).join(".npm_cache").to_str().unwrap_or(""));
        for (k, v) in &proxy_env { cmd.env(k, v); }
        (cmd.output(), "npm")
    };

    let output = output
        .map_err(|e| anyhow::anyhow!("执行 {} install 失败: {}", pkg_mgr_name, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = window.emit("deploy:log", format!("{} install 错误: {}", pkg_mgr_name, stderr));
        anyhow::bail!("{} install 失败（退出码 {}）:\n{}",
            pkg_mgr_name, output.status.code().unwrap_or(-1), stderr);
    }

    let _ = window.emit("deploy:log", format!("{} 依赖安装完成", pkg_mgr_name));
    Ok(())
}

/// 为所有自带插件安装 npm 依赖（pnpm 优先，回落到 npm）
fn install_plugin_dependencies(config: &DeployConfig, window: &Window) {
    let plugins_dir = PathBuf::from(&config.install_path)
        .join("openclaw_pkg/package/plugins");
    if !plugins_dir.exists() { return; }

    let node_bin = node_bin_path(&config.install_path);
    if !node_bin.exists() { return; }

    // 确定包管理器（复用 ensure_pnpm 的结果）
    let pnpm = ensure_pnpm(&node_bin, window);

    let entries: Vec<_> = std::fs::read_dir(&plugins_dir)
        .into_iter().flatten().flatten()
        .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .collect();

    let total = entries.len();
    let mut installed = 0u32;
    let mut skipped = 0u32;

    for entry in &entries {
        let dir = entry.path();
        if !dir.join("package.json").exists() || dir.join("node_modules").exists() {
            skipped += 1;
            continue;
        }
        // 检查是否有 dependencies
        let has_deps = std::fs::read_to_string(dir.join("package.json")).ok()
            .and_then(|d| serde_json::from_str::<serde_json::Value>(&d).ok())
            .map(|v| v.get("dependencies").and_then(|d| d.as_object()).map(|o| !o.is_empty()).unwrap_or(false))
            .unwrap_or(false);
        if !has_deps { skipped += 1; continue; }

        let name = entry.file_name().to_string_lossy().to_string();
        let _ = window.emit("deploy:log", format!("安装插件依赖: {}…", name));

        let output = if let Some(pnpm_path) = &pnpm {
            let mut cmd = std::process::Command::new(pnpm_path);
            cmd.args(["install", "--prod", &format!("--registry={}", CHINA_REGISTRY)])
                .current_dir(&dir)
                .env("NODE_ENV", "production");
            if let SourceMode::Online { proxy_url: Some(url) } = &config.source_mode {
                cmd.env("HTTP_PROXY", url).env("HTTPS_PROXY", url);
            }
            cmd.output()
        } else if let Ok(npm_cli) = find_npm_cli(&node_bin) {
            let mut cmd = std::process::Command::new(&node_bin);
            cmd.arg(&npm_cli)
                .args(["install", "--omit=dev", "--no-audit", "--no-fund",
                       "--no-package-lock", &format!("--registry={}", CHINA_REGISTRY)])
                .current_dir(&dir)
                .env("NODE_ENV", "production");
            if let SourceMode::Online { proxy_url: Some(url) } = &config.source_mode {
                cmd.env("HTTP_PROXY", url).env("HTTPS_PROXY", url);
            }
            cmd.output()
        } else {
            continue;
        };

        match output {
            Ok(out) if out.status.success() => { installed += 1; }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                let _ = window.emit("deploy:log", format!("  {} 依赖安装失败（非致命）: {}",
                    name, stderr.lines().take(3).collect::<Vec<_>>().join(" ")));
            }
            Err(e) => {
                let _ = window.emit("deploy:log", format!("  {} 依赖安装失败（非致命）: {}", name, e));
            }
        }
    }

    let _ = window.emit("deploy:log",
        format!("插件依赖: {} 安装 / {} 跳过 / {} 总计", installed, skipped, total));
}

/// 在部署的 node 中定位自带的 npm-cli.js
fn find_npm_cli(node_bin: &PathBuf) -> Result<PathBuf> {
    let node_dir = node_bin.parent().unwrap_or(node_bin.as_path());

    // Linux/macOS: node 旁边的 ../lib/node_modules/npm/bin/npm-cli.js
    // Windows: 可能在 node_modules/npm/bin/npm-cli.js
    let candidates = [
        node_dir.join("../lib/node_modules/npm/bin/npm-cli.js"),
        node_dir.join("node_modules/npm/bin/npm-cli.js"),
    ];

    for c in &candidates {
        if c.exists() {
            return Ok(c.canonicalize()?);
        }
    }

    // 回退：让 node 自己找
    let output = std::process::Command::new(node_bin)
        .args(["-e", "console.log(require.resolve('npm/bin/npm-cli.js'))"])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let cli = PathBuf::from(&path);
            if cli.exists() {
                return Ok(cli);
            }
        }
    }

    anyhow::bail!("未找到 npm，Node.js 安装可能不完整")
}

fn write_main_config(config: &DeployConfig) -> Result<()> {
    let config_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw");
    write_main_config_to(config_dir, config)
}

fn write_main_config_to(config_dir: PathBuf, config: &DeployConfig) -> Result<()> {
    use secrecy::ExposeSecret;
    std::fs::create_dir_all(&config_dir)?;

    // Gateway 配置（参考 https://docs.openclaw.ai/gateway/configuration-reference）
    let mut gateway = serde_json::json!({
        "port": config.service_port,
        "mode": "local",
        "auth": {
            "mode": "password",
            "password": config.admin_password.expose_secret()
        },
        "controlUi": { "enabled": true }
    });
    if let Some(domain) = &config.domain_name {
        if !domain.is_empty() {
            gateway["mode"] = serde_json::json!("remote");
            gateway["remote"] = serde_json::json!({
                "url": format!("https://{}", domain),
                "transport": "sse",
            });
        }
    }

    let mut cfg = serde_json::json!({ "gateway": gateway });

    // AI 模型配置：models.providers + agents.defaults.model.primary
    // 参考官方 onboard 生成格式：apiKey 直接放在 models.providers 内
    if let Some(ai) = &config.ai_config {
        if !ai.api_key.is_empty() {
            let model_id = if ai.model.contains('/') {
                ai.model.clone()
            } else {
                format!("{}/{}", ai.provider, ai.model)
            };
            // 从 model_id 提取 provider 前缀和裸模型名
            let (prov_key, bare_model) = model_id.split_once('/')
                .unwrap_or((&ai.provider, &ai.model));

            cfg["agents"] = serde_json::json!({
                "defaults": {
                    "model": { "primary": model_id }
                }
            });

            // models.providers：apiKey 直接内嵌、显式声明模型列表
            let base_url = if ai.base_url.is_empty() { None } else { Some(&ai.base_url) };
            let mut provider = serde_json::json!({
                "apiKey": ai.api_key,
                "api": "openai-completions",
                "models": [{
                    "id": bare_model,
                    "name": bare_model,
                }]
            });
            if let Some(url) = base_url {
                provider["baseUrl"] = serde_json::json!(url);
            }
            cfg["models"] = serde_json::json!({
                "mode": "merge",
                "providers": { prov_key: provider }
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
    let node_bin = node_bin_path(&config.install_path);
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
            .args([&script.to_string_lossy(), "gateway",
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
            .args([&script.to_string_lossy(), "gateway",
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
            .args([&script.to_string_lossy(), "gateway",
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

/// 下载完整 Node.js 发行版并解压到 node_dir（包含 bin/node + lib/node_modules/npm/）
async fn download_node_full(node_dir: &PathBuf, proxy: Option<&str>, window: &Window) -> Result<()> {
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
    { let _ = (node_dir, proxy, window); anyhow::bail!("当前平台不支持在线下载 Node.js"); }

    let archive_tmp = node_dir.join("node_archive.tmp");
    download_with_progress(&url, &archive_tmp, proxy, "Node.js", window).await?;

    let _ = window.emit("deploy:log", "解压 Node.js 完整发行版（含 npm）…");
    extract_node_full(&archive_tmp, node_dir)?;
    std::fs::remove_file(&archive_tmp).ok();
    Ok(())
}

/// 解压完整 Node.js 发行版到 node_dir，strip 顶层目录
fn extract_node_full(archive: &PathBuf, node_dir: &PathBuf) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let data = std::fs::read(archive)?;
        let xz = xz2::read::XzDecoder::new(std::io::Cursor::new(data));
        let mut tar = tar::Archive::new(xz);
        for entry in tar.entries()? {
            let mut e = entry?;
            let path = e.path()?.into_owned();
            // strip 顶层 "node-v22.17.0-linux-x64/" 前缀
            let stripped = path.components().skip(1).collect::<PathBuf>();
            if stripped.as_os_str().is_empty() { continue; }
            let dest = node_dir.join(&stripped);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            e.unpack(&dest)?;
        }
        // 确保 node 可执行
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let node_bin = node_dir.join("bin/node");
            if node_bin.exists() {
                std::fs::set_permissions(&node_bin, std::fs::Permissions::from_mode(0o755))?;
            }
        }
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        let data = std::fs::read(archive)?;
        let cursor = std::io::Cursor::new(data);
        let mut zip = zip::ZipArchive::new(cursor)?;
        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            let path = PathBuf::from(file.name());
            let stripped = path.components().skip(1).collect::<PathBuf>();
            if stripped.as_os_str().is_empty() { continue; }
            let dest = node_dir.join(&stripped);
            if file.is_dir() {
                std::fs::create_dir_all(&dest)?;
            } else {
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut out = std::fs::File::create(&dest)?;
                std::io::copy(&mut file, &mut out)?;
            }
        }
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        let file = std::fs::File::open(archive)?;
        let gz = flate2::read::GzDecoder::new(file);
        let mut tar = tar::Archive::new(gz);
        for entry in tar.entries()? {
            let mut e = entry?;
            let path = e.path()?.into_owned();
            let stripped = path.components().skip(1).collect::<PathBuf>();
            if stripped.as_os_str().is_empty() { continue; }
            let dest = node_dir.join(&stripped);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            e.unpack(&dest)?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let node_bin = node_dir.join("bin/node");
            if node_bin.exists() {
                std::fs::set_permissions(&node_bin, std::fs::Permissions::from_mode(0o755))?;
            }
        }
        return Ok(());
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    anyhow::bail!("当前平台不支持");
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
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3));
    if let Some(p) = proxy {
        if let Ok(px) = reqwest::Proxy::all(p) {
            builder = builder.proxy(px);
        }
    }
    let client = match builder.build() {
        Ok(c) => c,
        Err(_) => return false,
    };
    client.head("https://registry.npmmirror.com/openclaw")
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

    #[cfg(unix)]
    #[test]
    fn test_validate_install_path_rejects_root() {
        let err = validate_install_path("/").unwrap_err();
        assert!(err.to_string().contains("系统关键目录"));
    }

    #[test]
    fn test_validate_install_path_rejects_empty() {
        let err = validate_install_path("").unwrap_err();
        assert!(err.to_string().contains("不能为空"));
    }

    #[test]
    fn test_validate_install_path_rejects_invalid_chars() {
        let err = validate_install_path("/tmp/test<dir>").unwrap_err();
        assert!(err.to_string().contains("非法字符"));
    }

    #[cfg(unix)]
    #[test]
    fn test_validate_install_path_accepts_valid() {
        // /tmp 存在且可写，空间充足
        assert!(validate_install_path("/tmp/openclaw_test_install").is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn test_validate_install_path_rejects_dotdot_bypass() {
        // /usr/lib/../../etc 规范化后等于 /etc，应被黑名单拦截
        let err = validate_install_path("/usr/lib/../../etc").unwrap_err();
        assert!(err.to_string().contains("系统关键目录"));
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path(std::path::Path::new("/usr/lib/../../etc")), std::path::PathBuf::from("/etc"));
        assert_eq!(normalize_path(std::path::Path::new("/opt/./openclaw")), std::path::PathBuf::from("/opt/openclaw"));
    }

    #[cfg(unix)]
    #[test]
    fn test_validate_install_path_rejects_usr() {
        let err = validate_install_path("/usr").unwrap_err();
        assert!(err.to_string().contains("系统关键目录"));
    }

    /// 辅助：创建带 AI 配置的 DeployConfig
    fn make_config_with_ai(provider: &str, model: &str, api_key: &str, base_url: &str) -> DeployConfig {
        DeployConfig {
            install_path: "/tmp/oc_test".into(),
            service_port: 18789,
            admin_password: Secret::new("test123".into()),
            domain_name: None,
            install_service: false,
            start_on_boot: false,
            source_mode: SourceMode::Bundled,
            wecom_config: None,
            dingtalk_config: None,
            feishu_config: None,
            qq_config: None,
            ai_config: Some(AiConfigDto {
                provider: provider.into(),
                base_url: base_url.into(),
                api_key: api_key.into(),
                model: model.into(),
            }),
        }
    }

    #[test]
    fn test_write_main_config_models_providers_format() {
        let dir = tempfile::tempdir().unwrap();
        let config = make_config_with_ai("deepseek", "deepseek-chat", "sk-test123", "https://api.deepseek.com/v1");
        write_main_config_to(dir.path().to_path_buf(), &config).unwrap();

        let content = std::fs::read_to_string(dir.path().join("openclaw.json")).unwrap();
        let cfg: serde_json::Value = serde_json::from_str(&content).unwrap();

        // apiKey 应直接在 models.providers 中
        let provider = &cfg["models"]["providers"]["deepseek"];
        assert_eq!(provider["apiKey"], "sk-test123");
        assert_eq!(provider["baseUrl"], "https://api.deepseek.com/v1");
        assert_eq!(provider["api"], "openai-completions");
        assert_eq!(provider["models"][0]["id"], "deepseek-chat");
        assert_eq!(cfg["models"]["mode"], "merge");

        // agents.defaults.model 应为对象格式
        assert_eq!(cfg["agents"]["defaults"]["model"]["primary"], "deepseek/deepseek-chat");

        // 不应有 env 段或 auth.profiles
        assert!(cfg.get("env").is_none());
        assert!(cfg.get("auth").is_none());

        // gateway.controlUi.enabled 应为 true
        assert_eq!(cfg["gateway"]["controlUi"]["enabled"], true);
    }

    #[test]
    fn test_write_main_config_custom_base_url() {
        let dir = tempfile::tempdir().unwrap();
        let config = make_config_with_ai("custom", "gpt-4o", "sk-xxx", "https://my-proxy.com/v1");
        write_main_config_to(dir.path().to_path_buf(), &config).unwrap();

        let content = std::fs::read_to_string(dir.path().join("openclaw.json")).unwrap();
        let cfg: serde_json::Value = serde_json::from_str(&content).unwrap();

        // apiKey + baseUrl 都在 models.providers 中
        let provider = &cfg["models"]["providers"]["custom"];
        assert_eq!(provider["baseUrl"], "https://my-proxy.com/v1");
        assert_eq!(provider["apiKey"], "sk-xxx");
        assert_eq!(provider["api"], "openai-completions");
    }

    #[test]
    fn test_write_main_config_domain_name() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_config_with_ai("deepseek", "deepseek-chat", "sk-test", "");
        config.domain_name = Some("gw.example.com".into());
        write_main_config_to(dir.path().to_path_buf(), &config).unwrap();

        let content = std::fs::read_to_string(dir.path().join("openclaw.json")).unwrap();
        let cfg: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(cfg["gateway"]["mode"], "remote");
        assert_eq!(cfg["gateway"]["remote"]["url"], "https://gw.example.com");
        assert_eq!(cfg["gateway"]["remote"]["transport"], "sse");
    }

    #[test]
    fn test_write_main_config_no_ai() {
        let dir = tempfile::tempdir().unwrap();
        let config = DeployConfig {
            install_path: "/tmp/oc_test".into(),
            service_port: 9090,
            admin_password: Secret::new("pw".into()),
            domain_name: None,
            install_service: false,
            start_on_boot: false,
            source_mode: SourceMode::Bundled,
            wecom_config: None,
            dingtalk_config: None,
            feishu_config: None,
            qq_config: None,
            ai_config: None,
        };
        write_main_config_to(dir.path().to_path_buf(), &config).unwrap();

        let content = std::fs::read_to_string(dir.path().join("openclaw.json")).unwrap();
        let cfg: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert!(cfg.get("auth").is_none());
        assert!(cfg.get("agents").is_none());
        assert_eq!(cfg["gateway"]["port"], 9090);
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
