import Database from "better-sqlite3";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";
import { mkdirSync } from "fs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const DB_PATH = process.env.DB_PATH || resolve(__dirname, "../data/license.db");

let db;

export function initDb() {
  mkdirSync(dirname(DB_PATH), { recursive: true });
  db = new Database(DB_PATH);
  db.pragma("journal_mode = WAL");
  db.pragma("foreign_keys = ON");

  // ── 用户表 ──────────────────────────────────────────────
  db.prepare(`
    CREATE TABLE IF NOT EXISTS users (
      id         TEXT PRIMARY KEY,
      phone      TEXT UNIQUE,
      plan       TEXT NOT NULL DEFAULT 'free',
      skills     TEXT NOT NULL DEFAULT '[]',
      devices    INTEGER NOT NULL DEFAULT 1,
      auth_mode  TEXT NOT NULL DEFAULT 'free',
      created_at TEXT NOT NULL DEFAULT (datetime('now')),
      updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    )
  `).run();

  // ── 短信验证码 ──────────────────────────────────────────
  db.prepare(`
    CREATE TABLE IF NOT EXISTS sms_codes (
      phone      TEXT NOT NULL,
      code       TEXT NOT NULL,
      expires_at TEXT NOT NULL,
      used       INTEGER NOT NULL DEFAULT 0
    )
  `).run();

  // ── Skills 内容表 ───────────────────────────────────────
  db.prepare(`
    CREATE TABLE IF NOT EXISTS skills (
      slug        TEXT PRIMARY KEY,
      name        TEXT NOT NULL,
      description TEXT NOT NULL DEFAULT '',
      category    TEXT NOT NULL DEFAULT 'general',
      is_paid     INTEGER NOT NULL DEFAULT 0,
      price       REAL,
      price_label TEXT,
      version     TEXT NOT NULL DEFAULT '1.0.0',
      author      TEXT NOT NULL DEFAULT 'OpenClaw',
      icon        TEXT,
      content     TEXT,
      created_at  TEXT NOT NULL DEFAULT (datetime('now')),
      updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
    )
  `).run();

  // ── 支付订单表 ──────────────────────────────────────────
  db.prepare(`
    CREATE TABLE IF NOT EXISTS orders (
      id          TEXT PRIMARY KEY,
      user_id     TEXT,
      plan        TEXT NOT NULL,
      skill_slug  TEXT,
      amount      REAL NOT NULL,
      status      TEXT NOT NULL DEFAULT 'pending',
      qr_url      TEXT,
      created_at  TEXT NOT NULL DEFAULT (datetime('now')),
      paid_at     TEXT
    )
  `).run();

  // ── 授权码表 ────────────────────────────────────────────
  db.prepare(`
    CREATE TABLE IF NOT EXISTS activation_codes (
      code            TEXT PRIMARY KEY,
      plan            TEXT NOT NULL,
      skills          TEXT NOT NULL DEFAULT '[]',
      download_limit  INTEGER NOT NULL DEFAULT 1,
      download_used   INTEGER NOT NULL DEFAULT 0,
      expires_at      TEXT,
      note            TEXT,
      created_by      TEXT,
      created_at      TEXT NOT NULL DEFAULT (datetime('now'))
    )
  `).run();

  // ── 授权码使用记录 ──────────────────────────────────────
  db.prepare(`
    CREATE TABLE IF NOT EXISTS code_redemptions (
      id          INTEGER PRIMARY KEY AUTOINCREMENT,
      code        TEXT NOT NULL REFERENCES activation_codes(code),
      user_id     TEXT NOT NULL REFERENCES users(id),
      machine_id  TEXT,
      redeemed_at TEXT NOT NULL DEFAULT (datetime('now'))
    )
  `).run();

  // ── 用户设备绑定表（扫码支付模式）─────────────────────
  db.prepare(`
    CREATE TABLE IF NOT EXISTS user_devices (
      id          INTEGER PRIMARY KEY AUTOINCREMENT,
      user_id     TEXT NOT NULL REFERENCES users(id),
      machine_id  TEXT NOT NULL,
      device_name TEXT,
      bound_at    TEXT NOT NULL DEFAULT (datetime('now')),
      UNIQUE(user_id, machine_id)
    )
  `).run();

  // ── 客户配置表（多客户隔离）─────────────────────────────
  db.prepare(`
    CREATE TABLE IF NOT EXISTS clients (
      id            TEXT PRIMARY KEY,
      name          TEXT NOT NULL,
      wechat_appid  TEXT,
      wechat_secret TEXT,
      wechat_token  TEXT,
      license_api   TEXT,
      branding      TEXT DEFAULT '{}',
      created_at    TEXT DEFAULT (datetime('now'))
    )
  `).run();

  // ── 公众号激活票据表 ──────────────────────────────────
  db.prepare(`
    CREATE TABLE IF NOT EXISTS activation_tickets (
      ticket      TEXT PRIMARY KEY,
      client_id   TEXT NOT NULL,
      status      TEXT NOT NULL DEFAULT 'pending',
      openid      TEXT,
      created_at  TEXT NOT NULL DEFAULT (datetime('now')),
      verified_at TEXT
    )
  `).run();

  // ── 管理操作审计日志 ──────────────────────────────────
  db.prepare(`
    CREATE TABLE IF NOT EXISTS admin_audit_log (
      id         INTEGER PRIMARY KEY AUTOINCREMENT,
      action     TEXT NOT NULL,
      detail     TEXT,
      ip         TEXT,
      created_at TEXT DEFAULT (datetime('now'))
    )
  `).run();

  // ── 索引 ────────────────────────────────────────────────
  db.prepare("CREATE INDEX IF NOT EXISTS idx_sms_phone ON sms_codes(phone, expires_at)").run();
  db.prepare("CREATE INDEX IF NOT EXISTS idx_orders_user ON orders(user_id)").run();
  db.prepare("CREATE INDEX IF NOT EXISTS idx_redemptions_code ON code_redemptions(code)").run();
  db.prepare("CREATE INDEX IF NOT EXISTS idx_devices_user ON user_devices(user_id)").run();
  db.prepare("CREATE INDEX IF NOT EXISTS idx_activation_client ON activation_tickets(client_id, status)").run();

  // ── 迁移：为旧表加新字段（幂等）─────────────────────────
  try { db.prepare("ALTER TABLE users ADD COLUMN auth_mode TEXT NOT NULL DEFAULT 'free'").run(); } catch { /* 已存在 */ }

  // 插入默认客户（幂等）
  db.prepare(`
    INSERT OR IGNORE INTO clients (id, name) VALUES ('default', 'OpenClaw')
  `).run();

  return db;
}

export function getDb() {
  if (!db) throw new Error("数据库未初始化");
  return db;
}
