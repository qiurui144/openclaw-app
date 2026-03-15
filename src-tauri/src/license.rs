use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// RSA 公钥（PEM），用于离线验证 JWT 签名。
/// 多客户构建时，构建脚本将 keys/{CLIENT_ID}_pub.pem 复制为 keys/license_pub.pem。
/// 开发阶段使用 `DEV_SKIP_SIGNATURE=1` 跳过签名验证。
const PUBLIC_KEY_PEM: &str = include_str!("../keys/license_pub.pem");

/// 授权服务 API 地址（构建时注入，支持多客户独立部署）
const LICENSE_API: &str = match option_env!("OC_LICENSE_API") {
    Some(v) => v,
    None => "https://license.openclaw.cn/api",
};
const GRACE_PERIOD_DAYS: i64 = 7;
const JWT_ISSUER: &str = "license.openclaw.cn";

// ── 数据类型 ────────────────────────────────────────────────────────────────

/// 许可证计划
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LicensePlan {
    Free,
    ProSingle,
    ProAll,
    Enterprise,
}

impl Default for LicensePlan {
    fn default() -> Self { Self::Free }
}

/// JWT payload（由授权服务签发，客户端离线验证）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseClaims {
    pub sub: String,
    pub plan: LicensePlan,
    pub skills: Vec<String>,
    pub machine_id: String,
    pub devices_limit: u32,
    pub exp: i64,
    pub iss: String,
}

/// 授权模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    Free,
    Code,
    Payment,
}

impl Default for AuthMode {
    fn default() -> Self { Self::Free }
}

/// 前端展示用的许可证状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseStatus {
    pub authenticated: bool,
    pub plan: LicensePlan,
    pub auth_mode: AuthMode,
    pub user_id: Option<String>,
    pub skills: Vec<String>,
    pub expires_at: Option<String>,
    pub in_grace_period: bool,
    pub device_bound: bool,
}

impl Default for LicenseStatus {
    fn default() -> Self {
        Self {
            authenticated: false,
            plan: LicensePlan::Free,
            auth_mode: AuthMode::Free,
            user_id: None,
            skills: vec![],
            expires_at: None,
            in_grace_period: false,
            device_bound: false,
        }
    }
}

// ── 文件路径 ────────────────────────────────────────────────────────────────

fn jwt_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw")
        .join("license.jwt")
}

// ── 机器指纹 ────────────────────────────────────────────────────────────────

/// SHA256(hostname + MAC + OS + ARCH)
pub fn machine_id() -> String {
    let mut hasher = Sha256::new();
    if let Ok(name) = hostname::get() {
        hasher.update(name.to_string_lossy().as_bytes());
    }
    if let Ok(Some(mac)) = mac_address::get_mac_address() {
        hasher.update(mac.to_string().as_bytes());
    }
    hasher.update(std::env::consts::OS.as_bytes());
    hasher.update(std::env::consts::ARCH.as_bytes());
    hex::encode(hasher.finalize())
}

// ── JWT 验证 ────────────────────────────────────────────────────────────────

/// 使用 jsonwebtoken + RSA 公钥验证 JWT
fn verify_jwt(token: &str) -> Result<LicenseClaims> {
    let key = jsonwebtoken::DecodingKey::from_rsa_pem(PUBLIC_KEY_PEM.as_bytes())
        .context("RSA 公钥加载失败")?;

    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_issuer(&[JWT_ISSUER]);
    // 宽限期由我们自己检查，jsonwebtoken 默认 leeway 太短
    validation.leeway = (GRACE_PERIOD_DAYS * 86400) as u64;

    let token_data = jsonwebtoken::decode::<LicenseClaims>(token, &key, &validation)
        .context("JWT 验证失败")?;

    let claims = token_data.claims;

    // 授权码模式 machine_id == "*" 不校验设备
    // 扫码支付模式校验 machine_id 必须匹配
    if claims.machine_id != "*" {
        let local_mid = machine_id();
        if claims.machine_id != local_mid {
            anyhow::bail!("机器指纹不匹配（期望 {}，实际 {}）", claims.machine_id, local_mid);
        }
    }

    Ok(claims)
}

/// 开发模式：跳过签名验证，仅解析 payload（仅 debug 构建可用）
#[cfg(debug_assertions)]
fn verify_jwt_dev(token: &str) -> Result<LicenseClaims> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        anyhow::bail!("无效的 JWT 格式");
    }
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .context("JWT payload 解码失败")?;
    let claims: LicenseClaims =
        serde_json::from_slice(&payload_bytes).context("JWT payload 解析失败")?;

    let now = chrono::Utc::now().timestamp();
    if now > claims.exp + GRACE_PERIOD_DAYS * 86400 {
        anyhow::bail!("许可证已过期且超出宽限期");
    }
    if claims.iss != JWT_ISSUER {
        anyhow::bail!("JWT 签发者不匹配");
    }
    Ok(claims)
}

// ── 公开接口 ────────────────────────────────────────────────────────────────

/// 从本地文件加载并验证许可证
pub fn load_license() -> LicenseStatus {
    let token = match std::fs::read_to_string(jwt_path()) {
        Ok(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => return LicenseStatus::default(),
    };

    #[cfg(debug_assertions)]
    let verify_fn = if std::env::var("DEV_SKIP_SIGNATURE").is_ok() {
        verify_jwt_dev
    } else {
        verify_jwt
    };
    #[cfg(not(debug_assertions))]
    let verify_fn = verify_jwt;

    match verify_fn(&token) {
        Ok(claims) => {
            let now = chrono::Utc::now().timestamp();
            let is_code_mode = claims.machine_id == "*";
            LicenseStatus {
                authenticated: true,
                plan: claims.plan,
                auth_mode: if is_code_mode { AuthMode::Code } else { AuthMode::Payment },
                user_id: Some(claims.sub),
                skills: claims.skills,
                expires_at: chrono::DateTime::from_timestamp(claims.exp, 0)
                    .map(|dt| dt.format("%Y-%m-%d").to_string()),
                in_grace_period: now > claims.exp,
                device_bound: !is_code_mode,
            }
        }
        Err(_) => {
            std::fs::remove_file(jwt_path()).ok();
            LicenseStatus::default()
        }
    }
}

/// 保存 JWT 到本地文件
fn save_jwt(token: &str) -> Result<()> {
    let path = jwt_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, token)?;
    Ok(())
}

/// 发送短信验证码
pub async fn send_sms_code(phone: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let resp = client
        .post(format!("{LICENSE_API}/auth/send-code"))
        .json(&serde_json::json!({ "phone": phone }))
        .send()
        .await?;
    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        anyhow::bail!("发送验证码失败：{msg}");
    }
    Ok(())
}

/// 手机号 + 验证码登录，返回许可证状态
pub async fn login(phone: &str, code: &str) -> Result<LicenseStatus> {
    let mid = machine_id();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let resp = client
        .post(format!("{LICENSE_API}/auth/login"))
        .json(&serde_json::json!({
            "phone": phone,
            "code": code,
            "machine_id": mid,
        }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        anyhow::bail!("登录失败：{msg}");
    }

    #[derive(Deserialize)]
    struct LoginResp { jwt: String }
    let lr: LoginResp = resp.json().await?;
    save_jwt(&lr.jwt)?;
    Ok(load_license())
}

/// 刷新 JWT 令牌
pub async fn refresh_token() -> Result<LicenseStatus> {
    let old_token = std::fs::read_to_string(jwt_path()).context("未找到许可证文件")?;
    let mid = machine_id();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let resp = client
        .post(format!("{LICENSE_API}/auth/refresh"))
        .header("Authorization", format!("Bearer {}", old_token.trim()))
        .json(&serde_json::json!({ "machine_id": mid }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        anyhow::bail!("刷新令牌失败：{msg}");
    }

    #[derive(Deserialize)]
    struct RefreshResp { jwt: String }
    let rr: RefreshResp = resp.json().await?;
    save_jwt(&rr.jwt)?;
    Ok(load_license())
}

/// 使用授权码激活（不绑定设备）
pub async fn redeem_code(code: &str) -> Result<LicenseStatus> {
    let mid = machine_id(); // 仅用于防重复，不绑定
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let resp = client
        .post(format!("{LICENSE_API}/auth/redeem-code"))
        .json(&serde_json::json!({
            "code": code,
            "machine_id": mid,
        }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        // 解析服务端 JSON 错误
        let err_msg = serde_json::from_str::<serde_json::Value>(&msg)
            .ok()
            .and_then(|v| v["error"].as_str().map(String::from))
            .unwrap_or(msg);
        anyhow::bail!("{err_msg}");
    }

    #[derive(Deserialize)]
    struct RedeemResp {
        jwt: String,
        #[allow(dead_code)]
        remaining: Option<i64>,
    }
    let rr: RedeemResp = resp.json().await?;
    save_jwt(&rr.jwt)?;
    Ok(load_license())
}

/// 登出
pub fn logout() {
    std::fs::remove_file(jwt_path()).ok();
}

/// 获取当前 JWT 原文（供 skills_registry 调用 API 用）
pub fn current_jwt() -> Option<String> {
    std::fs::read_to_string(jwt_path())
        .ok()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

/// 检查是否有权访问某个付费 skill
pub fn can_access_skill(slug: &str) -> bool {
    let status = load_license();
    if !status.authenticated { return false; }
    match status.plan {
        LicensePlan::ProAll | LicensePlan::Enterprise => true,
        LicensePlan::ProSingle => {
            status.skills.contains(&"*".to_string())
                || status.skills.contains(&slug.to_string())
        }
        LicensePlan::Free => false,
    }
}

// ── 测试 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

    #[test]
    fn test_machine_id_deterministic_and_length() {
        let id1 = machine_id();
        let id2 = machine_id();
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 64);
    }

    #[test]
    fn test_default_license_status() {
        let s = LicenseStatus::default();
        assert!(!s.authenticated);
        assert_eq!(s.plan, LicensePlan::Free);
        assert!(s.skills.is_empty());
    }

    #[test]
    fn test_license_plan_serde_roundtrip() {
        for plan in [LicensePlan::Free, LicensePlan::ProSingle, LicensePlan::ProAll, LicensePlan::Enterprise] {
            let json = serde_json::to_string(&plan).unwrap();
            let parsed: LicensePlan = serde_json::from_str(&json).unwrap();
            assert_eq!(plan, parsed);
        }
    }

    #[test]
    fn test_license_plan_json_values() {
        assert_eq!(serde_json::to_string(&LicensePlan::Free).unwrap(), r#""free""#);
        assert_eq!(serde_json::to_string(&LicensePlan::ProSingle).unwrap(), r#""pro_single""#);
        assert_eq!(serde_json::to_string(&LicensePlan::ProAll).unwrap(), r#""pro_all""#);
        assert_eq!(serde_json::to_string(&LicensePlan::Enterprise).unwrap(), r#""enterprise""#);
    }

    #[test]
    fn test_verify_jwt_dev_valid() {
        let claims = LicenseClaims {
            sub: "user_123".into(),
            plan: LicensePlan::ProAll,
            skills: vec!["*".into()],
            machine_id: machine_id(),
            devices_limit: 1,
            exp: chrono::Utc::now().timestamp() + 86400 * 30,
            iss: JWT_ISSUER.into(),
        };
        let payload = serde_json::to_vec(&claims).unwrap();
        let header = r#"{"alg":"RS256","typ":"JWT"}"#;
        let token = format!(
            "{}.{}.fake_signature",
            URL_SAFE_NO_PAD.encode(header.as_bytes()),
            URL_SAFE_NO_PAD.encode(&payload),
        );
        let result = verify_jwt_dev(&token);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.sub, "user_123");
        assert_eq!(parsed.plan, LicensePlan::ProAll);
    }

    #[test]
    fn test_verify_jwt_dev_expired() {
        let claims = LicenseClaims {
            sub: "user_expired".into(),
            plan: LicensePlan::ProAll,
            skills: vec![],
            machine_id: machine_id(),
            devices_limit: 1,
            exp: chrono::Utc::now().timestamp() - 86400 * 40, // 40 天前过期
            iss: JWT_ISSUER.into(),
        };
        let payload = serde_json::to_vec(&claims).unwrap();
        let token = format!(
            "{}.{}.fake",
            URL_SAFE_NO_PAD.encode(b"{}"),
            URL_SAFE_NO_PAD.encode(&payload),
        );
        assert!(verify_jwt_dev(&token).is_err());
    }

    #[test]
    fn test_verify_jwt_dev_wrong_issuer() {
        let claims = LicenseClaims {
            sub: "user_bad".into(),
            plan: LicensePlan::Free,
            skills: vec![],
            machine_id: machine_id(),
            devices_limit: 1,
            exp: chrono::Utc::now().timestamp() + 86400,
            iss: "evil.example.com".into(),
        };
        let payload = serde_json::to_vec(&claims).unwrap();
        let token = format!(
            "{}.{}.fake",
            URL_SAFE_NO_PAD.encode(b"{}"),
            URL_SAFE_NO_PAD.encode(&payload),
        );
        assert!(verify_jwt_dev(&token).is_err());
    }

    #[test]
    fn test_verify_jwt_dev_in_grace_period() {
        let claims = LicenseClaims {
            sub: "user_grace".into(),
            plan: LicensePlan::ProAll,
            skills: vec!["*".into()],
            machine_id: machine_id(),
            devices_limit: 1,
            exp: chrono::Utc::now().timestamp() - 86400 * 3, // 3 天前过期（在 7 天宽限内）
            iss: JWT_ISSUER.into(),
        };
        let payload = serde_json::to_vec(&claims).unwrap();
        let token = format!(
            "{}.{}.fake",
            URL_SAFE_NO_PAD.encode(b"{}"),
            URL_SAFE_NO_PAD.encode(&payload),
        );
        assert!(verify_jwt_dev(&token).is_ok());
    }

    #[test]
    fn test_verify_jwt_invalid_format() {
        assert!(verify_jwt_dev("not.a.valid.jwt.token").is_err());
        assert!(verify_jwt_dev("only_one_part").is_err());
        assert!(verify_jwt_dev("").is_err());
    }

    #[test]
    fn test_license_claims_serde() {
        let claims = LicenseClaims {
            sub: "u_1".into(),
            plan: LicensePlan::Enterprise,
            skills: vec!["rag".into(), "coding".into()],
            machine_id: "abc123".into(),
            devices_limit: 5,
            exp: 1700000000,
            iss: JWT_ISSUER.into(),
        };
        let json = serde_json::to_string(&claims).unwrap();
        let parsed: LicenseClaims = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.sub, "u_1");
        assert_eq!(parsed.devices_limit, 5);
        assert_eq!(parsed.skills.len(), 2);
    }
}
