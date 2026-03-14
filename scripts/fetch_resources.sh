#!/usr/bin/env bash
# fetch_resources.sh - 下载 Full Bundle 所需的二进制资源
# 用法:
#   bash scripts/fetch_resources.sh              # 下载全部（Node.js + openclaw.tgz）
#   bash scripts/fetch_resources.sh --node-only  # 仅下载 Node.js
set -euo pipefail

NODE_VERSION="${NODE_VERSION:-22.17.0}"
OPENCLAW_PKG="${OPENCLAW_PKG:-openclaw}"
RESOURCES_DIR="${RESOURCES_DIR:-resources/binaries}"
NPM_REGISTRY="${NPM_REGISTRY:-https://registry.npmmirror.com}"

mkdir -p "$RESOURCES_DIR/linux" "$RESOURCES_DIR/windows"

# ── Node.js ───────────────────────────────────────────────
if [[ ! -f "$RESOURCES_DIR/linux/node" ]]; then
  echo "下载 Linux Node.js ${NODE_VERSION}..."
  curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-x64.tar.xz" \
    | tar -xJ --strip-components=2 -C "$RESOURCES_DIR/linux" "node-v${NODE_VERSION}-linux-x64/bin/node"
  chmod +x "$RESOURCES_DIR/linux/node"
else
  echo "Linux Node.js 已存在，跳过"
fi

if [[ ! -f "$RESOURCES_DIR/windows/node.exe" ]]; then
  echo "下载 Windows Node.js ${NODE_VERSION}..."
  curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-win-x64.zip" \
    -o /tmp/node_win.zip
  unzip -p /tmp/node_win.zip "node-v${NODE_VERSION}-win-x64/node.exe" > "$RESOURCES_DIR/windows/node.exe"
  rm -f /tmp/node_win.zip
else
  echo "Windows Node.js 已存在，跳过"
fi

# ── openclaw 服务包（npm pack tarball）────────────────────
if [[ "${1:-}" != "--node-only" ]]; then
  echo "获取 openclaw 服务包版本信息..."
  OC_VERSION=$(npm view "$OPENCLAW_PKG" version --registry "$NPM_REGISTRY" 2>/dev/null) || \
    OC_VERSION=$(npm view "$OPENCLAW_PKG" version 2>/dev/null) || \
    { echo "错误：无法获取 $OPENCLAW_PKG 版本" >&2; exit 1; }
  echo "最新版本：$OC_VERSION"

  OC_TGZ_URL=$(npm view "$OPENCLAW_PKG@$OC_VERSION" dist.tarball --registry "$NPM_REGISTRY" 2>/dev/null) || \
    OC_TGZ_URL=$(npm view "$OPENCLAW_PKG@$OC_VERSION" dist.tarball 2>/dev/null) || \
    { echo "错误：无法获取下载地址" >&2; exit 1; }

  # openclaw.tgz 是纯 JS 包，Linux 和 Windows 共用
  if [[ ! -f "$RESOURCES_DIR/linux/openclaw.tgz" ]] || [[ $(stat -c%s "$RESOURCES_DIR/linux/openclaw.tgz" 2>/dev/null || echo 0) -lt 1000 ]]; then
    echo "下载 openclaw.tgz ($OC_TGZ_URL)..."
    curl -fL --progress-bar "$OC_TGZ_URL" -o "$RESOURCES_DIR/linux/openclaw.tgz"
    # 复制一份给 Windows（纯 JS，二进制相同）
    cp "$RESOURCES_DIR/linux/openclaw.tgz" "$RESOURCES_DIR/windows/openclaw.tgz"
  else
    echo "openclaw.tgz 已存在（$(du -sh "$RESOURCES_DIR/linux/openclaw.tgz" | cut -f1)），跳过"
  fi
fi

echo ""
echo "=== 资源清单 ==="
ls -lh "$RESOURCES_DIR/linux/node" "$RESOURCES_DIR/linux/openclaw.tgz" \
       "$RESOURCES_DIR/windows/node.exe" "$RESOURCES_DIR/windows/openclaw.tgz" 2>/dev/null || true
echo "================"
