import { getDb } from "../db.js";
import { verifyToken } from "../jwt.js";
import { embedWatermark } from "../watermark.js";
import { deriveContentKey, encryptContent, isKeyActive } from "../keymanager.js";

/**
 * 从 JWT 中提取 activation_id（授权码兑换时写入 JWT 的标识）
 * 回退到 sub（用户 ID）以兼容旧 JWT
 */
function getActivationId(decoded) {
  return decoded.activation_id || decoded.sub;
}

/**
 * 检查 JWT claims 是否有权访问指定 Skill
 */
function hasSkillAccess(decoded, slug) {
  const plan = decoded.plan;
  const jwtSkills = decoded.skills || [];
  return (
    plan === "pro_all" ||
    plan === "enterprise" ||
    (plan === "pro_single" && (jwtSkills.includes("*") || jwtSkills.includes(slug)))
  );
}

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

    if (skill.is_paid && !hasSkillAccess(decoded, slug)) {
      ctx.throw(403, "无权访问此 Skill，请升级订阅");
    }

    ctx.set("Cache-Control", "no-cache, no-store, must-revalidate");

    const content = skill.content || "";

    // 非付费内容直接返回明文
    if (!skill.is_paid) {
      ctx.body = content;
      return;
    }

    // 检查该授权的密钥是否仍然活跃
    const activationId = getActivationId(decoded);
    if (!isKeyActive(activationId)) {
      ctx.throw(403, "授权已吊销");
    }

    // 一授权一密钥：从该授权的主密钥派生内容密钥
    const contentKey = deriveContentKey(activationId, slug);
    if (!contentKey) {
      ctx.throw(403, "密钥不可用");
    }

    // 注入水印 → 加密
    const watermarked = embedWatermark(content, activationId);
    const { ciphertext, nonce } = encryptContent(watermarked, contentKey);

    ctx.body = {
      encrypted: true,
      ciphertext,
      nonce,
      activation_id: activationId,
    };
  });

  // 下发解密密钥（一授权一密钥，客户端缓存 30 天）
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

    if (skill.is_paid && !hasSkillAccess(decoded, slug)) {
      ctx.throw(403, "无权访问此 Skill");
    }

    const activationId = getActivationId(decoded);
    const contentKey = deriveContentKey(activationId, slug);
    if (!contentKey) {
      ctx.throw(403, "授权已吊销或密钥不可用");
    }

    ctx.body = { key: contentKey.toString("base64") };
  });

  // 管理接口：添加/更新 Skill
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

    db.prepare(
      "INSERT INTO admin_audit_log (action, detail, ip) VALUES (?, ?, ?)"
    ).run("upsert_skill", `slug=${slug}`, ctx.ip);

    ctx.body = { ok: true };
  });
}
