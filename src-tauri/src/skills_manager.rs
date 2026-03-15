use anyhow::Result;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize, Clone)]
pub struct SkillInfo {
    pub name: String,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
}

/// 从 node_modules 读取已安装的 @openclaw/* Skills
pub fn list_installed(install_path: &str) -> Vec<SkillInfo> {
    let modules = PathBuf::from(install_path)
        .join("openclaw_pkg")
        .join("package")
        .join("node_modules");

    let scope_dir = modules.join("@openclaw");
    if !scope_dir.exists() { return vec![]; }

    std::fs::read_dir(&scope_dir)
        .ok()
        .map(|entries| {
            entries.filter_map(|e| {
                let entry = e.ok()?;
                let pkg_json = entry.path().join("package.json");
                let data = std::fs::read_to_string(&pkg_json).ok()?;
                let v: serde_json::Value = serde_json::from_str(&data).ok()?;
                let name = v["name"].as_str()?.to_string();
                let version = v["version"].as_str()?.to_string();
                Some(SkillInfo {
                    name,
                    current_version: version,
                    latest_version: None,
                    update_available: false,
                })
            })
            .collect()
        })
        .unwrap_or_default()
}

/// 查询 npmmirror 获取最新版本
pub async fn fetch_latest_version(
    skill_name: &str,
    proxy_url: Option<&str>
) -> Result<String> {
    let url = format!(
        "https://registry.npmmirror.com/{}/latest",
        skill_name
    );
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10));
    if let Some(proxy) = proxy_url {
        builder = builder.proxy(reqwest::Proxy::all(proxy)?);
    }
    let client = builder.build()?;
    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;
    resp["version"].as_str()
        .map(String::from)
        .ok_or_else(|| anyhow::anyhow!("无法解析版本"))
}

/// 下载 .tgz 并原子替换 node_modules 中的目录
pub async fn update_skill(
    install_path: &str,
    skill_name: &str,
    version: &str,
    proxy_url: Option<&str>,
) -> Result<()> {
    let short_name = skill_name.trim_start_matches("@openclaw/");
    let url = format!(
        "https://registry.npmmirror.com/@openclaw/{0}/-/{0}-{1}.tgz",
        short_name, version
    );

    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120));
    if let Some(proxy) = proxy_url {
        builder = builder.proxy(reqwest::Proxy::all(proxy)?);
    }
    let client = builder.build()?;
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("下载 {} v{} 失败，HTTP {}", skill_name, version, resp.status());
    }
    let bytes = resp.bytes().await?;

    let modules = PathBuf::from(install_path)
        .join("openclaw_pkg")
        .join("package")
        .join("node_modules");
    let tmp_dir = PathBuf::from(install_path)
        .join(".tmp")
        .join("skills")
        .join(format!("{}-{}", short_name, version));
    std::fs::create_dir_all(&tmp_dir)?;

    let gz = flate2::read::GzDecoder::new(std::io::Cursor::new(&bytes));
    let mut archive = tar::Archive::new(gz);
    archive.unpack(&tmp_dir)?;

    let extracted = tmp_dir.join("package");
    let target = modules.join("@openclaw").join(short_name);
    let backup = modules.join("@openclaw")
        .join(format!("{}.bak", short_name));

    if target.exists() {
        std::fs::rename(&target, &backup)?;
    }
    std::fs::rename(&extracted, &target)?;
    if backup.exists() {
        std::fs::remove_dir_all(&backup)?;
    }

    std::fs::remove_dir_all(&tmp_dir).ok();
    Ok(())
}

/// Windows：通过 HTTP 管理接口热重载（回退到重启服务）
#[cfg(target_os = "windows")]
pub async fn reload_skills_windows(port: u16, admin_password: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    let result = client
        .post(format!("http://127.0.0.1:{}/admin/reload-skills", port))
        .header("Authorization", format!("Bearer {}", admin_password))
        .send().await;

    if result.is_err() {
        std::process::Command::new("schtasks")
            .args(["/End", "/TN", "OpenClaw Gateway"])
            .status()?;
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        std::process::Command::new("schtasks")
            .args(["/Run", "/TN", "OpenClaw Gateway"])
            .status()?;
    }
    Ok(())
}

/// Linux/macOS：发送 SIGHUP
#[cfg(unix)]
pub fn send_reload_signal(install_path: &str) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let elevated = crate::system_check::is_elevated();
        if elevated {
            std::process::Command::new("systemctl")
                .args(["kill", "-s", "HUP", "openclaw.service"])
                .status()?;
        } else {
            std::process::Command::new("systemctl")
                .args(["--user", "kill", "-s", "HUP", "openclaw.service"])
                .status()?;
        }
    }
    #[cfg(target_os = "macos")]
    {
        let label = "com.openclaw.gateway";
        let result = std::process::Command::new("launchctl")
            .args(["kickstart", "-k", &format!("gui/{}/{}", unsafe { libc::getuid() }, label)])
            .status();
        if result.is_err() {
            let pid_path = PathBuf::from(install_path).join("openclaw.pid");
            if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
                if let Ok(pid) = pid_str.trim().parse::<i32>() {
                    unsafe { libc::kill(pid, libc::SIGHUP); }
                }
            }
        }
    }
    let _ = install_path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_installed_returns_empty_for_missing_dir() {
        let result = list_installed("/nonexistent/path");
        assert!(result.is_empty());
    }

    #[test]
    fn test_skill_info_serializable() {
        let info = SkillInfo {
            name: "@openclaw/feishu".into(),
            current_version: "1.0.0".into(),
            latest_version: Some("1.1.0".into()),
            update_available: true,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("update_available"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_list_installed_reads_package_json() {
        let tmp = std::env::temp_dir()
            .join(format!("oc_skills_test_{}", std::process::id()));
        let skill_dir = tmp
            .join("openclaw_pkg")
            .join("package")
            .join("node_modules")
            .join("@openclaw")
            .join("feishu");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("package.json"),
            r#"{"name":"@openclaw/feishu","version":"1.2.3"}"#
        ).unwrap();

        let skills = list_installed(tmp.to_str().unwrap());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "@openclaw/feishu");
        assert_eq!(skills[0].current_version, "1.2.3");

        std::fs::remove_dir_all(&tmp).ok();
    }
}
