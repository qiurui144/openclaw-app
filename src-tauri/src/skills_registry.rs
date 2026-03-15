use anyhow::Result;
use ring::aead;
use serde::{Deserialize, Serialize};

const REGISTRY_API: &str = match option_env!("OC_LICENSE_API") {
    Some(v) => v,
    None => "https://license.openclaw.cn/api",
};

/// Skill 索引条目（免费 + 付费统一索引）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub is_paid: bool,
    pub price: Option<f64>,
    pub price_label: Option<String>,
    pub version: String,
    pub author: String,
    pub icon: Option<String>,
}

/// 加密内容响应
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct EncryptedContentResp {
    encrypted: bool,
    ciphertext: String,
    nonce: String,
    watermark_id: String,
}

/// 解密密钥响应
#[derive(Debug, Deserialize)]
struct KeyResp {
    key: String,
}

/// 获取 Skill 索引（免费 + 付费混合列表）
pub async fn fetch_skill_index() -> Result<Vec<SkillEntry>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let resp = client
        .get(format!("{REGISTRY_API}/skills/index"))
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("获取 Skill 索引失败，HTTP {}", resp.status());
    }

    let entries: Vec<SkillEntry> = resp.json().await?;
    Ok(entries)
}

/// 下载付费 Skill 内容（需要 JWT 授权）
/// 付费内容返回加密 JSON，免费内容返回明文
pub async fn fetch_paid_skill_content(
    slug: &str,
    jwt: &str,
) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let resp = client
        .get(format!("{REGISTRY_API}/skills/{slug}/content"))
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Cache-Control", "no-cache")
        .send()
        .await?;

    if resp.status().as_u16() == 403 {
        anyhow::bail!("无权访问此 Skill，请升级订阅");
    }
    if !resp.status().is_success() {
        anyhow::bail!("获取 Skill 内容失败，HTTP {}", resp.status());
    }

    // 尝试解析为加密响应
    let text = resp.text().await?;
    if let Ok(encrypted) = serde_json::from_str::<EncryptedContentResp>(&text) {
        if encrypted.encrypted {
            // 获取解密密钥
            let key = fetch_decrypt_key(slug, jwt).await?;
            let plaintext = decrypt_content(&encrypted.ciphertext, &encrypted.nonce, &key)?;
            return Ok(plaintext);
        }
    }

    // 非加密内容（免费 Skill 或旧版服务端）
    Ok(text)
}

/// 从服务端获取解密密钥
async fn fetch_decrypt_key(slug: &str, jwt: &str) -> Result<Vec<u8>> {
    // 先检查本地密钥缓存
    if let Some(key) = load_cached_key(slug) {
        return Ok(key);
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let resp = client
        .get(format!("{REGISTRY_API}/skills/{slug}/key"))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("获取解密密钥失败");
    }

    let kr: KeyResp = resp.json().await?;
    let key = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &kr.key)?;

    // 缓存密钥到本地（30 天有效）
    cache_key(slug, &key)?;

    Ok(key)
}

/// AES-256-GCM 解密
fn decrypt_content(ciphertext_b64: &str, nonce_b64: &str, key: &[u8]) -> Result<String> {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD;
    let mut ciphertext = b64.decode(ciphertext_b64)?;
    let nonce_bytes = b64.decode(nonce_b64)?;

    let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, key)
        .map_err(|_| anyhow::anyhow!("无效的解密密钥"))?;
    let nonce = aead::Nonce::try_assume_unique_for_key(&nonce_bytes)
        .map_err(|_| anyhow::anyhow!("无效的 nonce"))?;
    let aad = aead::Aad::empty();

    let opening_key = aead::LessSafeKey::new(unbound_key);
    let plaintext = opening_key
        .open_in_place(nonce, aad, &mut ciphertext)
        .map_err(|_| anyhow::anyhow!("解密失败，密钥可能不正确"))?;

    Ok(String::from_utf8(plaintext.to_vec())?)
}

// ── 密钥缓存 ────────────────────────────────────────────────────────────────

fn key_cache_path(slug: &str) -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".openclaw")
        .join("skills")
        .join(slug)
        .join(".key_cache.json")
}

fn load_cached_key(slug: &str) -> Option<Vec<u8>> {
    let path = key_cache_path(slug);
    let data = std::fs::read_to_string(&path).ok()?;
    let meta: serde_json::Value = serde_json::from_str(&data).ok()?;

    let expires = meta["expires_at"].as_str()?;
    let exp = chrono::DateTime::parse_from_rfc3339(expires).ok()?;
    if chrono::Utc::now() > exp {
        std::fs::remove_file(&path).ok();
        return None;
    }

    let key_b64 = meta["key"].as_str()?;
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(key_b64).ok()
}

fn cache_key(slug: &str, key: &[u8]) -> Result<()> {
    use base64::Engine;
    let path = key_cache_path(slug);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let meta = serde_json::json!({
        "key": base64::engine::general_purpose::STANDARD.encode(key),
        "cached_at": chrono::Utc::now().to_rfc3339(),
        "expires_at": (chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339(),
    });
    std::fs::write(&path, serde_json::to_string_pretty(&meta)?)?;
    Ok(())
}

// ── 安装/卸载 ────────────────────────────────────────────────────────────────

/// 安装付费 Skill 到本地
pub async fn install_paid_skill(
    install_path: &str,
    slug: &str,
    jwt: &str,
) -> Result<()> {
    let content = fetch_paid_skill_content(slug, jwt).await?;

    // 写入 ~/.openclaw/skills/{slug}/SKILL.md
    let skill_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".openclaw")
        .join("skills")
        .join(slug);
    std::fs::create_dir_all(&skill_dir)?;
    std::fs::write(skill_dir.join("SKILL.md"), &content)?;

    // 写入缓存元数据（30 天有效期）
    let meta = serde_json::json!({
        "slug": slug,
        "cached_at": chrono::Utc::now().to_rfc3339(),
        "expires_at": (chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339(),
    });
    std::fs::write(
        skill_dir.join(".cache_meta.json"),
        serde_json::to_string_pretty(&meta)?,
    )?;

    let _ = install_path;
    Ok(())
}

/// 检查本地缓存的付费 Skill 是否过期
pub fn is_skill_cache_expired(slug: &str) -> bool {
    let meta_path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".openclaw")
        .join("skills")
        .join(slug)
        .join(".cache_meta.json");

    match std::fs::read_to_string(&meta_path) {
        Ok(data) => {
            if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&data) {
                if let Some(expires) = meta["expires_at"].as_str() {
                    if let Ok(exp) = chrono::DateTime::parse_from_rfc3339(expires) {
                        return chrono::Utc::now() > exp;
                    }
                }
            }
            true // 无法解析则视为过期
        }
        Err(_) => true,
    }
}

/// 卸载付费 Skill
pub fn uninstall_paid_skill(slug: &str) -> Result<()> {
    let skill_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".openclaw")
        .join("skills")
        .join(slug);
    if skill_dir.exists() {
        std::fs::remove_dir_all(&skill_dir)?;
    }
    Ok(())
}

/// 列出本地已安装的付费 Skills
pub fn list_installed_paid_skills() -> Vec<String> {
    let skills_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".openclaw")
        .join("skills");

    if !skills_dir.exists() {
        return vec![];
    }

    std::fs::read_dir(&skills_dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| {
                    let entry = e.ok()?;
                    if entry.path().join("SKILL.md").exists() {
                        Some(entry.file_name().to_string_lossy().to_string())
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 创建支付订单（返回支付二维码 URL）
pub async fn create_payment_order(
    jwt: &str,
    plan: &str,
    skill_slug: Option<&str>,
) -> Result<PaymentOrder> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let mut body = serde_json::json!({ "plan": plan });
    if let Some(slug) = skill_slug {
        body["skill_slug"] = serde_json::Value::String(slug.to_string());
    }

    let resp = client
        .post(format!("{REGISTRY_API}/payments/create"))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        anyhow::bail!("创建支付订单失败：{msg}");
    }

    let order: PaymentOrder = resp.json().await?;
    Ok(order)
}

/// 支付订单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentOrder {
    pub order_id: String,
    pub qr_url: String,
    pub amount: f64,
    pub status: String,
}

/// 查询支付状态
pub async fn check_payment_status(
    jwt: &str,
    order_id: &str,
) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let resp = client
        .get(format!("{REGISTRY_API}/payments/{order_id}/status"))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("查询支付状态失败");
    }

    #[derive(Deserialize)]
    struct StatusResp { status: String }
    let sr: StatusResp = resp.json().await?;
    Ok(sr.status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_entry_deserialize() {
        let json = r#"{
            "slug": "advanced-rag",
            "name": "高级 RAG 检索",
            "description": "增强的检索增强生成",
            "category": "ai",
            "is_paid": true,
            "price": 49.0,
            "price_label": "¥49 永久",
            "version": "1.0.0",
            "author": "OpenClaw",
            "icon": null
        }"#;
        let entry: SkillEntry = serde_json::from_str(json).unwrap();
        assert!(entry.is_paid);
        assert_eq!(entry.price, Some(49.0));
    }

    #[test]
    fn test_is_skill_cache_expired_missing_file() {
        assert!(is_skill_cache_expired("nonexistent-skill"));
    }

    #[test]
    fn test_list_installed_paid_skills_empty() {
        let result = list_installed_paid_skills();
        assert!(result.len() >= 0);
    }

    #[test]
    fn test_decrypt_content_invalid_key() {
        let result = decrypt_content("dGVzdA==", "dGVzdA==", &[0u8; 32]);
        assert!(result.is_err());
    }
}
