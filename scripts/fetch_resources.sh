#!/usr/bin/env bash
# fetch_resources.sh - 下载 Full Bundle 所需的二进制资源
# 用法:
#   bash scripts/fetch_resources.sh              # 下载全部（Node.js + openclaw fat tarball）
#   bash scripts/fetch_resources.sh --node-only  # 仅下载 Node.js
#
# Full Bundle（离线模式）资源说明：
#   - node:          Node.js 单独二进制（不含 npm，离线模式不需要 npm）
#   - openclaw.tgz:  "fat tarball" = npm 包 + 预装好的 node_modules
#                    解压即可运行，无需 npm install
#   - mihomo:        Mihomo 代理二进制（可选，供 Clash 代理使用）
#
# 在线模式（Online）会下载完整 Node.js 发行版（含 npm）并在线 npm install。
set -euo pipefail

# 跨平台文件大小获取（Linux: stat -c%s, macOS: stat -f%z）
file_size() {
  stat -c%s "$1" 2>/dev/null || stat -f%z "$1" 2>/dev/null || echo 0
}

NODE_VERSION="${NODE_VERSION:-22.17.0}"
OPENCLAW_PKG="${OPENCLAW_PKG:-openclaw}"
RESOURCES_DIR="${RESOURCES_DIR:-resources/binaries}"
NPM_REGISTRY="${NPM_REGISTRY:-https://registry.npmmirror.com}"

mkdir -p "$RESOURCES_DIR/linux" "$RESOURCES_DIR/windows" "$RESOURCES_DIR/macos"

# ── Node.js（单独二进制，离线模式只需要 node 本身）───────────
if [[ ! -f "$RESOURCES_DIR/linux/node" ]] || [[ $(file_size "$RESOURCES_DIR/linux/node") -lt 1000000 ]]; then
  echo "下载 Linux Node.js ${NODE_VERSION}..."
  curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-x64.tar.xz" \
    | tar -xJ --strip-components=2 -C "$RESOURCES_DIR/linux" "node-v${NODE_VERSION}-linux-x64/bin/node"
  chmod +x "$RESOURCES_DIR/linux/node"
else
  echo "Linux Node.js 已存在（$(du -sh "$RESOURCES_DIR/linux/node" | cut -f1)），跳过"
fi

MACOS_NODE_ARCH="${MACOS_NODE_ARCH:-arm64}"
if [[ ! -f "$RESOURCES_DIR/macos/node" ]] || [[ $(file_size "$RESOURCES_DIR/macos/node") -lt 1000000 ]]; then
  NODE_ARCH_SUFFIX="$MACOS_NODE_ARCH"
  if [[ "$MACOS_NODE_ARCH" == "x86_64" ]]; then NODE_ARCH_SUFFIX="x64"; fi
  echo "下载 macOS Node.js ${NODE_VERSION} (${MACOS_NODE_ARCH})..."
  curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-darwin-${NODE_ARCH_SUFFIX}.tar.gz" \
    | tar -xz --strip-components=2 -C "$RESOURCES_DIR/macos" "node-v${NODE_VERSION}-darwin-${NODE_ARCH_SUFFIX}/bin/node"
  chmod +x "$RESOURCES_DIR/macos/node"
else
  echo "macOS Node.js 已存在（$(du -sh "$RESOURCES_DIR/macos/node" | cut -f1)），跳过"
fi

if [[ ! -f "$RESOURCES_DIR/windows/node.exe" ]] || [[ $(file_size "$RESOURCES_DIR/windows/node.exe") -lt 1000000 ]]; then
  echo "下载 Windows Node.js ${NODE_VERSION}..."
  curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-win-x64.zip" \
    -o /tmp/node_win.zip
  unzip -p /tmp/node_win.zip "node-v${NODE_VERSION}-win-x64/node.exe" > "$RESOURCES_DIR/windows/node.exe"
  rm -f /tmp/node_win.zip
else
  echo "Windows Node.js 已存在（$(du -sh "$RESOURCES_DIR/windows/node.exe" | cut -f1)），跳过"
fi

# ── openclaw "fat tarball"（包含预装的 node_modules）────────
if [[ "${1:-}" != "--node-only" ]]; then
  echo "获取 openclaw 服务包版本信息..."
  OC_VERSION=$(npm view "$OPENCLAW_PKG" version --registry "$NPM_REGISTRY" 2>/dev/null) || \
    OC_VERSION=$(npm view "$OPENCLAW_PKG" version 2>/dev/null) || \
    { echo "错误：无法获取 $OPENCLAW_PKG 版本" >&2; exit 1; }
  echo "最新版本：$OC_VERSION"

  OC_TGZ_URL=$(npm view "$OPENCLAW_PKG@$OC_VERSION" dist.tarball --registry "$NPM_REGISTRY" 2>/dev/null) || \
    OC_TGZ_URL=$(npm view "$OPENCLAW_PKG@$OC_VERSION" dist.tarball 2>/dev/null) || \
    { echo "错误：无法获取下载地址" >&2; exit 1; }

  NEEDS_BUILD=0
  if [[ ! -f "$RESOURCES_DIR/linux/openclaw.tgz" ]] || [[ $(file_size "$RESOURCES_DIR/linux/openclaw.tgz") -lt 1000 ]]; then
    NEEDS_BUILD=1
  fi

  if [[ "$NEEDS_BUILD" -eq 1 ]]; then
    echo "构建 fat tarball（包含 node_modules）..."

    FAT_TMP=$(mktemp -d)
    trap 'rm -rf "$FAT_TMP"' EXIT

    # 1. 下载原始 npm tarball 并解压（去掉 package/ 前缀）
    echo "  下载 $OC_TGZ_URL ..."
    curl -fL --progress-bar "$OC_TGZ_URL" -o "$FAT_TMP/raw.tgz"
    mkdir -p "$FAT_TMP/package"
    tar -xzf "$FAT_TMP/raw.tgz" -C "$FAT_TMP/package" --strip-components=1
    rm "$FAT_TMP/raw.tgz"

    # 2. 在解压目录中 npm install 生产依赖（使用国内源）
    echo "  安装生产依赖（npm install --omit=dev --registry=${NPM_REGISTRY}）..."
    (cd "$FAT_TMP/package" && npm install --omit=dev --no-audit --no-fund --registry="$NPM_REGISTRY")

    # 3. 重新打包为 fat tarball（保留 package/ 前缀以兼容 deploy 解压逻辑）
    echo "  打包 fat tarball..."
    (cd "$FAT_TMP" && tar -czf openclaw.tgz package/)

    cp "$FAT_TMP/openclaw.tgz" "$RESOURCES_DIR/linux/openclaw.tgz"
    cp "$FAT_TMP/openclaw.tgz" "$RESOURCES_DIR/windows/openclaw.tgz"
    cp "$FAT_TMP/openclaw.tgz" "$RESOURCES_DIR/macos/openclaw.tgz"
    rm -rf "$FAT_TMP"
    trap - EXIT

    echo "  fat tarball 大小: $(du -sh "$RESOURCES_DIR/linux/openclaw.tgz" | cut -f1)"
  else
    echo "openclaw.tgz 已存在（$(du -sh "$RESOURCES_DIR/linux/openclaw.tgz" | cut -f1)），跳过"
  fi
fi

# ── Mihomo（Clash 代理）────────────────────────────────────
MIHOMO_VERSION="${MIHOMO_VERSION:-1.18.7}"
MIHOMO_BASE_URL="https://github.com/MetaCubeX/mihomo/releases/download/v${MIHOMO_VERSION}"

if [[ ! -f "$RESOURCES_DIR/linux/mihomo" ]] || [[ $(file_size "$RESOURCES_DIR/linux/mihomo") -lt 1000000 ]]; then
  echo "下载 Linux Mihomo ${MIHOMO_VERSION}..."
  curl -fsSL "${MIHOMO_BASE_URL}/mihomo-linux-amd64-v${MIHOMO_VERSION}.gz" -o /tmp/mihomo_linux.gz
  gunzip -f /tmp/mihomo_linux.gz
  mv /tmp/mihomo_linux "$RESOURCES_DIR/linux/mihomo"
  chmod +x "$RESOURCES_DIR/linux/mihomo"
else
  echo "Linux Mihomo 已存在，跳过"
fi

if [[ ! -f "$RESOURCES_DIR/windows/mihomo.exe" ]] || [[ $(file_size "$RESOURCES_DIR/windows/mihomo.exe") -lt 1000000 ]]; then
  echo "下载 Windows Mihomo ${MIHOMO_VERSION}..."
  curl -fsSL "${MIHOMO_BASE_URL}/mihomo-windows-amd64-v${MIHOMO_VERSION}.zip" -o /tmp/mihomo_win.zip
  INNER_WIN_EXE=$(unzip -Z1 /tmp/mihomo_win.zip | grep -i '\.exe$' | head -1) || INNER_WIN_EXE=""
  if [[ -n "$INNER_WIN_EXE" ]]; then
    unzip -p /tmp/mihomo_win.zip "$INNER_WIN_EXE" > "$RESOURCES_DIR/windows/mihomo.exe"
  else
    echo "错误：mihomo Windows ZIP 内未找到 .exe 文件" >&2; exit 1
  fi
  rm -f /tmp/mihomo_win.zip
else
  echo "Windows Mihomo 已存在，跳过"
fi

MACOS_MIHOMO_ARCH="${MACOS_NODE_ARCH:-arm64}"
if [[ "$MACOS_MIHOMO_ARCH" == "x86_64" ]]; then MACOS_MIHOMO_ARCH="amd64"; fi
if [[ ! -f "$RESOURCES_DIR/macos/mihomo" ]] || [[ $(file_size "$RESOURCES_DIR/macos/mihomo") -lt 1000000 ]]; then
  echo "下载 macOS Mihomo ${MIHOMO_VERSION} (${MACOS_MIHOMO_ARCH})..."
  curl -fsSL "${MIHOMO_BASE_URL}/mihomo-darwin-${MACOS_MIHOMO_ARCH}-v${MIHOMO_VERSION}.gz" -o /tmp/mihomo_mac.gz
  gunzip -f /tmp/mihomo_mac.gz
  mv /tmp/mihomo_mac "$RESOURCES_DIR/macos/mihomo"
  chmod +x "$RESOURCES_DIR/macos/mihomo"
else
  echo "macOS Mihomo 已存在，跳过"
fi

echo ""
echo "=== 资源清单 ==="
ls -lh "$RESOURCES_DIR/linux/node" "$RESOURCES_DIR/linux/openclaw.tgz" "$RESOURCES_DIR/linux/mihomo" \
       "$RESOURCES_DIR/windows/node.exe" "$RESOURCES_DIR/windows/openclaw.tgz" "$RESOURCES_DIR/windows/mihomo.exe" \
       "$RESOURCES_DIR/macos/node" "$RESOURCES_DIR/macos/openclaw.tgz" "$RESOURCES_DIR/macos/mihomo" 2>/dev/null || true
echo "================"
