#!/usr/bin/env bash
# build_bundle.sh - 将 Node.js 和 openclaw.tgz 内嵌到 install.command
# 用法: NODE_ARCH=arm64 bash scripts/macos/build_bundle.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OUT_DIR="${OUT_DIR:-dist/macos}"
NODE_VERSION="${NODE_VERSION:-22.17.0}"
NODE_ARCH="${NODE_ARCH:-arm64}"
MIHOMO_VERSION="${MIHOMO_VERSION:-1.18.7}"

# Node.js 使用 x64 而非 x86_64
case "$NODE_ARCH" in
  x86_64) NODE_DL_ARCH="x64" ;;
  *)      NODE_DL_ARCH="$NODE_ARCH" ;;
esac

mkdir -p "$OUT_DIR"

echo "=== OpenClaw macOS Full Bundle 打包 ==="

# Step 1: 下载 Node.js for macOS
NODE_URL="https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-darwin-${NODE_DL_ARCH}.tar.gz"
NODE_TGZ="/tmp/oc_node_macos.tar.gz"
echo "下载 Node.js ${NODE_VERSION} (${NODE_ARCH} -> ${NODE_DL_ARCH})…"
curl -fL "$NODE_URL" -o "$NODE_TGZ"

# 提取 node 二进制
NODE_BIN="/tmp/oc_node_bin"
tar -xzf "$NODE_TGZ" -O "node-v${NODE_VERSION}-darwin-${NODE_DL_ARCH}/bin/node" > "$NODE_BIN"
chmod +x "$NODE_BIN"

# Step 2: 下载 Mihomo for macOS
MIHOMO_ARCH="$NODE_ARCH"
case "$MIHOMO_ARCH" in
  x86_64) MIHOMO_DL_ARCH="amd64" ;;
  arm64)  MIHOMO_DL_ARCH="arm64" ;;
  *)      MIHOMO_DL_ARCH="$MIHOMO_ARCH" ;;
esac
MIHOMO_URL="https://github.com/MetaCubeX/mihomo/releases/download/v${MIHOMO_VERSION}/mihomo-darwin-${MIHOMO_DL_ARCH}-v${MIHOMO_VERSION}.gz"
MIHOMO_BIN="/tmp/oc_mihomo_bin"
echo "下载 Mihomo ${MIHOMO_VERSION} (${MIHOMO_ARCH} -> ${MIHOMO_DL_ARCH})…"
curl -fL "$MIHOMO_URL" -o "${MIHOMO_BIN}.gz"
gunzip -f "${MIHOMO_BIN}.gz"
chmod +x "$MIHOMO_BIN"

# Step 3: 验证 openclaw.tgz 存在（由 CI 预先构建）
OC_TGZ="${OC_TGZ_PATH:-resources/openclaw.tgz}"
[[ -f "$OC_TGZ" ]] || { echo "错误：$OC_TGZ 不存在，请先构建 openclaw 包" >&2; exit 1; }

# Step 4: 合并脚本文件
BUNDLE_OUT="$OUT_DIR/install.command"
echo "合并脚本…"

# 合并顺序：detect.sh -> service.sh -> clash.sh -> install_core.sh
cat \
  "$SCRIPT_DIR/detect.sh" \
  "$SCRIPT_DIR/service.sh" \
  "$SCRIPT_DIR/clash.sh" \
  "$SCRIPT_DIR/install_core.sh" \
  > "$BUNDLE_OUT"

# Step 5: 追加内嵌资源（base64 编码）
echo "编码 Node.js 二进制（$(wc -c < "$NODE_BIN") bytes）…"
echo "编码 openclaw.tgz（$(wc -c < "$OC_TGZ") bytes）…"
echo "编码 Mihomo 二进制（$(wc -c < "$MIHOMO_BIN") bytes）…"
{
  echo ""
  echo "__BUNDLE_DATA__"
  echo "__BUNDLE_node__"
  base64 < "$NODE_BIN"
  echo "__END_BUNDLE_node__"
  echo "__BUNDLE_openclaw.tgz__"
  base64 < "$OC_TGZ"
  echo "__END_BUNDLE_openclaw.tgz__"
  echo "__BUNDLE_mihomo__"
  base64 < "$MIHOMO_BIN"
  echo "__END_BUNDLE_mihomo__"
} >> "$BUNDLE_OUT"

chmod +x "$BUNDLE_OUT"

# 清理
rm -f "$NODE_TGZ" "$NODE_BIN" "$MIHOMO_BIN"

echo "=== 打包完成：$BUNDLE_OUT ==="
echo "大小：$(du -sh "$BUNDLE_OUT" | cut -f1)"

# Step 6: 验证脚本语法
bash -n "$BUNDLE_OUT" && echo "语法检查：通过"
