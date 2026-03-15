import { randomUUID } from "crypto";
import { getDb } from "../db.js";
import { signActivationToken } from "../jwt.js";
import { getAccessToken, createQrcode, parseCallbackXml, verifySignature } from "../wechat.js";

/**
 * @param {import('koa-router')} router
 */
export function activationRoutes(router) {

  // ── 生成带参二维码 ──────────────────────────────────────
  router.post("/activation/qrcode", async (ctx) => {
    const { client_id } = ctx.request.body;
    const clientId = client_id || "default";

    const db = getDb();
    const client = db.prepare("SELECT * FROM clients WHERE id = ?").get(clientId);
    if (!client) {
      ctx.throw(400, "无效的客户端 ID");
    }

    if (!client.wechat_appid || !client.wechat_secret) {
      ctx.throw(500, "该客户未配置微信公众号");
    }

    const ticket = `act_${randomUUID().replace(/-/g, "").slice(0, 16)}`;

    // 调用微信 API 生成带参二维码
    const accessToken = await getAccessToken(client.wechat_appid, client.wechat_secret);
    const qr = await createQrcode(accessToken, ticket);

    // 存入数据库
    db.prepare(
      "INSERT INTO activation_tickets (ticket, client_id, status) VALUES (?, ?, 'pending')"
    ).run(ticket, clientId);

    // 清理过期票据（超过 10 分钟的 pending 状态）
    db.prepare(
      "DELETE FROM activation_tickets WHERE status = 'pending' AND created_at < datetime('now', '-10 minutes')"
    ).run();

    ctx.body = {
      ticket,
      qr_url: qr.url,
      expires_in: qr.expire_seconds,
    };
  });

  // ── 轮询激活状态 ────────────────────────────────────────
  router.get("/activation/check", async (ctx) => {
    const { ticket } = ctx.query;
    if (!ticket) {
      ctx.throw(400, "缺少 ticket 参数");
    }

    const db = getDb();
    const row = db.prepare("SELECT * FROM activation_tickets WHERE ticket = ?").get(ticket);
    if (!row) {
      ctx.throw(404, "票据不存在或已过期");
    }

    // 检查是否超时（5 分钟）
    const createdAt = new Date(row.created_at + "Z").getTime();
    if (row.status === "pending" && Date.now() - createdAt > 5 * 60 * 1000) {
      db.prepare("UPDATE activation_tickets SET status = 'expired' WHERE ticket = ?").run(ticket);
      ctx.body = { verified: false, expired: true };
      return;
    }

    if (row.status === "verified") {
      // 生成激活 JWT（365 天有效，仅用于证明已关注公众号）
      const activationToken = signActivationToken(row.openid, row.client_id);
      ctx.body = { verified: true, activation_token: activationToken };
    } else {
      ctx.body = { verified: false, expired: false };
    }
  });

  // ── 微信事件推送回调 ────────────────────────────────────
  // GET: 微信验证服务器配置时的签名校验
  router.get("/wechat/callback", async (ctx) => {
    const { signature, timestamp, nonce, echostr } = ctx.query;
    const clientId = ctx.query.client_id || "default";

    const db = getDb();
    const client = db.prepare("SELECT wechat_token FROM clients WHERE id = ?").get(clientId);
    if (!client || !client.wechat_token) {
      ctx.status = 403;
      ctx.body = "无效客户端";
      return;
    }

    if (verifySignature(signature, timestamp, nonce, client.wechat_token)) {
      ctx.body = echostr;
    } else {
      ctx.status = 403;
      ctx.body = "签名验证失败";
    }
  });

  // POST: 微信事件推送（关注/扫码事件）
  router.post("/wechat/callback", async (ctx) => {
    const clientId = ctx.query.client_id || "default";

    const db = getDb();
    const client = db.prepare("SELECT wechat_token FROM clients WHERE id = ?").get(clientId);
    if (!client || !client.wechat_token) {
      ctx.status = 403;
      ctx.body = "success";
      return;
    }

    // 验证签名
    const { signature, timestamp, nonce } = ctx.query;
    if (!verifySignature(signature, timestamp, nonce, client.wechat_token)) {
      ctx.status = 403;
      ctx.body = "success";
      return;
    }

    // 获取原始 XML（需要 rawBody 中间件）
    const rawBody = ctx.request.rawBody || ctx.request.body;
    const msg = parseCallbackXml(typeof rawBody === "string" ? rawBody : JSON.stringify(rawBody));

    // 处理关注事件或扫码事件
    if (msg.MsgType === "event" && (msg.Event === "subscribe" || msg.Event === "SCAN")) {
      const eventKey = msg.EventKey || "";
      // subscribe 事件的 EventKey 格式为 "qrscene_xxx"
      const ticket = eventKey.startsWith("qrscene_") ? eventKey.slice(8) : eventKey;
      const openid = msg.FromUserName;

      if (ticket && openid) {
        db.prepare(`
          UPDATE activation_tickets
          SET status = 'verified', openid = ?, verified_at = datetime('now')
          WHERE ticket = ? AND status = 'pending'
        `).run(openid, ticket);
      }
    }

    // 微信要求返回 "success"
    ctx.body = "success";
  });
}
