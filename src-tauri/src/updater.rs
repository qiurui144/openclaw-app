use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
    pub release_notes: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    body: Option<String>,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

/// 比较版本号，返回 server > local
pub fn is_newer(server: &str, local: &str) -> bool {
    fn strip(s: &str) -> &str {
        s.trim_start_matches('v')
    }
    match (
        semver::Version::parse(strip(server)),
        semver::Version::parse(strip(local)),
    ) {
        (Ok(sv), Ok(lv)) => sv > lv,
        _ => false,
    }
}

pub async fn check_update(proxy_url: Option<&str>) -> Result<Option<UpdateInfo>> {
    let current = env!("CARGO_PKG_VERSION");
    let url = "https://api.github.com/repos/openclaw/openclaw/releases/latest";

    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("openclaw-wizard");
    if let Some(proxy) = proxy_url {
        builder = builder.proxy(reqwest::Proxy::all(proxy)?);
    }
    let client = builder.build()?;
    let release: GithubRelease = client.get(url).send().await?.json().await?;

    if !is_newer(&release.tag_name, current) {
        return Ok(None);
    }

    let asset = release.assets.iter()
        .find(|a| a.name == "openclaw.tgz")
        .ok_or_else(|| anyhow::anyhow!("Release 中未找到 openclaw.tgz"))?;

    Ok(Some(UpdateInfo {
        version: release.tag_name,
        download_url: asset.browser_download_url.clone(),
        release_notes: release.body.unwrap_or_default(),
        sha256: None,
    }))
}

pub async fn apply_update(
    install_path: &str,
    download_url: &str,
    sha256: Option<&str>,
    proxy_url: Option<&str>,
    window: &tauri::Window,
) -> Result<()> {
    use tauri::Emitter;

    // 1. 停止服务
    let _ = window.emit("update:progress", "正在停止服务…");
    stop_service()?;

    // 2. 下载 ZIP
    let _ = window.emit("update:progress", "正在下载新版本…");
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300));
    if let Some(proxy) = proxy_url {
        builder = builder.proxy(reqwest::Proxy::all(proxy)?);
    }
    let client = builder.build()?;
    let bytes = client.get(download_url).send().await?.bytes().await?;

    // 3. SHA256 校验
    if let Some(expected) = sha256 {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let actual = hex::encode(hasher.finalize());
        anyhow::ensure!(actual == expected,
            "SHA256 校验失败：期望 {}, 实际 {}", expected, actual);
    }

    // 4. 备份旧版本
    let _ = window.emit("update:progress", "正在备份旧版本…");
    let pkg_dir = PathBuf::from(install_path).join("openclaw_pkg");
    let backup_dir = PathBuf::from(install_path).join("openclaw_pkg.bak");
    if pkg_dir.exists() {
        if backup_dir.exists() {
            std::fs::remove_dir_all(&backup_dir)?;
        }
        std::fs::rename(&pkg_dir, &backup_dir)?;
    }

    // 5. 解压新版本
    let _ = window.emit("update:progress", "正在安装新版本…");
    let tmp_dir = PathBuf::from(install_path).join(".tmp").join("update");
    std::fs::create_dir_all(&tmp_dir)?;

    let gz = flate2::read::GzDecoder::new(std::io::Cursor::new(&bytes));
    let mut archive = tar::Archive::new(gz);
    archive.unpack(&tmp_dir)?;

    std::fs::rename(&tmp_dir, &pkg_dir)?;

    // 6. 重启服务
    let _ = window.emit("update:progress", "正在重启服务…");
    match start_service() {
        Ok(_) => {
            // S6 fix: 从 deploy_meta.json 读取端口
            let port: u16 = {
                let meta_path = dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".openclaw/deploy_meta.json");
                let meta: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&meta_path)?)?;
                meta["service_port"].as_u64().unwrap_or(18789) as u16
            };
            if crate::deploy::health_check(port).await.is_ok() {
                std::fs::remove_dir_all(&backup_dir).ok();
            } else {
                // 健康检查失败，回滚到旧版本
                let _ = window.emit("update:progress", "健康检查失败，正在回滚…");
                std::fs::remove_dir_all(&pkg_dir).ok();
                if backup_dir.exists() {
                    std::fs::rename(&backup_dir, &pkg_dir)?;
                }
                let _ = start_service();
                anyhow::bail!("更新后健康检查失败，已回滚到旧版本");
            }
        }
        Err(e) => {
            let _ = window.emit("update:progress", "启动失败，正在回滚…");
            std::fs::remove_dir_all(&pkg_dir).ok();
            if backup_dir.exists() {
                std::fs::rename(&backup_dir, &pkg_dir)?;
            }
            // S7 fix: 回滚启动不 propagate 错误
            let _ = start_service();
            anyhow::bail!("更新失败，已回滚到旧版本: {}", e);
        }
    }

    let _ = window.emit("update:done", ());
    Ok(())
}

fn stop_service() -> Result<()> {
    #[cfg(target_os = "linux")]
    std::process::Command::new("systemctl")
        .args(["--user", "stop", "openclaw.service"])
        .status()?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("schtasks")
        .args(["/End", "/TN", "OpenClaw Gateway"])
        .status()?;
    Ok(())
}

fn start_service() -> Result<()> {
    #[cfg(target_os = "linux")]
    std::process::Command::new("systemctl")
        .args(["--user", "start", "openclaw.service"])
        .status()?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("schtasks")
        .args(["/Run", "/TN", "OpenClaw Gateway"])
        .status()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_basic() {
        assert!(is_newer("1.1.0", "1.0.0"));
        assert!(is_newer("v1.1.0", "1.0.0"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("0.9.9", "1.0.0"));
    }

    #[test]
    fn test_is_newer_patch() {
        assert!(is_newer("1.0.1", "1.0.0"));
        assert!(!is_newer("1.0.0", "1.0.1"));
    }

    #[test]
    fn test_update_info_serializable() {
        let info = UpdateInfo {
            version: "1.1.0".into(),
            download_url: "https://example.com/openclaw.tgz".into(),
            release_notes: "Bug fixes".into(),
            sha256: Some("abc123".into()),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("1.1.0"));
    }
}
