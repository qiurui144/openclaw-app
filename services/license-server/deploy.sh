#!/usr/bin/env bash
# OpenClaw 授权服务部署脚本
# 用法：bash deploy.sh
set -euo pipefail

DEPLOY_DIR="/opt/openclaw-license"
SERVICE_NAME="openclaw-license"

echo "=== OpenClaw 授权服务部署 ==="
echo ""

# ── 1. 检查环境 ──────────────────────────────────────────────
command -v node >/dev/null || { echo "错误：需要 Node.js >= 18"; exit 1; }
NODE_VER=$(node -v | sed 's/v//' | cut -d. -f1)
if [ "$NODE_VER" -lt 18 ]; then
  echo "错误：Node.js 版本过低（当前 $(node -v)，需要 >= 18）"
  exit 1
fi
echo "✓ Node.js $(node -v)"

# ── 2. 部署文件 ──────────────────────────────────────────────
echo ""
echo "部署到 ${DEPLOY_DIR} ..."
sudo mkdir -p "${DEPLOY_DIR}"
sudo cp -r . "${DEPLOY_DIR}/"
cd "${DEPLOY_DIR}"
sudo chown -R "$(whoami)" "${DEPLOY_DIR}"

# ── 3. 安装依赖 ──────────────────────────────────────────────
echo ""
echo "安装依赖..."
npm install --omit=dev --no-audit --no-fund

# ── 4. 生成密钥（如果不存在）─────────────────────────────────
KEYS_DIR="${DEPLOY_DIR}/keys"
if [ ! -f "${KEYS_DIR}/license_priv.pem" ]; then
  echo ""
  echo "生成 RSA 密钥对..."
  node src/keygen.js "${KEYS_DIR}"
  echo ""
  echo "╔══════════════════════════════════════════════════╗"
  echo "║  重要：将公钥复制到客户端项目                      ║"
  echo "║  cp ${KEYS_DIR}/license_pub.pem                  ║"
  echo "║     → 你的项目/src-tauri/keys/license_pub.pem    ║"
  echo "║  然后重新编译客户端                                ║"
  echo "╚══════════════════════════════════════════════════╝"
else
  echo "✓ RSA 密钥已存在"
fi

# ── 5. 生成管理密钥（如果未设置）─────────────────────────────
if [ -z "${ADMIN_KEY:-}" ]; then
  ADMIN_KEY=$(openssl rand -hex 16)
  echo ""
  echo "╔══════════════════════════════════════════════════╗"
  echo "║  管理密钥（请保存）：                              ║"
  echo "║  ADMIN_KEY=${ADMIN_KEY}                          ║"
  echo "╚══════════════════════════════════════════════════╝"
fi

# ── 6. 写入环境文件 ──────────────────────────────────────────
ENV_FILE="${DEPLOY_DIR}/.env"
if [ ! -f "${ENV_FILE}" ]; then
  cat > "${ENV_FILE}" <<ENVEOF
# OpenClaw 授权服务配置
PORT=3800
ADMIN_KEY=${ADMIN_KEY}
PRIVATE_KEY_PATH=${KEYS_DIR}/license_priv.pem
PUBLIC_KEY_PATH=${KEYS_DIR}/license_pub.pem
DB_PATH=${DEPLOY_DIR}/data/license.db

# 主密钥加密密钥（KEK，保护数据库中的授权密钥）
KEY_ENCRYPTION_KEY=$(openssl rand -hex 32)

# 微信公众号配置（部署后通过 API 或直接编辑数据库配置）
# WECHAT_APPID=
# WECHAT_SECRET=
# WECHAT_TOKEN=
ENVEOF
  echo ""
  echo "✓ 环境文件已写入 ${ENV_FILE}"
else
  echo "✓ 环境文件已存在"
fi

# ── 7. 创建 systemd 服务 ─────────────────────────────────────
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
if [ ! -f "${SERVICE_FILE}" ]; then
  sudo tee "${SERVICE_FILE}" > /dev/null <<SVCEOF
[Unit]
Description=OpenClaw License Server
After=network.target

[Service]
Type=simple
User=$(whoami)
WorkingDirectory=${DEPLOY_DIR}
EnvironmentFile=${DEPLOY_DIR}/.env
ExecStart=$(which node) src/index.js
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
SVCEOF

  sudo systemctl daemon-reload
  sudo systemctl enable "${SERVICE_NAME}"
  echo "✓ systemd 服务已创建"
fi

# ── 8. 启动服务 ──────────────────────────────────────────────
echo ""
sudo systemctl restart "${SERVICE_NAME}"
sleep 1
if systemctl is-active --quiet "${SERVICE_NAME}"; then
  echo "✓ 服务已启动（端口 ${PORT:-3800}）"
else
  echo "✗ 启动失败，查看日志："
  echo "  journalctl -u ${SERVICE_NAME} -n 20"
  exit 1
fi

echo ""
echo "=== 部署完成 ==="
echo ""
echo "下一步："
echo "  1. 配置 Nginx 反代 → https://license.你的域名.com → localhost:${PORT:-3800}"
echo "  2. 配置微信公众号（见下方）"
echo "  3. 添加 Skills 内容"
echo "  4. 生成授权码"
echo ""
echo "管理命令示例："
echo "  # 配置微信公众号"
echo "  curl -X POST http://localhost:${PORT:-3800}/api/admin/clients -H 'X-Admin-Key: ${ADMIN_KEY}' \\"
echo "    -H 'Content-Type: application/json' -d '{\"id\":\"default\",\"wechat_appid\":\"wx...\",\"wechat_secret\":\"...\",\"wechat_token\":\"...\"}'"
echo ""
echo "  # 添加 Skill"
echo "  curl -X POST http://localhost:${PORT:-3800}/api/admin/skills -H 'X-Admin-Key: ${ADMIN_KEY}' \\"
echo "    -H 'Content-Type: application/json' -d '{\"slug\":\"rag\",\"name\":\"RAG 检索\",\"is_paid\":true,\"price\":49,\"content\":\"...\"}'"
echo ""
echo "  # 生成授权码（10 个，pro_all 计划）"
echo "  curl -X POST http://localhost:${PORT:-3800}/api/admin/codes/generate -H 'X-Admin-Key: ${ADMIN_KEY}' \\"
echo "    -H 'Content-Type: application/json' -d '{\"plan\":\"pro_all\",\"count\":10,\"download_limit\":3}'"
echo ""
echo "  # 查看服务日志"
echo "  journalctl -u ${SERVICE_NAME} -f"
