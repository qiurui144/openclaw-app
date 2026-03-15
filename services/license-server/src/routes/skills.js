import crypto from "crypto";
import { getDb } from "../db.js";
import { verifyToken } from "../jwt.js";
import { embedWatermark } from "../watermark.js";

const ENCRYPTION_SECRET = process.env.SKILL_ENCRYPTION_SECRET || "openclaw-default-encryption-key-change-me";

/**
 * @param {import('koa-router')} router
 */
export function skillsRoutes(router) {
  // 获取 Skill 索引（公开，不需要登录）
  router.get("/skills/index", async (ctx) => {
    const db = getDb();
    const skills = db.prepare(`
      SELECT slug, name, description, category, is_paid, price, price_label,
             version, author, icon
      FROM skills
      ORDER BY is_paid ASC, name ASC
    `).all();

    ctx.body = skills.map((s) => ({
      ...s,
      is_paid: !!s.is_paid,
    }));
  });

  // 获取付费 Skill 内容（需要 JWT + 权限）
  // 返回加密内容 + 水印标识
  router.get("/skills/:slug/content", async (ctx) => {
    const authHeader = ctx.get("Authorization");
    if (!authHeader?.startsWith("Bearer ")) {
      ctx.throw(401, "请先登录");
    }

    let decoded;
    try {
      decoded = verifyToken(authHeader.slice(7));
    } catch (e) {
      ctx.throw(401, `令牌验证失败：${e.message}`);
    }

    const { slug } = ctx.params;
    const db = getDb();
    const skill = db.prepare("SELECT * FROM skills WHERE slug = ?").get(slug);

    if (!skill) {
      ctx.throw(404, "Skill 不存在");
    }

    if (skill.is_paid) {
      // 直接从 JWT claims 判断权限（无需查用户表）
      const plan = decoded.plan;
      const jwtSkills = decoded.skills || [];
      const hasAccess =
        plan === "pro_all" ||
        plan === "enterprise" ||
        (plan === "pro_single" && (jwtSkills.includes("*") || jwtSkills.includes(slug)));

      if (!hasAccess) {
        ctx.throw(403, "无权访问此 Skill，请升级订阅");
      }
    }

    ctx.set("Cache-Control", "no-cache, no-store, must-revalidate");

    const content = skill.content || "";

    // 非付费内容直接返回明文（向后兼容）
    if (!skill.is_paid) {
      ctx.body = content;
      return;
    }

    // 付费内容：注入水印 → AES-256-GCM 加密
    const watermarked = embedWatermark(content, decoded.sub);
    const { ciphertext, nonce } = encryptContent(watermarked, decoded.sub, slug);

    ctx.body = {
      encrypted: true,
      ciphertext,
      nonce,
      watermark_id: decoded.sub,
    };
  });

  // 获取解密密钥（需要 JWT，临时密钥有效期 24h）
  router.get("/skills/:slug/key", async (ctx) => {
    const authHeader = ctx.get("Authorization");
    if (!authHeader?.startsWith("Bearer ")) {
      ctx.throw(401, "请先登录");
    }

    let decoded;
    try {
      decoded = verifyToken(authHeader.slice(7));
    } catch (e) {
      ctx.throw(401, `令牌验证失败：${e.message}`);
    }

    const { slug } = ctx.params;
    const db = getDb();
    const skill = db.prepare("SELECT is_paid FROM skills WHERE slug = ?").get(slug);
    if (!skill) {
      ctx.throw(404, "Skill 不存在");
    }

    if (skill.is_paid) {
      const plan = decoded.plan;
      const jwtSkills = decoded.skills || [];
      const hasAccess =
        plan === "pro_all" ||
        plan === "enterprise" ||
        (plan === "pro_single" && (jwtSkills.includes("*") || jwtSkills.includes(slug)));
      if (!hasAccess) ctx.throw(403, "无权访问此 Skill");
    }

    const key = deriveKey(decoded.sub, slug);
    ctx.body = { key: key.toString("base64") };
  });

  // 管理接口：添加/更新 Skill（需要管理员密钥）
  router.post("/admin/skills", async (ctx) => {
    const adminKey = ctx.get("X-Admin-Key");
    if (adminKey !== process.env.ADMIN_KEY) {
      ctx.throw(403, "管理员密钥无效");
    }

    const { slug, name, description, category, is_paid, price, price_label, version, author, icon, content } =
      ctx.request.body;

    if (!slug || !name) {
      ctx.throw(400, "slug 和 name 为必填");
    }

    const db = getDb();
    db.prepare(`
      INSERT INTO skills (slug, name, description, category, is_paid, price, price_label, version, author, icon, content)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      ON CONFLICT(slug) DO UPDATE SET
        name = excluded.name,
        description = excluded.description,
        category = excluded.category,
        is_paid = excluded.is_paid,
        price = excluded.price,
        price_label = excluded.price_label,
        version = excluded.version,
        author = excluded.author,
        icon = excluded.icon,
        content = excluded.content,
        updated_at = datetime('now')
    `).run(slug, name, description || "", category || "general", is_paid ? 1 : 0, price || null, price_label || null, version || "1.0.0", author || "OpenClaw", icon || null, content || null);

    // 审计日志
    db.prepare(
      "INSERT INTO admin_audit_log (action, detail, ip) VALUES (?, ?, ?)"
    ).run("upsert_skill", `slug=${slug}`, ctx.ip);

    ctx.body = { ok: true };
  });
}

/**
 * 派生加密密钥：HMAC-SHA256(SERVER_SECRET, user_id + slug)
 * @returns {Buffer} 32 字节密钥
 */
function deriveKey(userId, slug) {
  return crypto
    .createHmac("sha256", ENCRYPTION_SECRET)
    .update(`${userId}:${slug}`)
    .digest();
}

/**
 * AES-256-GCM 加密内容
 * @returns {{ ciphertext: string, nonce: string }} base64 编码
 */
function encryptContent(plaintext, userId, slug) {
  const key = deriveKey(userId, slug);
  const nonce = crypto.randomBytes(12);
  const cipher = crypto.createCipheriv("aes-256-gcm", key, nonce);
  const encrypted = Buffer.concat([cipher.update(plaintext, "utf-8"), cipher.final()]);
  const tag = cipher.getAuthTag();

  return {
    ciphertext: Buffer.concat([encrypted, tag]).toString("base64"),
    nonce: nonce.toString("base64"),
  };
}
