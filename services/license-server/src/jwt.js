import jwt from "jsonwebtoken";
import { readFileSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));

// 加载 RSA 私钥（与客户端内嵌的公钥配对）
const PRIVATE_KEY_PATH = process.env.PRIVATE_KEY_PATH
  || resolve(__dirname, "../../../src-tauri/keys/license_priv.pem");
const PRIVATE_KEY = readFileSync(PRIVATE_KEY_PATH, "utf-8");

const ISSUER = "license.openclaw.cn";
const TOKEN_EXPIRY_DAYS = 30;

/**
 * 签发 JWT
 * @param {object} user - 用户记录 { id, plan, skills, devices }
 * @param {string} machineId - 客户端机器指纹
 * @returns {string} JWT token
 */
export function signToken(user, machineId) {
  const payload = {
    sub: user.id,
    plan: user.plan,
    skills: JSON.parse(user.skills || "[]"),
    machine_id: machineId,
    devices_limit: user.devices,
  };

  return jwt.sign(payload, PRIVATE_KEY, {
    algorithm: "RS256",
    issuer: ISSUER,
    expiresIn: `${TOKEN_EXPIRY_DAYS}d`,
  });
}

/**
 * 签发激活 JWT（仅证明已关注公众号，有效期 365 天）
 * @param {string} openid - 微信用户 openid
 * @param {string} clientId - 客户端 ID
 * @returns {string} JWT token
 */
export function signActivationToken(openid, clientId) {
  return jwt.sign(
    {
      type: "activation",
      openid,
      client_id: clientId,
    },
    PRIVATE_KEY,
    {
      algorithm: "RS256",
      issuer: ISSUER,
      expiresIn: "365d",
    }
  );
}

/**
 * 验证 JWT（用于 refresh 等场景）
 * @param {string} token
 * @returns {object} decoded payload
 */
export function verifyToken(token) {
  // 使用公钥验证（也可以用私钥中提取公钥）
  const PUBLIC_KEY_PATH = process.env.PUBLIC_KEY_PATH
    || resolve(__dirname, "../../../src-tauri/keys/license_pub.pem");
  const PUBLIC_KEY = readFileSync(PUBLIC_KEY_PATH, "utf-8");

  return jwt.verify(token, PUBLIC_KEY, {
    algorithms: ["RS256"],
    issuer: ISSUER,
    // 刷新时允许已过期的 token（在宽限期内）
    clockTolerance: 7 * 86400,
  });
}
