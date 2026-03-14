use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    WeWork,
    DingTalk,
    Feishu,
}

impl Platform {
    pub fn channel_key(&self) -> &'static str {
        match self {
            Platform::WeWork    => "wecom",
            Platform::DingTalk  => "dingtalk",
            Platform::Feishu    => "feishu",
        }
    }

    pub fn doc_url(&self) -> &'static str {
        match self {
            Platform::WeWork =>
                "https://work.weixin.qq.com/api/doc/90000/90136/91770",
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

/// QQ 开放平台机器人凭据（与普通 Webhook 平台不同，需要 AppID + AppSecret）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QqConfig {
    pub app_id: String,
    pub app_secret: String,  // 用于 ED25519 签名验证和换取 AccessToken
}

pub fn write_platform_config(
    install_path: &str,
    platforms: &[PlatformEntry],
    qq: Option<&QqConfig>,
) -> Result<()> {
    write_platform_config_to(config_dir(), install_path, platforms, qq)
}

fn config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw")
}

fn write_platform_config_to(
    config_dir: PathBuf,
    _install_path: &str,
    platforms: &[PlatformEntry],
    qq: Option<&QqConfig>,
) -> Result<()> {
    if platforms.is_empty() && qq.is_none() { return Ok(()); }

    let config_path = config_dir.join("openclaw.json");

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

    // QQ 机器人使用 AppID + AppSecret（ED25519 签名验证），而非 webhookUrl
    if let Some(q) = qq {
        if !q.app_id.is_empty() && !q.app_secret.is_empty() {
            channels_map.insert(
                "qq".to_string(),
                serde_json::json!({
                    "appId": q.app_id,
                    "appSecret": q.app_secret,
                    // 回调路径由 OpenClaw 服务固定提供，用户需将此路径注册到 QQ 开放平台
                    "callbackPath": "/webhook/qq",
                }),
            );
        }
    }

    config["channels"] = serde_json::Value::Object(channels_map);

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_temp_dir() -> PathBuf {
        let tmp = std::env::temp_dir()
            .join(format!("oc_pf_test_{}_{:?}", std::process::id(), std::thread::current().id()));
        std::fs::create_dir_all(&tmp).unwrap();
        tmp
    }

    #[test]
    fn test_platform_channel_key() {
        assert_eq!(Platform::Feishu.channel_key(), "feishu");
        assert_eq!(Platform::DingTalk.channel_key(), "dingtalk");
        assert_eq!(Platform::WeWork.channel_key(), "wecom");
    }

    #[test]
    fn test_doc_url_not_empty() {
        for p in [Platform::WeWork, Platform::DingTalk, Platform::Feishu] {
            assert!(!p.doc_url().is_empty());
            assert!(p.doc_url().starts_with("https://"));
        }
    }

    #[test]
    fn test_write_platform_config_creates_channels() {
        let tmp = make_temp_dir();
        let config_dir = tmp.join(".openclaw");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("openclaw.json"),
            r#"{"gateway":{"port":18789}}"#
        ).unwrap();

        let entries = vec![
            PlatformEntry {
                platform: Platform::Feishu,
                webhook_url: "https://open.feishu.cn/hook/test".into(),
            },
        ];
        write_platform_config_to(config_dir.clone(), "/tmp/install", &entries, None).unwrap();

        let data = std::fs::read_to_string(config_dir.join("openclaw.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(v["channels"]["feishu"]["webhookUrl"], "https://open.feishu.cn/hook/test");
        assert_eq!(v["gateway"]["port"], 18789);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_write_qq_config() {
        let tmp = make_temp_dir();
        let config_dir = tmp.join(".openclaw");
        std::fs::create_dir_all(&config_dir).unwrap();

        let qq = QqConfig {
            app_id: "12345678".into(),
            app_secret: "my_secret".into(),
        };
        write_platform_config_to(config_dir.clone(), "/tmp/install", &[], Some(&qq)).unwrap();

        let data = std::fs::read_to_string(config_dir.join("openclaw.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(v["channels"]["qq"]["appId"], "12345678");
        assert_eq!(v["channels"]["qq"]["callbackPath"], "/webhook/qq");
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_write_empty_platforms_is_noop() {
        let tmp = make_temp_dir();
        let config_dir = tmp.join(".openclaw");
        let result = write_platform_config_to(config_dir.clone(), "/tmp/install", &[], None);
        assert!(result.is_ok());
        assert!(!config_dir.join("openclaw.json").exists());
        std::fs::remove_dir_all(&tmp).ok();
    }
}
