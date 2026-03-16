use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 企业微信自建应用凭据（corpId + corpSecret + agentId，Stream 长连接双向通信）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WecomConfig {
    pub corp_id: String,
    pub corp_secret: String,
    pub agent_id: String,
}

/// 钉钉应用凭据（clientId/AppKey + clientSecret/AppSecret，Stream 长连接双向通信）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DingtalkConfig {
    pub client_id: String,
    pub client_secret: String,
}

/// 飞书应用机器人凭据（App ID + App Secret，WebSocket 长连接，无需公网 IP）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeishuConfig {
    pub app_id: String,
    pub app_secret: String,
}

/// QQ 开放平台机器人凭据（AppID + AppSecret，回调推送模式）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QqConfig {
    pub app_id: String,
    pub app_secret: String,
}

pub struct PlatformConfigs<'a> {
    pub wecom: Option<&'a WecomConfig>,
    pub dingtalk: Option<&'a DingtalkConfig>,
    pub feishu: Option<&'a FeishuConfig>,
    pub qq: Option<&'a QqConfig>,
}

pub fn write_platform_config(install_path: &str, cfg: PlatformConfigs<'_>) -> Result<()> {
    write_platform_config_to(config_dir(), install_path, cfg)
}

fn config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw")
}

fn write_platform_config_to(
    config_dir: PathBuf,
    _install_path: &str,
    cfg: PlatformConfigs<'_>,
) -> Result<()> {
    let any = cfg.wecom.is_some() || cfg.dingtalk.is_some()
        || cfg.feishu.is_some() || cfg.qq.is_some();
    if !any {
        return Ok(());
    }

    let config_path = config_dir.join("openclaw.json");

    let mut config: serde_json::Value = if config_path.exists() {
        let data = std::fs::read_to_string(&config_path)?;
        serde_json::from_str(&data)?
    } else {
        serde_json::json!({})
    };

    let channels = config["channels"].as_object().cloned().unwrap_or_default();
    let mut ch = channels;

    // 企业微信：自建应用，agent 子对象
    if let Some(w) = cfg.wecom {
        if !w.corp_id.is_empty() && !w.corp_secret.is_empty() && !w.agent_id.is_empty() {
            ch.insert("wecom".to_string(), serde_json::json!({
                "enabled": true,
                "agent": {
                    "corpId": w.corp_id,
                    "corpSecret": w.corp_secret,
                    "agentId": w.agent_id,
                }
            }));
        }
    }

    // 钉钉：Stream 长连接，clientId + clientSecret
    if let Some(d) = cfg.dingtalk {
        if !d.client_id.is_empty() && !d.client_secret.is_empty() {
            ch.insert("dingtalk".to_string(), serde_json::json!({
                "enabled": true,
                "clientId": d.client_id,
                "clientSecret": d.client_secret,
            }));
        }
    }

    // 飞书：应用机器人，WebSocket 长连接
    if let Some(f) = cfg.feishu {
        if !f.app_id.is_empty() && !f.app_secret.is_empty() {
            ch.insert("feishu".to_string(), serde_json::json!({
                "enabled": true,
                "accounts": {
                    "main": {
                        "appId": f.app_id,
                        "appSecret": f.app_secret,
                    }
                },
                "connectionMode": "websocket",
            }));
        }
    }

    // QQ：@sliverp/qqbot 插件，appId + clientSecret
    if let Some(q) = cfg.qq {
        if !q.app_id.is_empty() && !q.app_secret.is_empty() {
            ch.insert("qqbot".to_string(), serde_json::json!({
                "enabled": true,
                "appId": q.app_id,
                "clientSecret": q.app_secret,
            }));
        }
    }

    config["channels"] = serde_json::Value::Object(ch);

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

    fn empty() -> PlatformConfigs<'static> {
        PlatformConfigs { wecom: None, dingtalk: None, feishu: None, qq: None }
    }

    #[test]
    fn test_write_wecom_config() {
        let tmp = make_temp_dir();
        let config_dir = tmp.join(".openclaw");
        std::fs::create_dir_all(&config_dir).unwrap();

        let wecom = WecomConfig {
            corp_id: "ww123456".into(),
            corp_secret: "secret".into(),
            agent_id: "1000001".into(),
        };
        write_platform_config_to(config_dir.clone(), "/tmp/install", PlatformConfigs {
            wecom: Some(&wecom), ..empty()
        }).unwrap();

        let data = std::fs::read_to_string(config_dir.join("openclaw.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(v["channels"]["wecom"]["agent"]["corpId"], "ww123456");
        assert_eq!(v["channels"]["wecom"]["agent"]["agentId"], "1000001");
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_write_dingtalk_config() {
        let tmp = make_temp_dir();
        let config_dir = tmp.join(".openclaw");
        std::fs::create_dir_all(&config_dir).unwrap();

        let dt = DingtalkConfig {
            client_id: "dingappkey".into(),
            client_secret: "dingsecret".into(),
        };
        write_platform_config_to(config_dir.clone(), "/tmp/install", PlatformConfigs {
            dingtalk: Some(&dt), ..empty()
        }).unwrap();

        let data = std::fs::read_to_string(config_dir.join("openclaw.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(v["channels"]["dingtalk"]["clientId"], "dingappkey");
        assert_eq!(v["channels"]["dingtalk"]["clientSecret"], "dingsecret");
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_write_feishu_config() {
        let tmp = make_temp_dir();
        let config_dir = tmp.join(".openclaw");
        std::fs::create_dir_all(&config_dir).unwrap();

        let feishu = FeishuConfig { app_id: "cli_test".into(), app_secret: "fs_secret".into() };
        write_platform_config_to(config_dir.clone(), "/tmp/install", PlatformConfigs {
            feishu: Some(&feishu), ..empty()
        }).unwrap();

        let data = std::fs::read_to_string(config_dir.join("openclaw.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(v["channels"]["feishu"]["accounts"]["main"]["appId"], "cli_test");
        assert_eq!(v["channels"]["feishu"]["connectionMode"], "websocket");
        assert_eq!(v["channels"]["feishu"]["enabled"], true);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_write_qq_config() {
        let tmp = make_temp_dir();
        let config_dir = tmp.join(".openclaw");
        std::fs::create_dir_all(&config_dir).unwrap();

        let qq = QqConfig { app_id: "12345678".into(), app_secret: "my_secret".into() };
        write_platform_config_to(config_dir.clone(), "/tmp/install", PlatformConfigs {
            qq: Some(&qq), ..empty()
        }).unwrap();

        let data = std::fs::read_to_string(config_dir.join("openclaw.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(v["channels"]["qqbot"]["appId"], "12345678");
        assert_eq!(v["channels"]["qqbot"]["clientSecret"], "my_secret");
        assert_eq!(v["channels"]["qqbot"]["enabled"], true);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_preserves_existing_config() {
        let tmp = make_temp_dir();
        let config_dir = tmp.join(".openclaw");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("openclaw.json"),
            r#"{"gateway":{"port":18789}}"#,
        ).unwrap();

        let feishu = FeishuConfig { app_id: "cli_x".into(), app_secret: "s".into() };
        write_platform_config_to(config_dir.clone(), "/tmp/install", PlatformConfigs {
            feishu: Some(&feishu), ..empty()
        }).unwrap();

        let data = std::fs::read_to_string(config_dir.join("openclaw.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(v["gateway"]["port"], 18789);
        assert_eq!(v["channels"]["feishu"]["accounts"]["main"]["appId"], "cli_x");
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_write_empty_is_noop() {
        let tmp = make_temp_dir();
        let config_dir = tmp.join(".openclaw");
        let result = write_platform_config_to(config_dir.clone(), "/tmp/install", empty());
        assert!(result.is_ok());
        assert!(!config_dir.join("openclaw.json").exists());
        std::fs::remove_dir_all(&tmp).ok();
    }
}
