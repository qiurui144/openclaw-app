import crypto from "crypto";

let cachedToken = null;
let tokenExpiresAt = 0;

/**
 * 获取微信 access_token（自动缓存，2 小时有效）
 * @param {string} appid
 * @param {string} secret
 * @returns {Promise<string>}
 */
export async function getAccessToken(appid, secret) {
  if (cachedToken && Date.now() < tokenExpiresAt) {
    return cachedToken;
  }

  const url = `https://api.weixin.qq.com/cgi-bin/token?grant_type=client_credential&appid=${appid}&secret=${secret}`;
  const resp = await fetch(url);
  const data = await resp.json();

  if (data.errcode) {
    throw new Error(`微信 access_token 获取失败: ${data.errmsg} (${data.errcode})`);
  }

  cachedToken = data.access_token;
  // 提前 5 分钟过期，避免边界条件
  tokenExpiresAt = Date.now() + (data.expires_in - 300) * 1000;
  return cachedToken;
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
 * 验证微信回调签名
 * @param {string} signature - 微信传来的签名
 * @param {string} timestamp
 * @param {string} nonce
 * @param {string} token - 公众号后台配置的 Token
 * @returns {boolean}
 */
export function verifySignature(signature, timestamp, nonce, token) {
  const arr = [token, timestamp, nonce].sort();
  const str = arr.join("");
  const hash = crypto.createHash("sha1").update(str).digest("hex");
  return hash === signature;
}
