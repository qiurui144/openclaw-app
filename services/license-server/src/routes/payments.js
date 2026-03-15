import { randomUUID } from "crypto";
import { getDb } from "../db.js";
import { verifyToken } from "../jwt.js";

// 套餐价格表
const PLAN_PRICES = {
  pro_single: null,   // 按具体 skill 定价
  pro_all: 199,       // 年费
  enterprise: 999,    // 年费
};

/**
 * 生成随机授权码（16 位大写字母 + 数字）
 */
function generateActivationCode() {
  const chars = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // 去掉易混淆字符
  let code = "";
  const bytes = new Uint8Array(16);
  globalThis.crypto.getRandomValues(bytes);
  for (const b of bytes) {
    code += chars[b % chars.length];
  }
  // 格式：XXXX-XXXX-XXXX-XXXX
  return `${code.slice(0, 4)}-${code.slice(4, 8)}-${code.slice(8, 12)}-${code.slice(12, 16)}`;
}

/**
 * @param {import('koa-router')} router
 */
export function paymentsRoutes(router) {
  // 创建支付订单（可匿名，无需登录）
  router.post("/payments/create", async (ctx) => {
    const { plan, skill_slug } = ctx.request.body;
    if (!plan || !PLAN_PRICES.hasOwnProperty(plan)) {
      ctx.throw(400, "无效的套餐类型");
    }

    const db = getDb();
    let amount;

    if (plan === "pro_single") {
      if (!skill_slug) {
        ctx.throw(400, "单包购买需要指定 skill_slug");
      }
      const skill = db.prepare("SELECT price FROM skills WHERE slug = ? AND is_paid = 1").get(skill_slug);
      if (!skill) {
        ctx.throw(404, "Skill 不存在或不是付费 Skill");
      }
      amount = skill.price;
    } else {
      amount = PLAN_PRICES[plan];
    }

    const orderId = `ord_${randomUUID().replace(/-/g, "").slice(0, 16)}`;

    // TODO: 接入真实支付网关（微信 Native 支付 / 支付宝当面付）生成真实二维码
    const qrUrl = `https://license.openclaw.cn/pay/${orderId}`;

    db.prepare(`
      INSERT INTO orders (id, user_id, plan, skill_slug, amount, status, qr_url)
      VALUES (?, ?, ?, ?, ?, 'pending', ?)
    `).run(orderId, "anonymous", plan, skill_slug || null, amount, qrUrl);

    ctx.body = {
      order_id: orderId,
      qr_url: qrUrl,
      amount,
      status: "pending",
    };
  });

  // 查询支付状态（含生成的授权码）
  router.get("/payments/:orderId/status", async (ctx) => {
    const { orderId } = ctx.params;
    const db = getDb();
    const order = db.prepare("SELECT * FROM orders WHERE id = ?").get(orderId);

    if (!order) {
      ctx.throw(404, "订单不存在");
    }

    // 超过 30 分钟未支付则过期
    const createdAt = new Date(order.created_at).getTime();
    if (order.status === "pending" && Date.now() - createdAt > 30 * 60 * 1000) {
      db.prepare("UPDATE orders SET status = 'expired' WHERE id = ?").run(orderId);
      ctx.body = { status: "expired" };
      return;
    }

    const result = { status: order.status };

    // 如果已支付，查找关联的授权码
    if (order.status === "paid") {
      const code = db.prepare(
        "SELECT code FROM activation_codes WHERE note = ?"
      ).get(`order:${orderId}`);
      if (code) {
        result.activation_code = code.code;
      }
    }

    ctx.body = result;
  });

  // 支付回调（由支付网关调用，需要验签）
  router.post("/payments/callback", async (ctx) => {
    // TODO: 验证微信/支付宝回调签名
    const { order_id, status } = ctx.request.body;
    if (!order_id || status !== "paid") {
      ctx.throw(400, "无效的回调");
    }

    const db = getDb();
    const order = db.prepare("SELECT * FROM orders WHERE id = ?").get(order_id);
    if (!order) {
      ctx.throw(404, "订单不存在");
    }
    if (order.status !== "pending") {
      ctx.body = { ok: true };
      return;
    }

    // 事务：更新订单 + 生成授权码
    const doComplete = db.transaction(() => {
      db.prepare("UPDATE orders SET status = 'paid', paid_at = datetime('now') WHERE id = ?")
        .run(order_id);

      // 根据订单生成授权码
      const code = generateActivationCode();
      const skills = order.plan === "pro_single" && order.skill_slug
        ? JSON.stringify([order.skill_slug])
        : '["*"]';

      db.prepare(`
        INSERT INTO activation_codes (code, plan, skills, download_limit, note, created_by)
        VALUES (?, ?, ?, ?, ?, ?)
      `).run(code, order.plan, skills, 3, `order:${order_id}`, "payment_system");

      return code;
    });

    const activationCode = doComplete();

    ctx.body = { ok: true, activation_code: activationCode };
  });

  // 开发辅助：模拟支付成功
  router.post("/dev/pay/:orderId", async (ctx) => {
    if (process.env.NODE_ENV === "production") {
      ctx.throw(403, "生产环境不可用");
    }

    const { orderId } = ctx.params;
    const db = getDb();
    const order = db.prepare("SELECT * FROM orders WHERE id = ?").get(orderId);
    if (!order) {
      ctx.throw(404, "订单不存在");
    }

    const doComplete = db.transaction(() => {
      db.prepare("UPDATE orders SET status = 'paid', paid_at = datetime('now') WHERE id = ?")
        .run(orderId);

      const code = generateActivationCode();
      const skills = order.plan === "pro_single" && order.skill_slug
        ? JSON.stringify([order.skill_slug])
        : '["*"]';

      db.prepare(`
        INSERT INTO activation_codes (code, plan, skills, download_limit, note, created_by)
        VALUES (?, ?, ?, ?, ?, ?)
      `).run(code, order.plan, skills, 3, `order:${orderId}`, "dev_payment");

      return code;
    });

    const activationCode = doComplete();
    ctx.body = { ok: true, activation_code: activationCode, message: "模拟支付成功" };
  });
}
