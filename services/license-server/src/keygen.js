/**
 * RSA 密钥对生成工具
 * 用法：node src/keygen.js [输出目录]
 */
import { generateKeyPairSync } from "crypto";
import { writeFileSync, mkdirSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const outputDir = process.argv[2] || resolve(__dirname, "../../src-tauri/keys");

mkdirSync(outputDir, { recursive: true });

const { publicKey, privateKey } = generateKeyPairSync("rsa", {
  modulusLength: 2048,
  publicKeyEncoding: { type: "spki", format: "pem" },
  privateKeyEncoding: { type: "pkcs8", format: "pem" },
});

writeFileSync(resolve(outputDir, "license_pub.pem"), publicKey);
writeFileSync(resolve(outputDir, "license_priv.pem"), privateKey);

console.log(`密钥对已生成到 ${outputDir}/`);
console.log("  license_pub.pem  → 内嵌到客户端 (src-tauri/keys/)");
console.log("  license_priv.pem → 仅部署到授权服务器，勿提交到 git");
