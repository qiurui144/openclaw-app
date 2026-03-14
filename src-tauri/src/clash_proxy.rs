use anyhow::Result;
use serde::Serialize;
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

/// Mihomo 二进制路径
fn mihomo_bin_path() -> PathBuf {
    let clash_dir = std::env::temp_dir().join("openclaw_clash");
    #[cfg(target_os = "windows")]
    return clash_dir.join("mihomo.exe");
    #[cfg(not(target_os = "windows"))]
    return clash_dir.join("mihomo");
}

/// 确保 mihomo 二进制可用（bundled 模式从内嵌资源提取）
fn ensure_mihomo_binary() -> Result<PathBuf> {
    let bin = mihomo_bin_path();
    if bin.exists() {
        return Ok(bin);
    }

    if let Some(p) = bin.parent() {
        std::fs::create_dir_all(p)?;
    }

    #[cfg(feature = "bundled")]
    {
        #[cfg(target_os = "linux")]
        let data = include_bytes!("../../resources/binaries/linux/mihomo");
        #[cfg(target_os = "windows")]
        let data = include_bytes!("..\\..\\resources\\binaries\\windows\\mihomo.exe");

        std::fs::write(&bin, &data[..])?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&bin,
                std::fs::Permissions::from_mode(0o755))?;
        }
        return Ok(bin);
    }

    #[cfg(not(feature = "bundled"))]
    anyhow::bail!("Mihomo 二进制不存在: {}。请使用 Full Bundle 版本或手动放置。", bin.display());
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

#[allow(dead_code)]
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

    // 3. 确保 Mihomo 二进制可用（bundled 模式自动提取）
    let bin = ensure_mihomo_binary()?;

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
