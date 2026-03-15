/**
 * 独立密钥管理框架
 *
 * 设计原则：一授权一生成一下发
 *   - 每次授权码兑换 → 生成独立的 AES-256 主密钥
 *   - 每个 Skill 内容 → 从主密钥派生独立的内容密钥
 *   - 吊销授权 → 删除主密钥 → 所有派生密钥立即失效
 *
 * 密钥层级：
 *   RSA 密钥对（全局，JWT 签名用）
 *     └─ activation_master_key（每授权一个，AES-256）
 *          └─ content_key = HMAC(master_key, slug)（每 Skill 一个，AES-256-GCM 加密用）
 *
 * 存储：key_store 表（SQLite），主密钥加密存储（用服务端 KEK 保护）
 */

import crypto from "crypto";
import { readFileSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";
import { getDb } from "./db.js";

const __dirname = dirname(fileURLToPath(import.meta.url));

// Key Encryption Key — 保护存储在数据库中的主密钥
const KEK_SOURCE = process.env.KEY_ENCRYPTION_KEY;
if (!KEK_SOURCE) {
  console.warn("[keymanager] 警告：未设置 KEY_ENCRYPTION_KEY，使用 ADMIN_KEY 作为 KEK（生产环境务必单独配置）");
}
const KEK = crypto.createHash("sha256")
  .update(KEK_SOURCE || process.env.ADMIN_KEY || "")
  .digest();

// ── 表初始化 ────────────────────────────────────────────────

export function initKeyStore() {
  const db = getDb();

  db.prepare(`
    CREATE TABLE IF NOT EXISTS key_store (
      activation_id  TEXT PRIMARY KEY,
      encrypted_key  TEXT NOT NULL,
      key_nonce      TEXT NOT NULL,
      status         TEXT NOT NULL DEFAULT 'active',
      created_at     TEXT NOT NULL DEFAULT (datetime('now')),
      revoked_at     TEXT
    )
  `).run();

  db.prepare("CREATE INDEX IF NOT EXISTS idx_keystore_status ON key_store(status)").run();
}

// ── RSA 密钥管理（JWT 签名）────────────────────────────────

let _privateKey = null;
let _publicKey = null;

export function getPrivateKey() {
  if (!_privateKey) {
    const path = process.env.PRIVATE_KEY_PATH
      || resolve(__dirname, "../../../src-tauri/keys/license_priv.pem");
    _privateKey = readFileSync(path, "utf-8");
  }
  return _privateKey;
}

export function getPublicKey() {
  if (!_publicKey) {
    const path = process.env.PUBLIC_KEY_PATH
      || resolve(__dirname, "../../../src-tauri/keys/license_pub.pem");
    _publicKey = readFileSync(path, "utf-8");
  }
  return _publicKey;
}

// ── 主密钥生命周期 ──────────────────────────────────────────

/**
 * 生成并存储激活主密钥
 * @param {string} activationId - 唯一标识（授权码 code 或 redemption ID）
 * @returns {Buffer} 32 字节主密钥（明文，仅在生成时返回一次用于即时操作）
 */
export function generateMasterKey(activationId) {
  const db = getDb();

  // 事务保护：幂等检查 + 插入原子化，防止并发竞态
  const doGenerate = db.transaction(() => {
    const existing = db.prepare("SELECT * FROM key_store WHERE activation_id = ?").get(activationId);
    if (existing && existing.status === "active") {
      return decryptStoredKey(existing.encrypted_key, existing.key_nonce);
    }

    const masterKey = crypto.randomBytes(32);
    const { ciphertext, nonce } = encryptWithKEK(masterKey);

    db.prepare(`
      INSERT OR REPLACE INTO key_store (activation_id, encrypted_key, key_nonce, status)
      VALUES (?, ?, ?, 'active')
    `).run(activationId, ciphertext, nonce);

    return masterKey;
  });

  return doGenerate();
}

/**
 * 获取激活主密钥（仅活跃状态）
 * @param {string} activationId
 * @returns {Buffer|null} 主密钥明文，吊销或不存在则返回 null
 */
export function getMasterKey(activationId) {
  const db = getDb();
  const row = db.prepare(
    "SELECT * FROM key_store WHERE activation_id = ? AND status = 'active'"
  ).get(activationId);

  if (!row) return null;
  return decryptStoredKey(row.encrypted_key, row.key_nonce);
}

/**
 * 吊销激活主密钥（所有派生密钥立即失效）
 * @param {string} activationId
 * @returns {boolean} 是否成功吊销
 */
export function revokeMasterKey(activationId) {
  const db = getDb();
  const result = db.prepare(
    "UPDATE key_store SET status = 'revoked', revoked_at = datetime('now') WHERE activation_id = ? AND status = 'active'"
  ).run(activationId);
  return result.changes > 0;
}

/**
 * 检查密钥是否活跃
 * @param {string} activationId
 * @returns {boolean}
 */
export function isKeyActive(activationId) {
  const db = getDb();
  const row = db.prepare(
    "SELECT status FROM key_store WHERE activation_id = ?"
  ).get(activationId);
  return row?.status === "active";
}

// ── 内容密钥派生 ────────────────────────────────────────────

/**
 * 派生 Skill 内容加密密钥
 * @param {string} activationId - 授权标识
 * @param {string} slug - Skill slug
 * @returns {Buffer|null} 32 字节 AES-256 密钥，密钥不存在或已吊销返回 null
 */
export function deriveContentKey(activationId, slug) {
  const masterKey = getMasterKey(activationId);
  if (!masterKey) return null;

  return crypto.createHmac("sha256", masterKey)
    .update(slug)
    .digest();
}

// ── 内容加密/解密 ───────────────────────────────────────────

/**
 * AES-256-GCM 加密
 * @param {string} plaintext
 * @param {Buffer} key - 32 字节
 * @returns {{ ciphertext: string, nonce: string }} base64 编码
 */
export function encryptContent(plaintext, key) {
  const nonce = crypto.randomBytes(12);
  const cipher = crypto.createCipheriv("aes-256-gcm", key, nonce);
  const encrypted = Buffer.concat([cipher.update(plaintext, "utf-8"), cipher.final()]);
  const tag = cipher.getAuthTag();

  return {
    ciphertext: Buffer.concat([encrypted, tag]).toString("base64"),
    nonce: nonce.toString("base64"),
  };
}

// ── KEK 加密/解密（保护存储的主密钥）───────────────────────

function encryptWithKEK(plainKey) {
  const nonce = crypto.randomBytes(12);
  const cipher = crypto.createCipheriv("aes-256-gcm", KEK, nonce);
  const encrypted = Buffer.concat([cipher.update(plainKey), cipher.final()]);
  const tag = cipher.getAuthTag();

  return {
    ciphertext: Buffer.concat([encrypted, tag]).toString("base64"),
    nonce: nonce.toString("base64"),
  };
}

function decryptStoredKey(ciphertextB64, nonceB64) {
  const ciphertext = Buffer.from(ciphertextB64, "base64");
  const nonce = Buffer.from(nonceB64, "base64");

  // GCM tag 是最后 16 字节
  const tag = ciphertext.subarray(ciphertext.length - 16);
  const data = ciphertext.subarray(0, ciphertext.length - 16);

  const decipher = crypto.createDecipheriv("aes-256-gcm", KEK, nonce);
  decipher.setAuthTag(tag);
  return Buffer.concat([decipher.update(data), decipher.final()]);
}
