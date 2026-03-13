use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    WeWork,
    QqWork,
    DingTalk,
    Feishu,
}

impl Platform {
    pub fn channel_key(&self) -> &'static str {
        match self {
            Platform::WeWork => "wecom",
            Platform::QqWork => "qqwork",
            Platform::DingTalk => "dingtalk",
            Platform::Feishu => "feishu",
        }
    }

    pub fn doc_url(&self) -> &'static str {
        match self {
            Platform::WeWork =>
                "https://work.weixin.qq.com/api/doc/90000/90136/91770",
            Platform::QqWork =>
                "https://work.qq.com/",
            Platform::DingTalk =>
                "https://open.dingtalk.com/document/robots/custom-robot-access",
            Platform::Feishu =>
                "https://open.feishu.cn/document/client-docs/bot-v3/add-custom-bot",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlatformEntry {
    pub platform: Platform,
    pub webhook_url: String,
}

pub fn write_platform_config(
    install_path: &str,
    platforms: &[PlatformEntry],
) -> Result<()> {
    if platforms.is_empty() { return Ok(()); }

    let config_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw")
        .join("openclaw.json");

    let mut config: serde_json::Value = if config_path.exists() {
        let data = std::fs::read_to_string(&config_path)?;
        serde_json::from_str(&data)?
    } else {
        serde_json::json!({})
    };

    let channels = config["channels"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let mut channels_map = channels;

    for entry in platforms {
        channels_map.insert(
            entry.platform.channel_key().to_string(),
            serde_json::json!({ "webhookUrl": entry.webhook_url }),
        );
    }

    config["channels"] = serde_json::Value::Object(channels_map);
    let _ = install_path;

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    static HOME_LOCK: Mutex<()> = Mutex::new(());

    fn with_temp_home<F: FnOnce()>(f: F) {
        let _guard = HOME_LOCK.lock().unwrap();
        let tmp = env::temp_dir()
            .join(format!("oc_pf_test_{}_{:?}", std::process::id(), std::thread::current().id()));
        std::fs::create_dir_all(&tmp).unwrap();
        #[cfg(unix)]
        env::set_var("HOME", &tmp);
        f();
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_platform_channel_key() {
        assert_eq!(Platform::Feishu.channel_key(), "feishu");
        assert_eq!(Platform::DingTalk.channel_key(), "dingtalk");
        assert_eq!(Platform::WeWork.channel_key(), "wecom");
    }

    #[test]
    fn test_doc_url_not_empty() {
        for p in [Platform::WeWork, Platform::QqWork,
                  Platform::DingTalk, Platform::Feishu] {
            assert!(!p.doc_url().is_empty());
            assert!(p.doc_url().starts_with("https://"));
        }
    }

    #[test]
    fn test_write_platform_config_creates_channels() {
        with_temp_home(|| {
            let dir = dirs::home_dir().unwrap().join(".openclaw");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("openclaw.json"),
                r#"{"gateway":{"port":18789}}"#
            ).unwrap();

            let entries = vec![
                PlatformEntry {
                    platform: Platform::Feishu,
                    webhook_url: "https://open.feishu.cn/hook/test".into(),
                },
            ];
            write_platform_config("/tmp/install", &entries).unwrap();

            let data = std::fs::read_to_string(dir.join("openclaw.json")).unwrap();
            let v: serde_json::Value = serde_json::from_str(&data).unwrap();
            assert_eq!(v["channels"]["feishu"]["webhookUrl"], "https://open.feishu.cn/hook/test");
            assert_eq!(v["gateway"]["port"], 18789);
        });
    }

    #[test]
    fn test_write_empty_platforms_is_noop() {
        with_temp_home(|| {
            let result = write_platform_config("/tmp/install", &[]);
            assert!(result.is_ok());
            let cfg = dirs::home_dir().unwrap().join(".openclaw").join("openclaw.json");
            assert!(!cfg.exists());
        });
    }
}
