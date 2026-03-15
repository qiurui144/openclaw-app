import jwt from "jsonwebtoken";
import { getPrivateKey, getPublicKey } from "./keymanager.js";

const ISSUER = "license.openclaw.cn";
const TOKEN_EXPIRY_DAYS = 30;

/**
 * 签发许可证 JWT
 * @param {object} user - 用户记录 { id, plan, skills, devices }
 * @param {string} machineId - 客户端机器指纹（授权码模式传 "*"）
 * @returns {string} JWT token
 */
export function signToken(user, machineId) {
  const payload = {
    sub: user.id,
    plan: user.plan,
    skills: JSON.parse(user.skills || "[]"),
    machine_id: machineId,
    devices_limit: user.devices,
    activation_id: user.activation_id || user.id,
  };

  return jwt.sign(payload, getPrivateKey(), {
    algorithm: "RS256",
    issuer: ISSUER,
    expiresIn: `${TOKEN_EXPIRY_DAYS}d`,
  });
}

/**
 * 签发激活 JWT（公众号关注验证，有效期 365 天）
 * @param {string} openid - 微信用户 openid
 * @param {string} clientId - 客户端 ID
 * @returns {string} JWT token
 */
export function signActivationToken(openid, clientId) {
  return jwt.sign(
    { type: "activation", openid, client_id: clientId },
    getPrivateKey(),
    { algorithm: "RS256", issuer: ISSUER, expiresIn: "365d" },
  );
}

/**
 * 验证 JWT
 * @param {string} token
 * @returns {object} decoded payload
 */
export function verifyToken(token) {
  return jwt.verify(token, getPublicKey(), {
    algorithms: ["RS256"],
    issuer: ISSUER,
    clockTolerance: 60,
  });
}

/**
 * 宽松验证 JWT（允许过期 7 天内的 token，仅用于 refresh 场景）
 * @param {string} token
 * @returns {object} decoded payload
 */
export function verifyTokenForRefresh(token) {
  return jwt.verify(token, getPublicKey(), {
    algorithms: ["RS256"],
    issuer: ISSUER,
    clockTolerance: 7 * 86400,
  });
}
