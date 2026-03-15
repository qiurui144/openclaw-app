import { randomUUID } from "crypto";
import { getDb } from "../db.js";
import { signToken, verifyToken } from "../jwt.js";

function generateCode() {
  return String(Math.floor(100000 + Math.random() * 900000));
}

/**
 * 检查设备绑定（仅扫码支付模式）。
 * 授权码模式 (auth_mode='code') 不检查设备。
 * @returns {string|null} 错误信息，null 表示通过
 */
function checkDeviceBinding(db, user, machineId) {
  if (user.auth_mode === "code") return null; // 授权码模式不限设备
  if (user.plan === "free") return null;       // 免费用户不限设备

  // 检查该设备是否已绑定
  const existing = db.prepare(
    "SELECT * FROM user_devices WHERE user_id = ? AND machine_id = ?"
  ).get(user.id, machineId);

  if (existing) return null; // 已绑定，放行

  // 检查已绑定设备数量
  const bound = db.prepare(
    "SELECT COUNT(*) as cnt FROM user_devices WHERE user_id = ?"
  ).get(user.id);

  if (bound.cnt >= user.devices) {
    return `设备数量已达上限（${user.devices} 台），请在管理页面解绑旧设备`;
  }

  // 绑定新设备
  let deviceName = null;
  // hostname 从 machine_id 无法还原，在客户端传过来
  db.prepare(
    "INSERT INTO user_devices (user_id, machine_id, device_name) VALUES (?, ?, ?)"
  ).run(user.id, machineId, deviceName);

  return null;
}

/**
 * @param {import('koa-router')} router
 */
export function authRoutes(router) {

  // ── 发送验证码 ─────────────────────────────────────────
  router.post("/auth/send-code", async (ctx) => {
    const { phone } = ctx.request.body;
    if (!phone || !/^1\d{10}$/.test(phone)) {
      ctx.throw(400, "请输入有效的手机号");
    }

    const code = generateCode();
    const expiresAt = new Date(Date.now() + 5 * 60 * 1000).toISOString();

    const db = getDb();
    db.prepare("INSERT INTO sms_codes (phone, code, expires_at) VALUES (?, ?, ?)")
      .run(phone, code, expiresAt);

    // TODO: 接入真实短信服务（阿里云/腾讯云短信）
    console.log(`[SMS] ${phone} → ${code}（有效期 5 分钟）`);

    ctx.body = { ok: true };
  });

  // ── 手机号登录（扫码支付模式）──────────────────────────
  router.post("/auth/login", async (ctx) => {
    const { phone, code, machine_id, device_name } = ctx.request.body;
    if (!phone || !code || !machine_id) {
      ctx.throw(400, "缺少必填字段");
    }

    const db = getDb();

    // 验证短信码
    const sms = db.prepare(`
      SELECT rowid, * FROM sms_codes
      WHERE phone = ? AND code = ? AND used = 0 AND expires_at > datetime('now')
      ORDER BY expires_at DESC LIMIT 1
    `).get(phone, code);

    if (!sms) {
      ctx.throw(401, "验证码无效或已过期");
    }

    db.prepare("UPDATE sms_codes SET used = 1 WHERE rowid = ?").run(sms.rowid);

    // 查找或创建用户
    let user = db.prepare("SELECT * FROM users WHERE phone = ?").get(phone);
    if (!user) {
      const userId = `user_${randomUUID().replace(/-/g, "").slice(0, 12)}`;
      db.prepare(`
        INSERT INTO users (id, phone, plan, skills, devices, auth_mode)
        VALUES (?, ?, 'free', '[]', 1, 'payment')
      `).run(userId, phone);
      user = db.prepare("SELECT * FROM users WHERE id = ?").get(userId);
    }

    // 扫码支付模式：检查设备绑定
    if (user.auth_mode === "payment" && user.plan !== "free") {
      const err = checkDeviceBinding(db, user, machine_id);
      if (err) ctx.throw(403, err);

      // 更新设备名
      if (device_name) {
        db.prepare(
          "UPDATE user_devices SET device_name = ? WHERE user_id = ? AND machine_id = ?"
        ).run(device_name, user.id, machine_id);
      }
    }

    // 扫码支付模式签发 JWT 时绑定 machine_id
    const token = signToken(user, machine_id);
    ctx.body = { jwt: token };
  });

  // ── 刷新令牌 ──────────────────────────────────────────
  router.post("/auth/refresh", async (ctx) => {
    const authHeader = ctx.get("Authorization");
    if (!authHeader?.startsWith("Bearer ")) {
      ctx.throw(401, "缺少授权头");
    }
    const oldToken = authHeader.slice(7);
    const { machine_id } = ctx.request.body;
    if (!machine_id) {
      ctx.throw(400, "缺少 machine_id");
    }

    let decoded;
    try {
      decoded = verifyToken(oldToken);
    } catch (e) {
      ctx.throw(401, `令牌验证失败：${e.message}`);
    }

    const db = getDb();
    const user = db.prepare("SELECT * FROM users WHERE id = ?").get(decoded.sub);
    if (!user) {
      ctx.throw(404, "用户不存在");
    }

    // 授权码模式继续使用 "*"，扫码支付模式使用实际 machine_id
    const mid = user.auth_mode === "code" ? "*" : machine_id;

    // 扫码支付模式：检查设备绑定
    if (user.auth_mode === "payment" && user.plan !== "free") {
      const err = checkDeviceBinding(db, user, machine_id);
      if (err) ctx.throw(403, err);
    }

    const token = signToken(user, mid);
    ctx.body = { jwt: token };
  });

  // ── 查看已绑定的设备列表（需登录）─────────────────────
  router.get("/auth/devices", async (ctx) => {
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

    const db = getDb();
    const user = db.prepare("SELECT * FROM users WHERE id = ?").get(decoded.sub);
    if (!user) {
      ctx.throw(404, "用户不存在");
    }

    const devices = db.prepare(
      "SELECT id, machine_id, device_name, bound_at FROM user_devices WHERE user_id = ? ORDER BY bound_at DESC"
    ).all(decoded.sub);

    ctx.body = {
      auth_mode: user.auth_mode,
      devices_limit: user.devices,
      devices,
    };
  });

  // ── 用户解绑自己的设备（需登录）───────────────────────
  router.delete("/auth/devices/:deviceId", async (ctx) => {
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

    const db = getDb();
    const result = db.prepare(
      "DELETE FROM user_devices WHERE id = ? AND user_id = ?"
    ).run(ctx.params.deviceId, decoded.sub);

    if (result.changes === 0) {
      ctx.throw(404, "设备记录不存在");
    }

    ctx.body = { ok: true };
  });
}
