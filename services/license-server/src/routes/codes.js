import { randomBytes, randomUUID } from "crypto";
import { getDb } from "../db.js";
import { signToken, verifyToken } from "../jwt.js";

/**
 * 生成授权码格式：OC-XXXX-XXXX-XXXX（大写字母+数字）
 */
function generateActivationCode() {
  const chars = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // 去掉易混淆的 I/O/0/1
  const segments = [];
  for (let s = 0; s < 3; s++) {
    let seg = "";
    const bytes = randomBytes(4);
    for (let i = 0; i < 4; i++) {
      seg += chars[bytes[i] % chars.length];
    }
    segments.push(seg);
  }
  return `OC-${segments.join("-")}`;
}

/**
 * @param {import('koa-router')} router
 */
export function codesRoutes(router) {

  // ── 管理接口：批量生成授权码 ───────────────────────────
  router.post("/admin/codes/generate", async (ctx) => {
    const adminKey = ctx.get("X-Admin-Key");
    if (adminKey !== process.env.ADMIN_KEY) {
      ctx.throw(403, "管理员密钥无效");
    }

    const {
      plan = "pro_all",
      skills = [],
      download_limit = 1,
      count = 1,
      expires_days,
      note,
    } = ctx.request.body;

    if (!["pro_single", "pro_all", "enterprise"].includes(plan)) {
      ctx.throw(400, "plan 必须是 pro_single / pro_all / enterprise");
    }
    if (plan === "pro_single" && (!skills || skills.length === 0)) {
      ctx.throw(400, "pro_single 计划必须指定 skills 列表");
    }
    if (count < 1 || count > 100) {
      ctx.throw(400, "单次生成数量 1~100");
    }
    if (download_limit < 1 || download_limit > 10000) {
      ctx.throw(400, "下载次数限制 1~10000");
    }

    const expiresAt = expires_days
      ? new Date(Date.now() + expires_days * 86400000).toISOString()
      : null;

    const db = getDb();
    const stmt = db.prepare(`
      INSERT INTO activation_codes (code, plan, skills, download_limit, expires_at, note, created_by)
      VALUES (?, ?, ?, ?, ?, ?, ?)
    `);

    const codes = [];
    const createdBy = "admin";

    const insertMany = db.transaction(() => {
      for (let i = 0; i < count; i++) {
        const code = generateActivationCode();
        stmt.run(
          code,
          plan,
          JSON.stringify(plan === "pro_single" ? skills : ["*"]),
          download_limit,
          expiresAt,
          note || null,
          createdBy,
        );
        codes.push(code);
      }
    });

    insertMany();

    ctx.body = {
      ok: true,
      codes,
      plan,
      download_limit,
      expires_at: expiresAt,
    };
  });

  // ── 管理接口：查看授权码列表 ───────────────────────────
  router.get("/admin/codes", async (ctx) => {
    const adminKey = ctx.get("X-Admin-Key");
    if (adminKey !== process.env.ADMIN_KEY) {
      ctx.throw(403, "管理员密钥无效");
    }

    const db = getDb();
    const { status, limit = 50, offset = 0 } = ctx.query;

    let sql = "SELECT * FROM activation_codes";
    const params = [];

    if (status === "available") {
      sql += " WHERE download_used < download_limit AND (expires_at IS NULL OR expires_at > datetime('now'))";
    } else if (status === "exhausted") {
      sql += " WHERE download_used >= download_limit";
    } else if (status === "expired") {
      sql += " WHERE expires_at IS NOT NULL AND expires_at <= datetime('now')";
    }

    sql += " ORDER BY created_at DESC LIMIT ? OFFSET ?";
    params.push(Number(limit), Number(offset));

    const codes = db.prepare(sql).all(...params);
    const total = db.prepare("SELECT COUNT(*) as cnt FROM activation_codes").get();

    ctx.body = { codes, total: total.cnt };
  });

  // ── 管理接口：查看单个授权码详情 + 使用记录 ─────────────
  router.get("/admin/codes/:code", async (ctx) => {
    const adminKey = ctx.get("X-Admin-Key");
    if (adminKey !== process.env.ADMIN_KEY) {
      ctx.throw(403, "管理员密钥无效");
    }

    const db = getDb();
    const codeInfo = db.prepare("SELECT * FROM activation_codes WHERE code = ?")
      .get(ctx.params.code);
    if (!codeInfo) {
      ctx.throw(404, "授权码不存在");
    }

    const redemptions = db.prepare(`
      SELECT r.*, u.phone FROM code_redemptions r
      LEFT JOIN users u ON r.user_id = u.id
      WHERE r.code = ? ORDER BY r.redeemed_at DESC
    `).all(ctx.params.code);

    ctx.body = { ...codeInfo, redemptions };
  });

  // ── 管理接口：作废授权码 ───────────────────────────────
  router.post("/admin/codes/:code/revoke", async (ctx) => {
    const adminKey = ctx.get("X-Admin-Key");
    if (adminKey !== process.env.ADMIN_KEY) {
      ctx.throw(403, "管理员密钥无效");
    }

    const db = getDb();
    const result = db.prepare(
      "UPDATE activation_codes SET download_limit = download_used WHERE code = ?"
    ).run(ctx.params.code);

    if (result.changes === 0) {
      ctx.throw(404, "授权码不存在");
    }

    ctx.body = { ok: true, message: "授权码已作废" };
  });

  // ── 用户接口：兑换授权码 ───────────────────────────────
  router.post("/auth/redeem-code", async (ctx) => {
    const { code, machine_id } = ctx.request.body;
    if (!code) {
      ctx.throw(400, "请输入授权码");
    }

    const db = getDb();

    // 查找授权码
    const codeInfo = db.prepare("SELECT * FROM activation_codes WHERE code = ?")
      .get(code.toUpperCase().trim());

    if (!codeInfo) {
      ctx.throw(404, "授权码无效");
    }

    // 检查是否过期
    if (codeInfo.expires_at && new Date(codeInfo.expires_at) < new Date()) {
      ctx.throw(410, "授权码已过期");
    }

    // 检查下载次数
    if (codeInfo.download_used >= codeInfo.download_limit) {
      ctx.throw(410, "授权码已达使用上限");
    }

    // 创建或获取匿名用户（授权码模式不需要手机号）
    const userId = `code_${randomUUID().replace(/-/g, "").slice(0, 12)}`;

    // 检查此机器是否已用过这个码（同一机器同一码不重复扣次数）
    const existingRedemption = machine_id
      ? db.prepare("SELECT * FROM code_redemptions WHERE code = ? AND machine_id = ?")
          .get(code.toUpperCase().trim(), machine_id)
      : null;

    if (existingRedemption) {
      // 已经兑换过，直接签发新 JWT
      const user = db.prepare("SELECT * FROM users WHERE id = ?")
        .get(existingRedemption.user_id);
      if (user) {
        const token = signToken(user, "*"); // 授权码模式不绑定设备
        ctx.body = { jwt: token, reused: true };
        return;
      }
    }

    // 事务：创建用户 + 消耗次数 + 记录兑换
    const doRedeem = db.transaction(() => {
      db.prepare(`
        INSERT INTO users (id, phone, plan, skills, devices, auth_mode)
        VALUES (?, NULL, ?, ?, 999, 'code')
      `).run(userId, codeInfo.plan, codeInfo.skills);

      db.prepare("UPDATE activation_codes SET download_used = download_used + 1 WHERE code = ?")
        .run(codeInfo.code);

      db.prepare("INSERT INTO code_redemptions (code, user_id, machine_id) VALUES (?, ?, ?)")
        .run(codeInfo.code, userId, machine_id || null);
    });

    doRedeem();

    const user = db.prepare("SELECT * FROM users WHERE id = ?").get(userId);
    // 授权码模式：machine_id = "*"，不绑定设备
    const token = signToken(user, "*");

    ctx.body = {
      jwt: token,
      reused: false,
      remaining: codeInfo.download_limit - codeInfo.download_used - 1,
    };
  });

  // ── 管理接口：更新客户配置（微信公众号等）────────────────
  router.post("/admin/clients", async (ctx) => {
    const adminKey = ctx.get("X-Admin-Key");
    if (adminKey !== process.env.ADMIN_KEY) {
      ctx.throw(403, "管理员密钥无效");
    }

    const { id, name, wechat_appid, wechat_secret, wechat_token, license_api, branding } = ctx.request.body;
    if (!id) {
      ctx.throw(400, "客户 id 为必填");
    }

    const db = getDb();
    db.prepare(`
      INSERT INTO clients (id, name, wechat_appid, wechat_secret, wechat_token, license_api, branding)
      VALUES (?, ?, ?, ?, ?, ?, ?)
      ON CONFLICT(id) DO UPDATE SET
        name = COALESCE(excluded.name, clients.name),
        wechat_appid = COALESCE(excluded.wechat_appid, clients.wechat_appid),
        wechat_secret = COALESCE(excluded.wechat_secret, clients.wechat_secret),
        wechat_token = COALESCE(excluded.wechat_token, clients.wechat_token),
        license_api = COALESCE(excluded.license_api, clients.license_api),
        branding = COALESCE(excluded.branding, clients.branding)
    `).run(id, name || id, wechat_appid || null, wechat_secret || null, wechat_token || null, license_api || null, branding ? JSON.stringify(branding) : null);

    ctx.body = { ok: true };
  });

  // ── 管理接口：查看审计日志 ──────────────────────────────
  router.get("/admin/audit", async (ctx) => {
    const adminKey = ctx.get("X-Admin-Key");
    if (adminKey !== process.env.ADMIN_KEY) {
      ctx.throw(403, "管理员密钥无效");
    }

    const db = getDb();
    const { limit = 50, offset = 0 } = ctx.query;
    const logs = db.prepare(
      "SELECT * FROM admin_audit_log ORDER BY created_at DESC LIMIT ? OFFSET ?"
    ).all(Number(limit), Number(offset));

    ctx.body = { logs };
  });
}
