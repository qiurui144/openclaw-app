/// 服务控制模块：供托盘图标调用，独立于部署流程
use anyhow::Result;
use std::path::PathBuf;

// ── 数据结构 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum ServiceStatus {
    Running,
    Stopped,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DeployMeta {
    pub install_path: String,
    pub version: String,
    pub service_port: u16,
}

// ── meta 文件读取 ─────────────────────────────────────────────────────────────

pub fn read_meta() -> Option<DeployMeta> {
    let path = dirs::home_dir()?.join(".openclaw/deploy_meta.json");
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

// ── 状态检测（单次探测，无重试，供托盘轮询用）────────────────────────────────

pub async fn check_status(port: u16) -> ServiceStatus {
    let connected = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port)),
    )
    .await
    .map(|r| r.is_ok())
    .unwrap_or(false);

    if connected {
        ServiceStatus::Running
    } else {
        ServiceStatus::Stopped
    }
}

// ── 启动服务 ──────────────────────────────────────────────────────────────────

pub fn start(meta: &DeployMeta) -> Result<()> {
    let node_bin = crate::deploy::node_bin_path(&meta.install_path);
    let script = PathBuf::from(&meta.install_path)
        .join("openclaw_pkg")
        .join("package")
        .join("openclaw.mjs");

    #[cfg(target_os = "linux")]
    {
        let elevated = crate::system_check::is_elevated();
        let unit_path = if elevated {
            PathBuf::from("/etc/systemd/system/openclaw.service")
        } else {
            dirs::home_dir()
                .unwrap_or_default()
                .join(".config/systemd/user/openclaw.service")
        };

        if unit_path.exists() {
            let ok = if elevated {
                std::process::Command::new("systemctl")
                    .args(["start", "openclaw.service"])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            } else {
                std::process::Command::new("timeout")
                    .args(["10", "systemctl", "--user", "start", "openclaw.service"])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            };
            if ok {
                return Ok(());
            }
        }
        spawn_node(&node_bin, &script, meta.service_port)?;
    }

    #[cfg(target_os = "macos")]
    {
        let elevated = crate::system_check::is_elevated();
        let plist_path = if elevated {
            PathBuf::from("/Library/LaunchDaemons/com.openclaw.gateway.plist")
        } else {
            dirs::home_dir().unwrap_or_default()
                .join("Library/LaunchAgents/com.openclaw.gateway.plist")
        };

        if plist_path.exists() {
            let ok = std::process::Command::new("launchctl")
                .args(["load", "-w", plist_path.to_str().unwrap_or("")])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if ok {
                return Ok(());
            }
        }
        spawn_node(&node_bin, &script, meta.service_port)?;
    }

    #[cfg(target_os = "windows")]
    {
        let ran = std::process::Command::new("schtasks")
            .args(["/Run", "/TN", "OpenClaw Gateway"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ran {
            spawn_node(&node_bin, &script, meta.service_port)?;
        }
    }

    Ok(())
}

// ── 停止服务 ──────────────────────────────────────────────────────────────────

pub fn stop() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let elevated = crate::system_check::is_elevated();
        if elevated {
            let _ = std::process::Command::new("systemctl")
                .args(["stop", "openclaw.service"])
                .status();
        } else {
            let _ = std::process::Command::new("timeout")
                .args(["10", "systemctl", "--user", "stop", "openclaw.service"])
                .status();
        }
        let _ = std::process::Command::new("pkill")
            .args(["-f", "openclaw.mjs"])
            .status();
    }

    #[cfg(target_os = "macos")]
    {
        let elevated = crate::system_check::is_elevated();
        let plist_path = if elevated {
            PathBuf::from("/Library/LaunchDaemons/com.openclaw.gateway.plist")
        } else {
            dirs::home_dir().unwrap_or_default()
                .join("Library/LaunchAgents/com.openclaw.gateway.plist")
        };
        if plist_path.exists() {
            let _ = std::process::Command::new("launchctl")
                .args(["unload", "-w", plist_path.to_str().unwrap_or("")])
                .status();
        }
        let _ = std::process::Command::new("pkill")
            .args(["-f", "openclaw.mjs"])
            .status();
    }

    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("schtasks")
            .args(["/End", "/TN", "OpenClaw Gateway"])
            .status();
        let _ = std::process::Command::new("wmic")
            .args([
                "process",
                "where",
                "commandline like '%openclaw.mjs%'",
                "call",
                "terminate",
            ])
            .status();
    }

    Ok(())
}

// ── 辅助：直接 spawn node 进程（各平台通用回退）─────────────────────────────

fn spawn_node(node_bin: &PathBuf, script: &PathBuf, port: u16) -> Result<()> {
    std::process::Command::new(node_bin)
        .args([
            script.to_str().unwrap_or_default(),
            "gateway",
            "--port",
            &port.to_string(),
        ])
        .env("NODE_ENV", "production")
        .spawn()
        .map_err(|e| anyhow::anyhow!("启动失败: {e}"))?;
    Ok(())
}
