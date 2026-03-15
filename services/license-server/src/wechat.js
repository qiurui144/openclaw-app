import crypto from "crypto";
import { readFileSync } from "fs";

// 按 appid 缓存 access_token（支持多客户）
const tokenCache = new Map();

/**
 * 获取微信 access_token（按 appid 缓存，2 小时有效）
 * @param {string} appid
 * @param {string} secret
 * @returns {Promise<string>}
 */
export async function getAccessToken(appid, secret) {
  const cached = tokenCache.get(appid);
  if (cached && Date.now() < cached.expiresAt) {
    return cached.token;
  }

  const url = `https://api.weixin.qq.com/cgi-bin/token?grant_type=client_credential&appid=${appid}&secret=${secret}`;
  const resp = await fetch(url);
  const data = await resp.json();

  if (data.errcode) {
    throw new Error(`微信 access_token 获取失败: ${data.errmsg} (${data.errcode})`);
  }

  // 提前 5 分钟过期，避免边界条件
  tokenCache.set(appid, {
    token: data.access_token,
    expiresAt: Date.now() + (data.expires_in - 300) * 1000,
  });
  return data.access_token;
}

/**
 * 创建临时带参二维码（5 分钟有效）
 * @param {string} accessToken
 * @param {string} sceneStr - 场景值字符串（即 ticket ID）
 * @returns {Promise<{ticket: string, url: string, expire_seconds: number}>}
 */
export async function createQrcode(accessToken, sceneStr) {
  const url = `https://api.weixin.qq.com/cgi-bin/qrcode/create?access_token=${accessToken}`;
  const resp = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      expire_seconds: 300,
      action_name: "QR_STR_SCENE",
      action_info: { scene: { scene_str: sceneStr } },
    }),
  });

  const data = await resp.json();
  if (data.errcode) {
    throw new Error(`微信二维码创建失败: ${data.errmsg} (${data.errcode})`);
  }

  return {
    ticket: data.ticket,
    url: `https://mp.weixin.qq.com/cgi-bin/showqrcode?ticket=${encodeURIComponent(data.ticket)}`,
    expire_seconds: 300,
  };
}

/**
 * 解析微信推送的 XML 消息
 * @param {string} xml
 * @returns {object} 解析后的消息对象
 */
export function parseCallbackXml(xml) {
  const result = {};
  const tags = ["ToUserName", "FromUserName", "CreateTime", "MsgType", "Event", "EventKey", "Ticket"];
  for (const tag of tags) {
    const match = xml.match(new RegExp(`<${tag}><!\\[CDATA\\[(.+?)\\]\\]></${tag}>`))
      || xml.match(new RegExp(`<${tag}>(\\d+)</${tag}>`));
    if (match) {
      result[tag] = match[1];
    }
  }
  return result;
}

/**
 * 验证微信回调签名（公众号 / 支付通用）
 * @param {string} signature
 * @param {string} timestamp
 * @param {string} nonce
 * @param {string} token
 * @returns {boolean}
 */
export function verifySignature(signature, timestamp, nonce, token) {
  const arr = [token, timestamp, nonce].sort();
  const str = arr.join("");
  const hash = crypto.createHash("sha1").update(str).digest("hex");
  return hash === signature;
}

// ── 微信支付（Native 支付，扫码付款）───────────────────────

/**
 * 微信支付 Native 下单（生成支付二维码）
 *
 * 需要环境变量：
 *   WXPAY_MCHID       — 商户号
 *   WXPAY_SERIAL_NO    — 证书序列号
 *   WXPAY_PRIVATE_KEY  — 商户 API 私钥路径
 *   WXPAY_APPID        — 关联的公众号 appid
 *   WXPAY_NOTIFY_URL   — 支付回调地址
 *
 * @param {object} params
 * @param {string} params.orderId   — 商户订单号
 * @param {number} params.amount    — 金额（元）
 * @param {string} params.description — 商品描述
 * @returns {Promise<string>} 支付二维码 URL（code_url）
 */
export async function createNativeOrder({ orderId, amount, description }) {
  const mchid = process.env.WXPAY_MCHID;
  const serialNo = process.env.WXPAY_SERIAL_NO;
  const appid = process.env.WXPAY_APPID;
  const notifyUrl = process.env.WXPAY_NOTIFY_URL;

  if (!mchid || !serialNo || !appid) {
    throw new Error("微信支付未配置（需要 WXPAY_MCHID / WXPAY_SERIAL_NO / WXPAY_APPID）");
  }

  const url = "https://api.mch.weixin.qq.com/v3/pay/transactions/native";
  const body = {
    appid,
    mchid,
    description,
    out_trade_no: orderId,
    notify_url: notifyUrl || `https://${process.env.HOST || "license.openclaw.cn"}/api/payments/wx-callback`,
    amount: { total: Math.round(amount * 100), currency: "CNY" },
  };

  const timestamp = Math.floor(Date.now() / 1000).toString();
  const nonceStr = crypto.randomBytes(16).toString("hex");
  const signature = signWxPayRequest("POST", "/v3/pay/transactions/native", timestamp, nonceStr, body);

  const resp = await fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "Authorization": `WECHATPAY2-SHA256-RSA2048 mchid="${mchid}",nonce_str="${nonceStr}",timestamp="${timestamp}",serial_no="${serialNo}",signature="${signature}"`,
    },
    body: JSON.stringify(body),
  });

  const data = await resp.json();
  if (!resp.ok) {
    throw new Error(`微信支付下单失败: ${data.message || JSON.stringify(data)}`);
  }

  return data.code_url;
}

/**
 * 验证微信支付 V3 回调签名
 * @param {string} timestamp   — Wechatpay-Timestamp 头
 * @param {string} nonce       — Wechatpay-Nonce 头
 * @param {string} body        — 原始 body
 * @param {string} signature   — Wechatpay-Signature 头
 * @param {string} wxPublicKey — 微信平台公钥 PEM
 * @returns {boolean}
 */
export function verifyWxPayCallback(timestamp, nonce, body, signature, wxPublicKey) {
  const message = `${timestamp}\n${nonce}\n${body}\n`;
  const verify = crypto.createVerify("RSA-SHA256");
  verify.update(message);
  return verify.verify(wxPublicKey, signature, "base64");
}

/**
 * 解密微信支付回调中的加密数据（AEAD_AES_256_GCM）
 * @param {object} resource — 回调 JSON 中的 resource 字段
 * @param {string} apiV3Key — 商户 APIv3 密钥
 * @returns {object} 解密后的订单数据
 */
export function decryptWxPayResource(resource, apiV3Key) {
  const { ciphertext, nonce: nonceStr, associated_data } = resource;
  const key = Buffer.from(apiV3Key, "utf-8");
  const iv = Buffer.from(nonceStr, "utf-8");
  const encrypted = Buffer.from(ciphertext, "base64");

  // GCM tag = 最后 16 字节
  const tag = encrypted.subarray(encrypted.length - 16);
  const data = encrypted.subarray(0, encrypted.length - 16);

  const decipher = crypto.createDecipheriv("aes-256-gcm", key, iv);
  decipher.setAuthTag(tag);
  if (associated_data) decipher.setAAD(Buffer.from(associated_data, "utf-8"));
  const decrypted = Buffer.concat([decipher.update(data), decipher.final()]);
  return JSON.parse(decrypted.toString("utf-8"));
}

/**
 * 发送微信公众号模板消息（用于下发授权码）
 * @param {string} accessToken
 * @param {string} openid     — 接收者 openid
 * @param {string} templateId — 模板消息 ID
 * @param {object} data       — 模板数据
 * @param {string} [url]      — 点击跳转链接
 */
export async function sendTemplateMessage(accessToken, openid, templateId, data, url) {
  const apiUrl = `https://api.weixin.qq.com/cgi-bin/message/template/send?access_token=${accessToken}`;
  const resp = await fetch(apiUrl, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      touser: openid,
      template_id: templateId,
      url: url || "",
      data,
    }),
  });

  const result = await resp.json();
  if (result.errcode && result.errcode !== 0) {
    throw new Error(`模板消息发送失败: ${result.errmsg} (${result.errcode})`);
  }
  return result;
}

// ── 内部工具 ────────────────────────────────────────────────

/**
 * 微信支付 V3 请求签名
 */
function signWxPayRequest(method, urlPath, timestamp, nonceStr, body) {
  const keyPath = process.env.WXPAY_PRIVATE_KEY;
  if (!keyPath) throw new Error("缺少 WXPAY_PRIVATE_KEY 环境变量");

  const privateKey = readFileSync(keyPath, "utf-8");
  const message = `${method}\n${urlPath}\n${timestamp}\n${nonceStr}\n${JSON.stringify(body)}\n`;

  const sign = crypto.createSign("RSA-SHA256");
  sign.update(message);
  return sign.sign(privateKey, "base64");
}
