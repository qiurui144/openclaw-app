#!/usr/bin/env bash
# fetch_resources.sh - 下载 Full/Lite Bundle 所需的二进制资源
# 用法:
#   bash scripts/fetch_resources.sh              # Full Bundle（全量预置）
#   bash scripts/fetch_resources.sh --lite       # Lite Bundle（不含 skills 和 skill 依赖工具）
#   bash scripts/fetch_resources.sh --node-only  # 仅下载 Node.js
#
# Full Bundle（离线模式）资源说明：
#   - node:          Node.js 单独二进制（不含 npm，离线模式不需要 npm）
#   - openclaw.tgz:  "fat tarball" = npm 包 + 预装好的 node_modules + qqbot 插件
#   - mihomo:        Mihomo 代理二进制（供 Clash 代理使用）
#   - tools/:        Skills 所需的系统工具静态二进制（jq, rg, gh, ffmpeg）
#
# Lite Bundle：
#   - node + openclaw.tgz + mihomo（不含 skills 预缓存和 tools/）
#
# 在线模式（Online）会下载完整 Node.js 发行版（含 npm）并在线 npm install。
set -euo pipefail

MODE="${1:-full}"  # full | --lite | --node-only

# 跨平台文件大小获取（Linux: stat -c%s, macOS: stat -f%z）
file_size() {
  stat -c%s "$1" 2>/dev/null || stat -f%z "$1" 2>/dev/null || echo 0
}

# 下载辅助：支持重试
download() {
  local url="$1" dest="$2"
  echo "  下载 $url ..."
  curl -fSL --retry 3 --progress-bar "$url" -o "$dest"
}

NODE_VERSION="${NODE_VERSION:-22.17.0}"
OPENCLAW_PKG="${OPENCLAW_PKG:-openclaw}"
RESOURCES_DIR="${RESOURCES_DIR:-resources/binaries}"
NPM_REGISTRY="${NPM_REGISTRY:-https://registry.npmmirror.com}"

# Skills 系统工具版本（定期更新）
JQ_VERSION="${JQ_VERSION:-1.8.1}"
RG_VERSION="${RG_VERSION:-15.1.0}"
GH_VERSION="${GH_VERSION:-2.88.1}"
# ffmpeg 使用 johnvansickle 的静态构建（Linux），BtbN（Windows），evermeet（macOS）

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

[[ "$MODE" == "--node-only" ]] && { echo "Node.js 下载完成（--node-only）"; exit 0; }

# ── openclaw "fat tarball"（包含预装的 node_modules）────────
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

  # 1. 下载原始 npm tarball 并解压
  echo "  下载 $OC_TGZ_URL ..."
  curl -fL --progress-bar "$OC_TGZ_URL" -o "$FAT_TMP/raw.tgz"
  mkdir -p "$FAT_TMP/package"
  tar -xzf "$FAT_TMP/raw.tgz" -C "$FAT_TMP/package" --strip-components=1
  rm "$FAT_TMP/raw.tgz"

  # 2. npm install 生产依赖
  echo "  安装生产依赖..."
  # 优先使用国内源，失败则回落到 npmjs.org（部分包可能有同步延迟）
  (cd "$FAT_TMP/package" && npm install --omit=dev --no-audit --no-fund --registry="$NPM_REGISTRY") || \
  (echo "  npmmirror 安装失败，回落到 npmjs.org..." && \
   cd "$FAT_TMP/package" && npm install --omit=dev --no-audit --no-fund --registry="https://registry.npmjs.org")

  # 2.5. 预置第三方插件（qqbot 等）
  EXTENSIONS_DIR="$FAT_TMP/package/extensions"
  mkdir -p "$EXTENSIONS_DIR"

  QQBOT_PKG="@sliverp/qqbot"
  QQBOT_VER=$(npm view "$QQBOT_PKG" version --registry "$NPM_REGISTRY" 2>/dev/null) || true
  if [ -n "$QQBOT_VER" ]; then
    echo "  预置 qqbot v${QQBOT_VER}..."
    QQBOT_URL=$(npm view "$QQBOT_PKG@$QQBOT_VER" dist.tarball --registry "$NPM_REGISTRY" 2>/dev/null)
    if [ -n "$QQBOT_URL" ]; then
      mkdir -p "$EXTENSIONS_DIR/qqbot"
      curl -fsSL "$QQBOT_URL" | tar -xz -C "$EXTENSIONS_DIR/qqbot" --strip-components=1
      if grep -q '"dependencies"' "$EXTENSIONS_DIR/qqbot/package.json" 2>/dev/null; then
        echo "    安装 qqbot 依赖..."
        (cd "$EXTENSIONS_DIR/qqbot" && npm install --omit=dev --no-audit --no-fund --registry="$NPM_REGISTRY") || \
          echo "    警告: qqbot 依赖安装失败（非致命）"
      fi
    fi
  else
    echo "  警告: 无法获取 qqbot 版本，跳过预置"
  fi

  # 2.6. 为所有 extensions 安装生产依赖
  echo "  检查 extensions 依赖..."
  for ext_dir in "$EXTENSIONS_DIR"/*/; do
    if [ -f "$ext_dir/package.json" ] && [ ! -d "$ext_dir/node_modules" ]; then
      if grep -q '"dependencies"' "$ext_dir/package.json" 2>/dev/null; then
        ext_name=$(basename "$ext_dir")
        echo "    $ext_name..."
        (cd "$ext_dir" && npm install --omit=dev --no-audit --no-fund --registry="$NPM_REGISTRY") || \
          echo "    警告: $ext_name 依赖安装失败（非致命）"
      fi
    fi
  done

  # 3. 打包 fat tarball
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

# ── Skills 系统工具（静态二进制，Full Bundle 专属）──────────────
# Lite 模式跳过 skills 预缓存和系统工具
if [[ "$MODE" == "--lite" ]]; then
  echo ""
  echo "=== Lite Bundle 模式：跳过 Skills 预缓存和系统工具 ==="
else

# -- tools 目录 --
TOOLS_LINUX="$RESOURCES_DIR/linux/tools"
TOOLS_MACOS="$RESOURCES_DIR/macos/tools"
TOOLS_WIN="$RESOURCES_DIR/windows/tools"
mkdir -p "$TOOLS_LINUX" "$TOOLS_MACOS" "$TOOLS_WIN"

# ── jq ──
echo "下载 jq ${JQ_VERSION}..."
JQ_BASE="https://github.com/jqlang/jq/releases/download/jq-${JQ_VERSION}"
[[ ! -f "$TOOLS_LINUX/jq" ]] && {
  download "${JQ_BASE}/jq-linux-amd64" "$TOOLS_LINUX/jq"
  chmod +x "$TOOLS_LINUX/jq"
} || echo "  Linux jq 已存在，跳过"
[[ ! -f "$TOOLS_MACOS/jq" ]] && {
  download "${JQ_BASE}/jq-macos-arm64" "$TOOLS_MACOS/jq"
  chmod +x "$TOOLS_MACOS/jq"
} || echo "  macOS jq 已存在，跳过"
[[ ! -f "$TOOLS_WIN/jq.exe" ]] && {
  download "${JQ_BASE}/jq-windows-amd64.exe" "$TOOLS_WIN/jq.exe"
} || echo "  Windows jq 已存在，跳过"

# ── ripgrep (rg) ──
echo "下载 ripgrep ${RG_VERSION}..."
RG_BASE="https://github.com/BurntSushi/ripgrep/releases/download/${RG_VERSION}"
if [[ ! -f "$TOOLS_LINUX/rg" ]]; then
  download "${RG_BASE}/ripgrep-${RG_VERSION}-x86_64-unknown-linux-musl.tar.gz" /tmp/rg_linux.tar.gz
  tar -xzf /tmp/rg_linux.tar.gz -C /tmp "ripgrep-${RG_VERSION}-x86_64-unknown-linux-musl/rg"
  mv "/tmp/ripgrep-${RG_VERSION}-x86_64-unknown-linux-musl/rg" "$TOOLS_LINUX/rg"
  chmod +x "$TOOLS_LINUX/rg"
  rm -rf /tmp/rg_linux.tar.gz "/tmp/ripgrep-${RG_VERSION}-x86_64-unknown-linux-musl"
else echo "  Linux rg 已存在，跳过"; fi
if [[ ! -f "$TOOLS_MACOS/rg" ]]; then
  download "${RG_BASE}/ripgrep-${RG_VERSION}-aarch64-apple-darwin.tar.gz" /tmp/rg_mac.tar.gz
  tar -xzf /tmp/rg_mac.tar.gz -C /tmp "ripgrep-${RG_VERSION}-aarch64-apple-darwin/rg"
  mv "/tmp/ripgrep-${RG_VERSION}-aarch64-apple-darwin/rg" "$TOOLS_MACOS/rg"
  chmod +x "$TOOLS_MACOS/rg"
  rm -rf /tmp/rg_mac.tar.gz "/tmp/ripgrep-${RG_VERSION}-aarch64-apple-darwin"
else echo "  macOS rg 已存在，跳过"; fi
if [[ ! -f "$TOOLS_WIN/rg.exe" ]]; then
  download "${RG_BASE}/ripgrep-${RG_VERSION}-x86_64-pc-windows-msvc.zip" /tmp/rg_win.zip
  unzip -p /tmp/rg_win.zip "ripgrep-${RG_VERSION}-x86_64-pc-windows-msvc/rg.exe" > "$TOOLS_WIN/rg.exe"
  rm -f /tmp/rg_win.zip
else echo "  Windows rg 已存在，跳过"; fi

# ── GitHub CLI (gh) ──
echo "下载 GitHub CLI ${GH_VERSION}..."
GH_BASE="https://github.com/cli/cli/releases/download/v${GH_VERSION}"
if [[ ! -f "$TOOLS_LINUX/gh" ]]; then
  download "${GH_BASE}/gh_${GH_VERSION}_linux_amd64.tar.gz" /tmp/gh_linux.tar.gz
  tar -xzf /tmp/gh_linux.tar.gz -C /tmp "gh_${GH_VERSION}_linux_amd64/bin/gh"
  mv "/tmp/gh_${GH_VERSION}_linux_amd64/bin/gh" "$TOOLS_LINUX/gh"
  chmod +x "$TOOLS_LINUX/gh"
  rm -rf /tmp/gh_linux.tar.gz "/tmp/gh_${GH_VERSION}_linux_amd64"
else echo "  Linux gh 已存在，跳过"; fi
if [[ ! -f "$TOOLS_MACOS/gh" ]]; then
  download "${GH_BASE}/gh_${GH_VERSION}_macOS_arm64.zip" /tmp/gh_mac.zip
  unzip -p /tmp/gh_mac.zip "gh_${GH_VERSION}_macOS_arm64/bin/gh" > "$TOOLS_MACOS/gh"
  chmod +x "$TOOLS_MACOS/gh"
  rm -f /tmp/gh_mac.zip
else echo "  macOS gh 已存在，跳过"; fi
if [[ ! -f "$TOOLS_WIN/gh.exe" ]]; then
  download "${GH_BASE}/gh_${GH_VERSION}_windows_amd64.zip" /tmp/gh_win.zip
  unzip -p /tmp/gh_win.zip "bin/gh.exe" > "$TOOLS_WIN/gh.exe"
  rm -f /tmp/gh_win.zip
else echo "  Windows gh 已存在，跳过"; fi

# ── ffmpeg（静态构建）──
echo "下载 ffmpeg..."
if [[ ! -f "$TOOLS_LINUX/ffmpeg" ]]; then
  download "https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz" /tmp/ffmpeg_linux.tar.xz
  tar -xJf /tmp/ffmpeg_linux.tar.xz -C /tmp --wildcards "ffmpeg-*-amd64-static/ffmpeg"
  mv /tmp/ffmpeg-*-amd64-static/ffmpeg "$TOOLS_LINUX/ffmpeg"
  chmod +x "$TOOLS_LINUX/ffmpeg"
  rm -rf /tmp/ffmpeg_linux.tar.xz /tmp/ffmpeg-*-amd64-static
else echo "  Linux ffmpeg 已存在，跳过"; fi
if [[ ! -f "$TOOLS_WIN/ffmpeg.exe" ]]; then
  download "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip" /tmp/ffmpeg_win.zip
  unzip -p /tmp/ffmpeg_win.zip "ffmpeg-master-latest-win64-gpl/bin/ffmpeg.exe" > "$TOOLS_WIN/ffmpeg.exe"
  rm -f /tmp/ffmpeg_win.zip
else echo "  Windows ffmpeg 已存在，跳过"; fi
# macOS: ffmpeg 静态构建从 evermeet.cx（仅 x86_64）或用户自行 brew install
if [[ ! -f "$TOOLS_MACOS/ffmpeg" ]]; then
  echo "  macOS ffmpeg: 请通过 brew install ffmpeg 安装（无官方静态 arm64 构建）"
fi

# ── 官方 Skills 预缓存（全量预装）────────────────────────────
SKILLS_DIR="${RESOURCES_DIR}/skills"
if [[ ! -d "$SKILLS_DIR" ]] || [[ $(ls -1 "$SKILLS_DIR" 2>/dev/null | wc -l) -eq 0 ]]; then
  echo "下载官方 Skills 到 ${SKILLS_DIR}..."
  mkdir -p "$SKILLS_DIR"
  if command -v npx &>/dev/null; then
    echo "  尝试通过 npx 下载 Skills..."
    (cd "$SKILLS_DIR" && npx --yes @anthropic-ai/clawhub install --all-official --dir . 2>/dev/null) || \
      echo "  clawhub 不可用，跳过 Skills 预缓存（将在部署时在线安装）"
  else
    echo "  npx 不可用，跳过 Skills 预缓存"
  fi
else
  echo "Skills 已存在（$(ls -1 "$SKILLS_DIR" | wc -l) 个），跳过"
fi

fi  # end of Full Bundle exclusive section

echo ""
echo "=== 资源清单 ==="
echo "--- 核心 ---"
ls -lh "$RESOURCES_DIR/linux/node" "$RESOURCES_DIR/linux/openclaw.tgz" "$RESOURCES_DIR/linux/mihomo" \
       "$RESOURCES_DIR/windows/node.exe" "$RESOURCES_DIR/windows/openclaw.tgz" "$RESOURCES_DIR/windows/mihomo.exe" \
       "$RESOURCES_DIR/macos/node" "$RESOURCES_DIR/macos/openclaw.tgz" "$RESOURCES_DIR/macos/mihomo" 2>/dev/null || true
if [[ -d "$RESOURCES_DIR/linux/tools" ]]; then
  echo "--- Skills 工具 ---"
  ls -lh "$RESOURCES_DIR/linux/tools/"* "$RESOURCES_DIR/macos/tools/"* "$RESOURCES_DIR/windows/tools/"* 2>/dev/null || true
fi
echo "================"
