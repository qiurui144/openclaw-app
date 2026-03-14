use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::process::Child;

static CLASH_PROCESS: OnceLock<Mutex<Option<Child>>> = OnceLock::new();

fn process_lock() -> &'static Mutex<Option<Child>> {
    CLASH_PROCESS.get_or_init(|| Mutex::new(None))
}

#[derive(Debug, Serialize)]
pub struct ClashTestResult {
    pub success: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

/// Mihomo 二进制路径（内置资源）
fn mihomo_bin_path() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));

    #[cfg(target_os = "linux")]
    return exe_dir.join("clash").join("mihomo-linux-amd64");
    #[cfg(target_os = "windows")]
    return exe_dir.join("clash").join("mihomo-windows-amd64.exe");
    #[cfg(target_os = "macos")]
    {
        let arch = match std::env::consts::ARCH {
            "x86_64" => "amd64",
            _ => "arm64",
        };
        return exe_dir.join("clash").join(format!("mihomo-darwin-{}", arch));
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    return exe_dir.join("clash").join("mihomo");
}

/// 保存订阅 URL 供 Skills 更新复用
fn save_subscription_url(url: &str) -> Result<()> {
    let path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw")
        .join("proxy.json");
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::write(&path, serde_json::json!({
        "subscription_url": url
    }).to_string())?;
    Ok(())
}

pub fn load_subscription_url() -> Option<String> {
    let path = dirs::home_dir()?
        .join(".openclaw")
        .join("proxy.json");
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<serde_json::Value>(&data).ok()?
        ["subscription_url"].as_str().map(String::from)
}

/// 启动 Mihomo，返回代理地址 "http://127.0.0.1:7890"
pub async fn start(subscription_url: &str) -> Result<String> {
    // 1. 下载订阅配置
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let config_data = client.get(subscription_url).send().await?.text().await?;

    // 2. 写临时配置文件
    let config_dir = std::env::temp_dir().join("openclaw_clash");
    std::fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.yaml");
    std::fs::write(&config_path, &config_data)?;

    // 3. 启动 Mihomo 进程
    let bin = mihomo_bin_path();
    anyhow::ensure!(bin.exists(),
        "Mihomo 二进制不存在: {}", bin.display());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin,
            std::fs::Permissions::from_mode(0o755))?;
    }

    let child = std::process::Command::new(&bin)
        .args(["-d", config_dir.to_str().unwrap()])
        .spawn()?;

    *process_lock().lock().unwrap() = Some(child);

    // 4. 等待 Mihomo 启动（最多 5s）
    let proxy_addr = "http://127.0.0.1:7890".to_string();
    for _ in 0..10 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if test_proxy(&proxy_addr).await.success { break; }
    }

    save_subscription_url(subscription_url)?;
    Ok(proxy_addr)
}

/// 停止 Mihomo 进程并删除二进制
pub fn stop() -> Result<()> {
    if let Ok(mut guard) = process_lock().lock() {
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    // 清理临时配置
    let config_dir = std::env::temp_dir().join("openclaw_clash");
    std::fs::remove_dir_all(&config_dir).ok();
    Ok(())
}

/// 测试代理延迟（连接 https://github.com）
pub async fn test_proxy(proxy_url: &str) -> ClashTestResult {
    let start = std::time::Instant::now();
    let proxy = match reqwest::Proxy::all(proxy_url) {
        Ok(p) => p,
        Err(e) => return ClashTestResult {
            success: false,
            latency_ms: None,
            error: Some(e.to_string()),
        },
    };
    let client = match reqwest::Client::builder()
        .proxy(proxy)
        .timeout(std::time::Duration::from_secs(8))
        .build()
    {
        Ok(c) => c,
        Err(e) => return ClashTestResult {
            success: false, latency_ms: None, error: Some(e.to_string()),
        },
    };

    match client.head("https://github.com").send().await {
        Ok(_) => ClashTestResult {
            success: true,
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: None,
        },
        Err(e) => ClashTestResult {
            success: false,
            latency_ms: None,
            error: Some(e.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clash_test_result_serializable() {
        let r = ClashTestResult {
            success: true,
            latency_ms: Some(100),
            error: None,
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("latency_ms"));
    }

    #[test]
    fn test_save_and_load_subscription_url() {
        let tmp = std::env::temp_dir()
            .join(format!("oc_clash_test_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        #[cfg(unix)]
        std::env::set_var("HOME", &tmp);

        save_subscription_url("https://example.com/sub").unwrap();
        let loaded = load_subscription_url();
        assert_eq!(loaded.as_deref(), Some("https://example.com/sub"));

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_stop_is_idempotent() {
        let result = stop();
        assert!(result.is_ok());
    }
}
