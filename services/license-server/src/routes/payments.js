import { randomUUID } from "crypto";
import { getDb } from "../db.js";
import {
  createNativeOrder,
  verifyWxPayCallback,
  decryptWxPayResource,
  getAccessToken,
  sendTemplateMessage,
} from "../wechat.js";
import { generateMasterKey } from "../keymanager.js";

// 套餐价格表（元）
const PLAN_PRICES = {
  pro_single: null,   // 按具体 skill 定价
  pro_all: 199,
  enterprise: 999,
};

const PLAN_LABELS = {
  pro_single: "Pro 单包",
  pro_all: "Pro 全包",
  enterprise: "企业版",
};

/**
 * 生成授权码（XXXX-XXXX-XXXX-XXXX）
 */
function generateActivationCode() {
  const chars = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
  let code = "";
  const bytes = new Uint8Array(16);
  globalThis.crypto.getRandomValues(bytes);
  for (const b of bytes) code += chars[b % chars.length];
  return `${code.slice(0, 4)}-${code.slice(4, 8)}-${code.slice(8, 12)}-${code.slice(12, 16)}`;
}

/**
 * 支付完成 → 生成授权码 + 密钥（事务）
 * @returns {{ code: string, activationId: string }}
 */
function completeOrder(db, orderId, order) {
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
    `).run(code, order.plan, skills, 3, `order:${orderId}`, "payment_system");

    return code;
  });

  const code = doComplete();

  // 一授权一生成：为授权码生成独立主密钥（generateMasterKey 内部有事务保护）
  const activationId = `pay_${orderId}`;
  try {
    generateMasterKey(activationId);
  } catch (e) {
    console.error(`[支付] 主密钥生成失败（订单 ${orderId}）: ${e.message}`);
    // 密钥生成失败不回滚订单（授权码已可用于兑换，兑换时会重新生成密钥）
  }

  return { code, activationId };
}

/**
 * @param {import('koa-router')} router
 */
export function paymentsRoutes(router) {

  // ── 创建支付订单 ──────────────────────────────────────────
  router.post("/payments/create", async (ctx) => {
    const { plan, skill_slug, client_id } = ctx.request.body;
    if (!plan || !PLAN_PRICES.hasOwnProperty(plan)) {
      ctx.throw(400, "无效的套餐类型");
    }

    const db = getDb();
    let amount;
    let description;

    if (plan === "pro_single") {
      if (!skill_slug) ctx.throw(400, "单包购买需要指定 skill_slug");
      const skill = db.prepare("SELECT price, name FROM skills WHERE slug = ? AND is_paid = 1").get(skill_slug);
      if (!skill) ctx.throw(404, "Skill 不存在或不是付费 Skill");
      amount = skill.price;
      description = `OpenClaw - ${skill.name}`;
    } else {
      amount = PLAN_PRICES[plan];
      description = `OpenClaw - ${PLAN_LABELS[plan]}`;
    }

    const orderId = `ord_${randomUUID().replace(/-/g, "").slice(0, 16)}`;
    let qrUrl;

    // 尝试创建微信支付订单（配置了 WXPAY_MCHID 时走真实支付）
    if (process.env.WXPAY_MCHID) {
      try {
        qrUrl = await createNativeOrder({ orderId, amount, description });
      } catch (e) {
        console.error(`[支付] 微信下单失败: ${e.message}`);
        ctx.throw(500, `支付创建失败: ${e.message}`);
      }
    } else {
      // 未配置支付网关，生成开发占位 URL
      qrUrl = `https://${process.env.HOST || "license.openclaw.cn"}/pay/${orderId}`;
    }

    db.prepare(`
      INSERT INTO orders (id, user_id, plan, skill_slug, amount, status, qr_url)
      VALUES (?, ?, ?, ?, ?, 'pending', ?)
    `).run(orderId, client_id || "anonymous", plan, skill_slug || null, amount, qrUrl);

    ctx.body = { order_id: orderId, qr_url: qrUrl, amount, status: "pending" };
  });

  // ── 客户端轮询支付状态 ────────────────────────────────────
  // 支付成功后返回授权码，客户端可直接自动填入激活
  router.get("/payments/:orderId/status", async (ctx) => {
    const { orderId } = ctx.params;
    const db = getDb();
    const order = db.prepare("SELECT * FROM orders WHERE id = ?").get(orderId);
    if (!order) ctx.throw(404, "订单不存在");

    // 30 分钟超时
    const createdAt = new Date(order.created_at).getTime();
    if (order.status === "pending" && Date.now() - createdAt > 30 * 60 * 1000) {
      db.prepare("UPDATE orders SET status = 'expired' WHERE id = ?").run(orderId);
      ctx.body = { status: "expired" };
      return;
    }

    const result = { status: order.status };

    if (order.status === "paid") {
      const row = db.prepare("SELECT code FROM activation_codes WHERE note = ?")
        .get(`order:${orderId}`);
      if (row) result.activation_code = row.code;
    }

    ctx.body = result;
  });

  // ── 微信支付 V3 回调 ──────────────────────────────────────
  router.post("/payments/wx-callback", async (ctx) => {
    const wxTimestamp = ctx.get("Wechatpay-Timestamp");
    const wxNonce = ctx.get("Wechatpay-Nonce");
    const wxSignature = ctx.get("Wechatpay-Signature");
    const wxSerial = ctx.get("Wechatpay-Serial");
    const rawBody = typeof ctx.request.body === "string"
      ? ctx.request.body
      : JSON.stringify(ctx.request.body);

    // 签名验证（必须配置微信平台证书，否则拒绝处理）
    if (!process.env.WXPAY_PLATFORM_CERT) {
      console.error("[支付回调] 未配置 WXPAY_PLATFORM_CERT，拒绝处理回调");
      ctx.status = 500;
      ctx.body = { code: "FAIL", message: "支付回调签名验证未配置" };
      return;
    }
    const { readFileSync } = await import("fs");
    const platformCert = readFileSync(process.env.WXPAY_PLATFORM_CERT, "utf-8");
    if (!verifyWxPayCallback(wxTimestamp, wxNonce, rawBody, wxSignature, platformCert)) {
      ctx.status = 401;
      ctx.body = { code: "FAIL", message: "签名验证失败" };
      return;
    }

    const body = typeof ctx.request.body === "object" ? ctx.request.body : JSON.parse(rawBody);

    if (body.event_type !== "TRANSACTION.SUCCESS") {
      ctx.body = { code: "SUCCESS", message: "忽略非成功事件" };
      return;
    }

    // 解密回调数据
    const apiV3Key = process.env.WXPAY_API_V3_KEY;
    if (!apiV3Key) {
      console.error("[支付回调] 缺少 WXPAY_API_V3_KEY");
      ctx.status = 500;
      ctx.body = { code: "FAIL", message: "服务端配置错误" };
      return;
    }

    let txn;
    try {
      txn = decryptWxPayResource(body.resource, apiV3Key);
    } catch (e) {
      console.error(`[支付回调] 解密失败: ${e.message}`);
      ctx.status = 500;
      ctx.body = { code: "FAIL", message: "解密失败" };
      return;
    }

    if (txn.trade_state !== "SUCCESS") {
      ctx.body = { code: "SUCCESS", message: "非成功交易" };
      return;
    }

    const orderId = txn.out_trade_no;
    const db = getDb();
    const order = db.prepare("SELECT * FROM orders WHERE id = ?").get(orderId);

    if (!order) {
      ctx.body = { code: "SUCCESS", message: "订单不存在" };
      return;
    }
    if (order.status !== "pending") {
      ctx.body = { code: "SUCCESS", message: "订单已处理" };
      return;
    }

    // 核心：支付完成 → 生成授权码 + 密钥
    const { code } = completeOrder(db, orderId, order);

    console.log(`[支付] 订单 ${orderId} 支付成功，授权码: ${code}`);

    // 异步推送授权码到买家微信（如果有 openid）
    pushCodeToWechat(db, orderId, code, order).catch(e => {
      console.error(`[支付] 模板消息推送失败: ${e.message}`);
    });

    ctx.body = { code: "SUCCESS", message: "OK" };
  });

  // ── 兼容旧回调格式（需 ADMIN_KEY 鉴权）─────────────────
  router.post("/payments/callback", async (ctx) => {
    const adminKey = ctx.get("X-Admin-Key");
    if (adminKey !== process.env.ADMIN_KEY) {
      ctx.throw(403, "管理员密钥无效");
    }
    const { order_id, status } = ctx.request.body;
    if (!order_id || status !== "paid") ctx.throw(400, "无效的回调");

    const db = getDb();
    const order = db.prepare("SELECT * FROM orders WHERE id = ?").get(order_id);
    if (!order) ctx.throw(404, "订单不存在");
    if (order.status !== "pending") {
      ctx.body = { ok: true };
      return;
    }

    const { code } = completeOrder(db, order_id, order);
    ctx.body = { ok: true, activation_code: code };
  });

  // ── 开发辅助：模拟支付 ────────────────────────────────────
  router.post("/dev/pay/:orderId", async (ctx) => {
    if (process.env.NODE_ENV === "production") ctx.throw(403, "生产环境不可用");

    const { orderId } = ctx.params;
    const db = getDb();
    const order = db.prepare("SELECT * FROM orders WHERE id = ?").get(orderId);
    if (!order) ctx.throw(404, "订单不存在");

    const { code } = completeOrder(db, orderId, order);
    ctx.body = { ok: true, activation_code: code, message: "模拟支付成功" };
  });
}

/**
 * 异步推送授权码到买家微信
 *
 * 需要：
 *   1. 订单创建时记录了买家 openid（通过 user_id 字段或关联 activation_tickets）
 *   2. 公众号已配置模板消息
 *
 * 环境变量：
 *   WXPAY_CODE_TEMPLATE_ID — 授权码下发模板消息 ID
 */
async function pushCodeToWechat(db, orderId, code, order) {
  const templateId = process.env.WXPAY_CODE_TEMPLATE_ID;
  if (!templateId) return;

  // 查找关联的 openid（从 activation_tickets 中通过 order 的 user_id 关联）
  const clientId = order.user_id !== "anonymous" ? order.user_id : "default";
  const client = db.prepare("SELECT * FROM clients WHERE id = ?").get(clientId);
  if (!client?.wechat_appid || !client?.wechat_secret) return;

  // 如果 user_id 是 openid 格式（以 o 开头且长度 28），直接使用
  let openid = null;
  if (order.user_id && order.user_id.startsWith("o") && order.user_id.length >= 28) {
    openid = order.user_id;
  }
  if (!openid) return;

  const accessToken = await getAccessToken(client.wechat_appid, client.wechat_secret);

  await sendTemplateMessage(accessToken, openid, templateId, {
    first: { value: "您的授权码已生成" },
    keyword1: { value: code },
    keyword2: { value: PLAN_LABELS[order.plan] || order.plan },
    keyword3: { value: new Date().toLocaleString("zh-CN") },
    remark: { value: "请在 OpenClaw 部署向导中输入此授权码激活。" },
  });

  console.log(`[支付] 授权码已推送到 ${openid}`);
}
