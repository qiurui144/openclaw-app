use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 构建时注入客户端配置
const CLIENT_ID: &str = match option_env!("OC_CLIENT_ID") {
    Some(v) => v,
    None => "default",
};

const LICENSE_API: &str = match option_env!("OC_LICENSE_API") {
    Some(v) => v,
    None => "https://license.openclaw.cn/api",
};

const JWT_ISSUER: &str = "license.openclaw.cn";

// ── 数据类型 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationQr {
    pub ticket: String,
    pub qr_url: String,
    pub expires_in: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationResult {
    pub verified: bool,
    pub expired: Option<bool>,
    pub activation_token: Option<String>,
}

// ── 本地激活标记 ────────────────────────────────────────────────────────────

fn activation_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw")
        .join("activation.jwt")
}

/// 检查是否已通过公众号验证
pub fn is_activated() -> bool {
    let path = activation_path();
    let token = match std::fs::read_to_string(&path) {
        Ok(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => return false,
    };

    // 验证 activation JWT 的有效期
    let verify_fn = if std::env::var("DEV_SKIP_SIGNATURE").is_ok() {
        verify_activation_dev
    } else {
        verify_activation
    };

    verify_fn(&token).is_ok()
}

/// 请求带参二维码
pub async fn request_qrcode() -> Result<ActivationQr> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let resp = client
        .post(format!("{LICENSE_API}/activation/qrcode"))
        .json(&serde_json::json!({ "client_id": CLIENT_ID }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        anyhow::bail!("获取二维码失败：{msg}");
    }

    let qr: ActivationQr = resp.json().await?;
    Ok(qr)
}

/// 轮询验证状态
pub async fn check_activation(ticket: &str) -> Result<ActivationResult> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let resp = client
        .get(format!("{LICENSE_API}/activation/check"))
        .query(&[("ticket", ticket)])
        .send()
        .await?;

    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        anyhow::bail!("检查激活状态失败：{msg}");
    }

    let result: ActivationResult = resp.json().await?;

    // 验证通过 → 保存 activation_token 到本地
    if result.verified {
        if let Some(ref token) = result.activation_token {
            save_activation_token(token)?;
        }
    }

    Ok(result)
}

/// 获取 CLIENT_ID（供前端显示品牌等）
pub fn get_client_id() -> &'static str {
    CLIENT_ID
}

// ── 内部函数 ────────────────────────────────────────────────────────────────

fn save_activation_token(token: &str) -> Result<()> {
    let path = activation_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, token)?;
    Ok(())
}

/// 使用 RSA 公钥验证 activation JWT
fn verify_activation(token: &str) -> Result<serde_json::Value> {
    let public_key_pem = include_str!("../keys/license_pub.pem");
    let key = jsonwebtoken::DecodingKey::from_rsa_pem(public_key_pem.as_bytes())
        .context("RSA 公钥加载失败")?;

    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_issuer(&[JWT_ISSUER]);
    validation.leeway = 86400; // 1 天宽限

    let token_data = jsonwebtoken::decode::<serde_json::Value>(token, &key, &validation)
        .context("激活令牌验证失败")?;

    // 确认是 activation 类型
    if token_data.claims.get("type").and_then(|v| v.as_str()) != Some("activation") {
        anyhow::bail!("不是有效的激活令牌");
    }

    Ok(token_data.claims)
}

/// 开发模式：跳过签名验证
fn verify_activation_dev(token: &str) -> Result<serde_json::Value> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        anyhow::bail!("无效的 JWT 格式");
    }
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .context("JWT payload 解码失败")?;
    let claims: serde_json::Value =
        serde_json::from_slice(&payload_bytes).context("JWT payload 解析失败")?;

    // 检查过期
    if let Some(exp) = claims.get("exp").and_then(|v| v.as_i64()) {
        let now = chrono::Utc::now().timestamp();
        if now > exp + 86400 {
            anyhow::bail!("激活令牌已过期");
        }
    }

    if claims.get("type").and_then(|v| v.as_str()) != Some("activation") {
        anyhow::bail!("不是有效的激活令牌");
    }

    Ok(claims)
}

// ── 测试 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_id_default() {
        // 在没有设置 OC_CLIENT_ID 时应该是 "default"
        assert_eq!(CLIENT_ID, "default");
    }

    #[test]
    fn test_activation_path() {
        let path = activation_path();
        assert!(path.to_string_lossy().contains(".openclaw"));
        assert!(path.to_string_lossy().ends_with("activation.jwt"));
    }

    #[test]
    fn test_is_activated_no_file() {
        // 没有文件时应该返回 false
        assert!(!is_activated());
    }

    #[test]
    fn test_verify_activation_dev_invalid() {
        assert!(verify_activation_dev("not.a.jwt").is_err());
        assert!(verify_activation_dev("only_one_part").is_err());
    }

    #[test]
    fn test_verify_activation_dev_wrong_type() {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

        let claims = serde_json::json!({
            "type": "license",
            "exp": chrono::Utc::now().timestamp() + 86400,
        });
        let payload = serde_json::to_vec(&claims).unwrap();
        let token = format!(
            "{}.{}.fake",
            URL_SAFE_NO_PAD.encode(b"{}"),
            URL_SAFE_NO_PAD.encode(&payload),
        );
        assert!(verify_activation_dev(&token).is_err());
    }

    #[test]
    fn test_verify_activation_dev_valid() {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

        let claims = serde_json::json!({
            "type": "activation",
            "openid": "test_openid",
            "client_id": "default",
            "exp": chrono::Utc::now().timestamp() + 86400 * 365,
            "iss": JWT_ISSUER,
        });
        let payload = serde_json::to_vec(&claims).unwrap();
        let token = format!(
            "{}.{}.fake",
            URL_SAFE_NO_PAD.encode(b"{}"),
            URL_SAFE_NO_PAD.encode(&payload),
        );
        assert!(verify_activation_dev(&token).is_ok());
    }
}
