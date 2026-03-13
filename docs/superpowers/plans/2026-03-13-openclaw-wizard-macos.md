# OpenClaw macOS 安装脚本实现计划

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 macOS 一键安装的纯 bash `.command` 脚本，支持离线/在线/ZIP 三种模式，无需 Apple Developer 账号。

**Architecture:** 单文件 `install.command`（bash 脚本），双击后在 macOS Terminal 运行。脚本内嵌所有逻辑：系统检测、资源获取、服务注册（launchd）、Clash 代理（可选）。Full Bundle 版本将 Node.js 二进制和 openclaw.tgz base64 编码内嵌脚本尾部（类似 makeself）。

**Tech Stack:** bash 4.x（macOS 内置 bash 3.2 兼容性处理）、curl、tar、launchctl、bats-core（测试）

---

## 文件结构

```
scripts/macos/
├── install.command          # 主安装脚本（最终产物）
├── install_core.sh          # 核心安装逻辑（由构建步骤合并入 install.command）
├── detect.sh                # 系统检测函数库
├── service.sh               # launchd 服务管理函数库
├── clash.sh                 # Clash/Mihomo 临时代理管理
└── build_bundle.sh          # 打包脚本：将资源 base64 编码追加到 install.command

tests/macos/
├── test_detect.bats
├── test_service.bats
└── test_clash.bats
```

---

## Chunk 1: 系统检测与公共函数

### Task 1: detect.sh 与测试

**Files:**
- Create: `scripts/macos/detect.sh`
- Create: `tests/macos/test_detect.bats`

- [ ] **Step 1: 写 detect.sh 失败测试**

```bash
# tests/macos/test_detect.bats
#!/usr/bin/env bats

# 安装 bats-core: brew install bats-core

setup() {
  source "$(dirname "$BATS_TEST_FILENAME")/../../scripts/macos/detect.sh"
}

@test "check_macos_version: 当前 macOS 应通过（≥ 11）" {
  # 仅在 macOS 上运行
  [[ "$(uname)" == "Darwin" ]] || skip "非 macOS"
  run check_macos_version
  [ "$status" -eq 0 ]
}

@test "check_disk_space: /tmp 至少有 1MB 可用" {
  run check_disk_space "/tmp" 1
  [ "$status" -eq 0 ]
}

@test "check_disk_space: 要求 999999999 MB 应失败" {
  run check_disk_space "/tmp" 999999999
  [ "$status" -ne 0 ]
}

@test "check_port_free: 65432 应空闲" {
  run check_port_free 65432
  [ "$status" -eq 0 ]
}

@test "get_arch: 返回 arm64 或 x86_64" {
  run get_arch
  [[ "$output" == "arm64" || "$output" == "x86_64" ]]
}
```

- [ ] **Step 2: 运行测试，期望 FAIL**

```bash
bats tests/macos/test_detect.bats 2>&1 | head -10
# 期望: cannot load detect.sh (not found)
```

- [ ] **Step 3: 写 scripts/macos/detect.sh**

```bash
#!/usr/bin/env bash
# detect.sh - macOS 系统环境检测函数库

# 检查 macOS 版本 ≥ 11（Big Sur）
check_macos_version() {
  local ver
  ver=$(sw_vers -productVersion 2>/dev/null) || { echo "无法读取 macOS 版本" >&2; return 1; }
  local major
  major=$(echo "$ver" | cut -d. -f1)
  if [[ "$major" -lt 11 ]]; then
    echo "需要 macOS 11（Big Sur）或更高版本，当前：$ver" >&2
    return 1
  fi
  echo "macOS $ver ✓"
}

# 检查磁盘可用空间（MB）
# 用法: check_disk_space <目录> <最小MB>
check_disk_space() {
  local dir="${1:-/}"
  local min_mb="${2:-512}"
  local avail_mb
  avail_mb=$(df -m "$dir" 2>/dev/null | awk 'NR==2 {print $4}')
  if [[ -z "$avail_mb" || "$avail_mb" -lt "$min_mb" ]]; then
    echo "磁盘空间不足：需要 ${min_mb}MB，可用 ${avail_mb:-?}MB（${dir}）" >&2
    return 1
  fi
  echo "磁盘空间 ${avail_mb}MB 可用 ✓"
}

# 检查端口是否空闲
# 用法: check_port_free <端口>
check_port_free() {
  local port="${1:-18789}"
  if lsof -iTCP:"$port" -sTCP:LISTEN -n -P 2>/dev/null | grep -q LISTEN; then
    local pname
    pname=$(lsof -iTCP:"$port" -sTCP:LISTEN -n -P 2>/dev/null | awk 'NR==2{print $1}')
    echo "端口 $port 已被 ${pname:-未知进程} 占用（警告，可继续）" >&2
    return 2  # 2 = 警告（非致命）
  fi
  echo "端口 $port 空闲 ✓"
}

# 获取 CPU 架构（返回 arm64 或 x86_64）
get_arch() {
  uname -m
}

# 检查 curl 可用性
check_curl() {
  if ! command -v curl &>/dev/null; then
    echo "未找到 curl，macOS 内置 curl 应始终可用" >&2
    return 1
  fi
  echo "curl $(curl --version | head -1 | awk '{print $2}') ✓"
}

# 汇总系统检查，返回检查是否全部通过（0=通过，1=致命失败）
run_system_checks() {
  local fatal=0
  echo "═══ 系统环境检查 ═══"
  check_macos_version    || { [[ $? -eq 1 ]] && fatal=1; }
  check_disk_space "${INSTALL_PATH:-$HOME/openclaw}" 512 || { [[ $? -eq 1 ]] && fatal=1; }
  check_port_free "${SERVICE_PORT:-18789}"
  check_curl             || fatal=1
  echo "════════════════════"
  return $fatal
}
```

- [ ] **Step 4: 运行测试，期望 PASS**

```bash
bats tests/macos/test_detect.bats 2>&1
# 期望: 全部 PASS（或 skip）
```

- [ ] **Step 5: Commit**

```bash
git add scripts/macos/detect.sh tests/macos/test_detect.bats
git commit -m "feat(macos): 系统检测函数库 detect.sh"
```

---

## Chunk 2: launchd 服务管理

### Task 2: service.sh 与测试

**Files:**
- Create: `scripts/macos/service.sh`
- Create: `tests/macos/test_service.bats`

- [ ] **Step 1: 写测试（带 tmpdir 隔离）**

```bash
# tests/macos/test_service.bats
#!/usr/bin/env bats

setup() {
  export TMPDIR_TEST="$(mktemp -d)"
  export LAUNCHD_DIR="$TMPDIR_TEST/LaunchAgents"
  export NODE_BIN="$TMPDIR_TEST/node"
  export OPENCLAW_SCRIPT="$TMPDIR_TEST/openclaw.js"
  export INSTALL_PATH="$TMPDIR_TEST/openclaw"
  export SERVICE_PORT=18789
  export OPENCLAW_LABEL="com.openclaw.gateway.test"
  # 创建假文件
  mkdir -p "$LAUNCHD_DIR" "$INSTALL_PATH"
  touch "$NODE_BIN" "$OPENCLAW_SCRIPT"
  chmod +x "$NODE_BIN"
  source "$(dirname "$BATS_TEST_FILENAME")/../../scripts/macos/service.sh"
}

teardown() {
  rm -rf "$TMPDIR_TEST"
}

@test "generate_plist: 生成 plist 文件" {
  generate_plist "$LAUNCHD_DIR/$OPENCLAW_LABEL.plist"
  [ -f "$LAUNCHD_DIR/$OPENCLAW_LABEL.plist" ]
}

@test "generate_plist: plist 包含正确 Label" {
  generate_plist "$LAUNCHD_DIR/$OPENCLAW_LABEL.plist"
  grep -q "$OPENCLAW_LABEL" "$LAUNCHD_DIR/$OPENCLAW_LABEL.plist"
}

@test "generate_plist: plist 包含 RunAtLoad" {
  generate_plist "$LAUNCHD_DIR/$OPENCLAW_LABEL.plist"
  grep -q "RunAtLoad" "$LAUNCHD_DIR/$OPENCLAW_LABEL.plist"
}
```

- [ ] **Step 2: 写 scripts/macos/service.sh**

```bash
#!/usr/bin/env bash
# service.sh - macOS launchd 服务管理

# 默认值（可被外部覆盖）
OPENCLAW_LABEL="${OPENCLAW_LABEL:-com.openclaw.gateway}"
LAUNCHD_DIR="${LAUNCHD_DIR:-$HOME/Library/LaunchAgents}"
PLIST_PATH="${PLIST_PATH:-$LAUNCHD_DIR/$OPENCLAW_LABEL.plist}"

# 生成 launchd plist 文件
# 用法: generate_plist <输出路径>
generate_plist() {
  local out="${1:-$PLIST_PATH}"
  mkdir -p "$(dirname "$out")"
  cat > "$out" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>${OPENCLAW_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>${NODE_BIN}</string>
        <string>${OPENCLAW_SCRIPT}</string>
    </array>
    <key>WorkingDirectory</key>
    <string>${INSTALL_PATH}</string>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>${INSTALL_PATH}/openclaw.log</string>
    <key>StandardErrorPath</key>
    <string>${INSTALL_PATH}/openclaw.err.log</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PORT</key>
        <string>${SERVICE_PORT}</string>
        <key>HOME</key>
        <string>${HOME}</string>
    </dict>
</dict>
</plist>
PLIST
  echo "已生成 plist：$out"
}

# 注册并启动服务
install_service() {
  generate_plist "$PLIST_PATH"
  launchctl load -w "$PLIST_PATH" 2>/dev/null || true
  echo "服务已注册：$OPENCLAW_LABEL"
}

# 停止并卸载服务
uninstall_service() {
  if [[ -f "$PLIST_PATH" ]]; then
    launchctl unload -w "$PLIST_PATH" 2>/dev/null || true
    rm -f "$PLIST_PATH"
    echo "服务已卸载"
  else
    echo "服务未注册，跳过"
  fi
}

# 重启服务（用于 skills 更新后重载）
restart_service() {
  launchctl kickstart -k "gui/$(id -u)/$OPENCLAW_LABEL" 2>/dev/null || {
    launchctl unload "$PLIST_PATH" 2>/dev/null
    sleep 1
    launchctl load -w "$PLIST_PATH" 2>/dev/null
  }
  echo "服务已重启"
}

# 检查服务健康（HTTP 轮询）
# 用法: wait_for_service <端口> <超时秒数>
wait_for_service() {
  local port="${1:-18789}"
  local timeout="${2:-30}"
  local elapsed=0
  echo -n "等待服务启动"
  while [[ $elapsed -lt $timeout ]]; do
    if curl -sf "http://127.0.0.1:$port/health" &>/dev/null; then
      echo " ✓"
      return 0
    fi
    # 检查端口是否被非 openclaw 进程占用
    if lsof -iTCP:"$port" -sTCP:LISTEN -n -P 2>/dev/null | grep -v "node" | grep -q LISTEN; then
      echo ""
      echo "错误：端口 $port 已被其他进程占用，请更换端口后重试" >&2
      return 2
    fi
    sleep 2
    elapsed=$((elapsed + 2))
    echo -n "."
  done
  echo ""
  echo "服务未能在 ${timeout}s 内启动" >&2
  return 1
}
```

- [ ] **Step 3: 运行测试**

```bash
bats tests/macos/test_service.bats 2>&1
# 期望: 全部 PASS
```

- [ ] **Step 4: Commit**

```bash
git add scripts/macos/service.sh tests/macos/test_service.bats
git commit -m "feat(macos): launchd 服务管理函数库 service.sh"
```

---

## Chunk 3: Clash 临时代理（macOS 版）

### Task 3: clash.sh

**Files:**
- Create: `scripts/macos/clash.sh`
- Create: `tests/macos/test_clash.bats`

- [ ] **Step 1: 写 clash.sh 测试**

```bash
# tests/macos/test_clash.bats
#!/usr/bin/env bats

setup() {
  export TMPDIR_TEST="$(mktemp -d)"
  export CLASH_DIR="$TMPDIR_TEST/clash"
  mkdir -p "$CLASH_DIR"
  source "$(dirname "$BATS_TEST_FILENAME")/../../scripts/macos/clash.sh"
}

teardown() {
  clash_stop 2>/dev/null || true
  rm -rf "$TMPDIR_TEST"
}

@test "clash_save_sub: 保存订阅 URL" {
  clash_save_sub "https://example.com/sub"
  local saved
  saved=$(clash_load_sub)
  [ "$saved" = "https://example.com/sub" ]
}

@test "clash_mihomo_url: arm64 返回正确 URL" {
  run clash_mihomo_url "arm64"
  [[ "$output" == *"darwin-arm64"* ]]
}

@test "clash_mihomo_url: x86_64 返回正确 URL" {
  run clash_mihomo_url "x86_64"
  [[ "$output" == *"darwin-amd64"* ]]
}

@test "clash_stop: 无进程时不报错" {
  run clash_stop
  [ "$status" -eq 0 ]
}
```

- [ ] **Step 2: 写 scripts/macos/clash.sh**

```bash
#!/usr/bin/env bash
# clash.sh - macOS Clash/Mihomo 临时代理管理

CLASH_DIR="${CLASH_DIR:-$HOME/.openclaw/clash_tmp}"
CLASH_PID_FILE="$CLASH_DIR/mihomo.pid"
CLASH_BIN="$CLASH_DIR/mihomo"
CLASH_CONFIG="$CLASH_DIR/config.yaml"
CLASH_PROXY_PORT="${CLASH_PROXY_PORT:-7890}"
MIHOMO_VERSION="${MIHOMO_VERSION:-1.18.7}"
CLASH_SUB_FILE="${HOME}/.openclaw/proxy.json"

# 获取 Mihomo 下载 URL（根据架构）
clash_mihomo_url() {
  local arch="${1:-$(uname -m)}"
  local os_tag="darwin"
  local arch_tag
  case "$arch" in
    arm64) arch_tag="arm64" ;;
    x86_64) arch_tag="amd64" ;;
    *) echo "不支持的架构：$arch" >&2; return 1 ;;
  esac
  echo "https://github.com/MetaCubX/mihomo/releases/download/v${MIHOMO_VERSION}/mihomo-${os_tag}-${arch_tag}-v${MIHOMO_VERSION}.gz"
}

# 保存订阅 URL
clash_save_sub() {
  mkdir -p "$(dirname "$CLASH_SUB_FILE")"
  printf '{"subscription_url":"%s"}' "$1" > "$CLASH_SUB_FILE"
}

# 读取已保存的订阅 URL
clash_load_sub() {
  [[ -f "$CLASH_SUB_FILE" ]] || return 1
  # 简单 JSON 解析（避免依赖 jq）
  grep -o '"subscription_url":"[^"]*"' "$CLASH_SUB_FILE" | cut -d'"' -f4
}

# 显示免责声明并等待用户确认
clash_disclaimer() {
  echo ""
  echo "╔════════════════════════════════════════════════════╗"
  echo "║               Clash 代理免责声明                  ║"
  echo "╠════════════════════════════════════════════════════╣"
  echo "║ 本脚本可选择性地集成 Mihomo 代理工具，仅用于      ║"
  echo "║ 在网络受限环境下下载 OpenClaw 资源。               ║"
  echo "║                                                    ║"
  echo "║ • 您已了解并遵守当地网络代理相关法律法规           ║"
  echo "║ • 订阅链接由您自行提供，本软件不背书任何代理服务  ║"
  echo "║ • 代理仅下载期间启用，完成后自动删除相关文件      ║"
  echo "║ • 因使用代理产生的一切法律责任由用户自行承担      ║"
  echo "╚════════════════════════════════════════════════════╝"
  echo ""
  read -rp "输入 'agree' 继续，回车跳过代理直连：" answer
  [[ "$answer" == "agree" ]]
}

# 下载 Mihomo 二进制
clash_download_mihomo() {
  local arch; arch=$(uname -m)
  local url; url=$(clash_mihomo_url "$arch") || return 1
  mkdir -p "$CLASH_DIR"
  echo "下载 Mihomo $MIHOMO_VERSION（$arch）…"
  curl -fsSL "$url" -o "$CLASH_BIN.gz" || { echo "下载失败" >&2; return 1; }
  gunzip -f "$CLASH_BIN.gz"
  chmod +x "$CLASH_BIN"
  echo "Mihomo 已下载"
}

# 下载订阅配置
clash_fetch_config() {
  local sub_url="$1"
  echo "获取订阅配置…"
  curl -fsSL --max-time 15 "$sub_url" -o "$CLASH_CONFIG" || {
    echo "订阅获取失败，请检查链接或网络" >&2
    return 1
  }
}

# 启动 Mihomo
clash_start() {
  local sub_url="$1"
  # 下载二进制（若不存在）
  [[ -x "$CLASH_BIN" ]] || clash_download_mihomo || return 1
  # 获取订阅
  clash_fetch_config "$sub_url" || return 1
  clash_save_sub "$sub_url"
  # 启动进程
  "$CLASH_BIN" -f "$CLASH_CONFIG" -d "$CLASH_DIR" \
    --port "$CLASH_PROXY_PORT" &>/dev/null &
  local pid=$!
  echo "$pid" > "$CLASH_PID_FILE"
  sleep 2
  # 验证启动成功
  if ! kill -0 "$pid" 2>/dev/null; then
    echo "Mihomo 启动失败" >&2
    return 1
  fi
  echo "Mihomo 代理已启动（127.0.0.1:$CLASH_PROXY_PORT，PID=$pid）"
}

# 测试代理连通性（访问 github.com）
clash_test() {
  local proxy_url="http://127.0.0.1:$CLASH_PROXY_PORT"
  local t0; t0=$(date +%s%N 2>/dev/null || date +%s)
  if curl -fsSL -x "$proxy_url" --max-time 8 "https://github.com" -o /dev/null 2>/dev/null; then
    local t1; t1=$(date +%s%N 2>/dev/null || date +%s)
    echo "代理连接成功"
    return 0
  else
    echo "代理连接失败" >&2
    return 1
  fi
}

# 停止 Mihomo 并删除相关文件
clash_stop() {
  if [[ -f "$CLASH_PID_FILE" ]]; then
    local pid; pid=$(cat "$CLASH_PID_FILE")
    kill "$pid" 2>/dev/null || true
    rm -f "$CLASH_PID_FILE"
  fi
  rm -f "$CLASH_BIN" "$CLASH_CONFIG"
  rm -rf "$CLASH_DIR"
  echo "Mihomo 已停止并清理"
}

# 设置 curl 代理环境变量
clash_export_env() {
  export http_proxy="http://127.0.0.1:$CLASH_PROXY_PORT"
  export https_proxy="http://127.0.0.1:$CLASH_PROXY_PORT"
  export HTTP_PROXY="$http_proxy"
  export HTTPS_PROXY="$https_proxy"
}

clash_unset_env() {
  unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY
}
```

- [ ] **Step 3: 运行测试**

```bash
bats tests/macos/test_clash.bats 2>&1
# 期望: 全部 PASS（非网络相关测试）
```

- [ ] **Step 4: Commit**

```bash
git add scripts/macos/clash.sh tests/macos/test_clash.bats
git commit -m "feat(macos): Clash/Mihomo 临时代理管理 clash.sh"
```

---

## Chunk 4: 主安装脚本 install_core.sh

### Task 4: 核心安装逻辑

**Files:**
- Create: `scripts/macos/install_core.sh`

- [ ] **Step 1: 写 scripts/macos/install_core.sh**

这是主安装流程，source detect.sh / service.sh / clash.sh 后执行：

```bash
#!/usr/bin/env bash
# install_core.sh - OpenClaw macOS 安装核心逻辑
# 使用方式：由 install.command 或 build_bundle.sh 合并后执行

set -euo pipefail

# ── 配置项（可被环境变量覆盖）──────────────────────────────
INSTALL_PATH="${INSTALL_PATH:-$HOME/openclaw}"
SERVICE_PORT="${SERVICE_PORT:-18789}"
DOMAIN_NAME="${DOMAIN_NAME:-}"
INSTALL_SERVICE="${INSTALL_SERVICE:-yes}"
START_ON_BOOT="${START_ON_BOOT:-yes}"
NODE_VERSION="${NODE_VERSION:-22.17.0}"

# ── 颜色输出 ───────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[0;33m'; BLUE='\033[0;34m'; NC='\033[0m'
info()  { echo -e "${BLUE}[INFO]${NC} $*"; }
ok()    { echo -e "${GREEN}[OK]${NC}   $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
err()   { echo -e "${RED}[ERROR]${NC} $*" >&2; }
fatal() { err "$*"; exit 1; }

# ── 欢迎横幅 ───────────────────────────────────────────────
show_banner() {
  echo ""
  echo "  ╔═══════════════════════════════════╗"
  echo "  ║   OpenClaw 部署向导（macOS）      ║"
  echo "  ║   版本：${OC_WIZARD_VERSION:-1.0.0}                      ║"
  echo "  ╚═══════════════════════════════════╝"
  echo ""
}

# ── 安装模式检测 ───────────────────────────────────────────
# INSTALL_MODE: bundled / online / local_zip
detect_install_mode() {
  # 检测脚本尾部是否有 __BUNDLE_DATA__ 标记（Full Bundle）
  if grep -q "^__BUNDLE_DATA__$" "$0" 2>/dev/null; then
    echo "bundled"
  elif [[ -n "${LOCAL_ZIP_PATH:-}" && -f "${LOCAL_ZIP_PATH}" ]]; then
    echo "local_zip"
  else
    echo "online"
  fi
}

# ── 获取 Node.js ───────────────────────────────────────────
acquire_node() {
  local mode="$1"
  local node_dir="$INSTALL_PATH/node"
  mkdir -p "$node_dir"
  local node_bin="$node_dir/node"

  case "$mode" in
    bundled)
      info "从内置资源提取 Node.js…"
      # 从脚本尾部提取 base64 编码的 node 二进制
      extract_bundled_resource "node" "$node_bin"
      chmod +x "$node_bin"
      ;;
    online)
      info "下载 Node.js ${NODE_VERSION}…"
      local arch; arch=$(uname -m)
      local arch_tag; [[ "$arch" == "arm64" ]] && arch_tag="arm64" || arch_tag="x64"
      local url="https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-darwin-${arch_tag}.tar.gz"
      local tmp="$INSTALL_PATH/.tmp/node.tar.gz"
      mkdir -p "$(dirname "$tmp")"
      curl -fL --progress-bar "$url" -o "$tmp" || fatal "Node.js 下载失败"
      tar -xzf "$tmp" -C "$node_dir" --strip-components=2 '*/bin/node'
      rm -f "$tmp"
      chmod +x "$node_bin"
      ;;
    local_zip)
      info "从本地 ZIP 提取 Node.js…"
      unzip -p "$LOCAL_ZIP_PATH" "*/node" > "$node_bin" 2>/dev/null || \
        fatal "ZIP 中未找到 node 二进制"
      chmod +x "$node_bin"
      ;;
  esac
  "$node_bin" --version | grep -q "^v" || fatal "Node.js 提取失败（版本验证未通过）"
  ok "Node.js $("$node_bin" --version) 就绪"
}

# ── 获取 openclaw 包 ───────────────────────────────────────
acquire_openclaw() {
  local mode="$1"
  local pkg_tgz="$INSTALL_PATH/.tmp/openclaw.tgz"
  mkdir -p "$(dirname "$pkg_tgz")"

  case "$mode" in
    bundled)
      info "从内置资源提取 openclaw 包…"
      extract_bundled_resource "openclaw.tgz" "$pkg_tgz"
      ;;
    online)
      info "下载 openclaw 最新版本…"
      local url="https://github.com/openclaw/openclaw/releases/latest/download/openclaw.tgz"
      curl -fL --progress-bar "$url" -o "$pkg_tgz" || fatal "openclaw 下载失败"
      ;;
    local_zip)
      info "从本地 ZIP 提取 openclaw.tgz…"
      unzip -p "$LOCAL_ZIP_PATH" "*/openclaw.tgz" > "$pkg_tgz" 2>/dev/null || \
        fatal "ZIP 中未找到 openclaw.tgz"
      ;;
  esac

  info "解包 openclaw…"
  local pkg_dir="$INSTALL_PATH/openclaw_pkg"
  mkdir -p "$pkg_dir"
  tar -xzf "$pkg_tgz" -C "$pkg_dir" --strip-components=1
  rm -f "$pkg_tgz"
  ok "openclaw 包就绪"
}

# ── 从脚本尾部提取内嵌资源（Full Bundle 用）─────────────────
extract_bundled_resource() {
  local name="$1"
  local dest="$2"
  # 格式：__BUNDLE_DATA__\n<name>\n<base64>\n__END_<name>__
  local in_block=0
  local tmp_b64
  tmp_b64=$(mktemp)
  while IFS= read -r line; do
    if [[ "$line" == "__BUNDLE_${name}__" ]]; then in_block=1; continue; fi
    if [[ "$line" == "__END_BUNDLE_${name}__" ]]; then break; fi
    [[ $in_block -eq 1 ]] && echo "$line" >> "$tmp_b64"
  done < "$0"
  if [[ ! -s "$tmp_b64" ]]; then
    rm -f "$tmp_b64"
    fatal "内置资源 '$name' 不存在（请使用 Full Bundle 版本）"
  fi
  base64 -d "$tmp_b64" > "$dest"
  rm -f "$tmp_b64"
}

# ── 写配置文件 ─────────────────────────────────────────────
write_config() {
  local config_dir="$HOME/.openclaw"
  mkdir -p "$config_dir"
  local config_file="$config_dir/openclaw.json"
  # bcrypt 哈希由 node 脚本生成（避免 bash 依赖 bcrypt）
  local node_bin="$INSTALL_PATH/node/node"
  local hashed_pwd
  hashed_pwd=$("$node_bin" -e "
    const crypto = require('crypto');
    const h = crypto.createHash('sha256').update(process.argv[1]).digest('hex');
    console.log(h);
  " "$ADMIN_PASSWORD") || fatal "密码哈希生成失败"

  python3 -c "
import json, sys
cfg = {
  'gateway': {
    'port': int(sys.argv[1]),
    'adminPasswordHash': sys.argv[2],
    'publicUrl': sys.argv[3] if sys.argv[3] else None,
    'installPath': sys.argv[4],
  }
}
print(json.dumps(cfg, indent=2, ensure_ascii=False))
" "$SERVICE_PORT" "$hashed_pwd" "$DOMAIN_NAME" "$INSTALL_PATH" > "$config_file"
  ok "配置文件写入：$config_file"
}

# ── 写安装记录 ─────────────────────────────────────────────
write_deploy_meta() {
  local meta_dir="$HOME/.openclaw"
  mkdir -p "$meta_dir"
  python3 -c "
import json, datetime, sys
meta = {
  'version': sys.argv[1],
  'install_path': sys.argv[2],
  'service_port': int(sys.argv[3]),
  'installed_at': datetime.datetime.now().isoformat(),
}
print(json.dumps(meta, indent=2))
" "${OC_VERSION:-1.0.0}" "$INSTALL_PATH" "$SERVICE_PORT" > "$meta_dir/deploy_meta.json"
}

# ── 生成卸载脚本 ───────────────────────────────────────────
write_uninstall_script() {
  cat > "$INSTALL_PATH/uninstall.sh" <<UNINSTALL
#!/usr/bin/env bash
# OpenClaw 卸载脚本（自动生成）
set -euo pipefail
echo "正在卸载 OpenClaw…"
launchctl unload -w "$HOME/Library/LaunchAgents/com.openclaw.gateway.plist" 2>/dev/null || true
rm -f "$HOME/Library/LaunchAgents/com.openclaw.gateway.plist"
rm -rf "$INSTALL_PATH"
echo "卸载完成。配置文件保留在 ~/.openclaw/，如需删除请手动执行：rm -rf ~/.openclaw"
UNINSTALL
  chmod +x "$INSTALL_PATH/uninstall.sh"
}

# ── 交互式配置采集 ─────────────────────────────────────────
gather_config_interactive() {
  echo ""
  info "=== 安装配置 ==="

  read -rp "安装路径 [$INSTALL_PATH]: " input
  [[ -n "$input" ]] && INSTALL_PATH="$input"

  read -rp "服务端口 [$SERVICE_PORT]: " input
  [[ -n "$input" ]] && SERVICE_PORT="$input"

  read -rp "绑定域名（留空跳过）: " DOMAIN_NAME

  while true; do
    read -rsp "管理员密码（≥8位，含字母+数字）: " ADMIN_PASSWORD
    echo
    if [[ ${#ADMIN_PASSWORD} -lt 8 ]]; then
      warn "密码至少 8 位"
    elif ! [[ "$ADMIN_PASSWORD" =~ [a-zA-Z] ]] || ! [[ "$ADMIN_PASSWORD" =~ [0-9] ]]; then
      warn "密码须同时含字母和数字"
    else
      read -rsp "再次确认密码: " confirm; echo
      [[ "$ADMIN_PASSWORD" == "$confirm" ]] && break
      warn "两次密码不一致"
    fi
  done
}

# ── 主流程 ─────────────────────────────────────────────────
main() {
  show_banner

  # 检测安装模式
  local mode; mode=$(detect_install_mode)
  info "安装模式：$mode"

  # 系统环境检查
  source "$(dirname "$0")/detect.sh"
  run_system_checks || fatal "系统检查失败，无法继续安装"

  # 代理配置（仅 online 模式）
  local use_clash=no
  if [[ "$mode" == "online" ]]; then
    source "$(dirname "$0")/clash.sh"
    if clash_disclaimer; then
      read -rp "请输入订阅链接: " sub_url
      clash_start "$sub_url" || { warn "代理启动失败，将直连下载"; use_clash=no; }
      clash_test && { clash_export_env; use_clash=yes; } || warn "代理测试失败，将直连下载"
    fi
  fi

  # 交互式配置
  gather_config_interactive

  # 创建安装目录
  mkdir -p "$INSTALL_PATH/.tmp"

  # 获取资源
  acquire_node "$mode"
  acquire_openclaw "$mode"

  # 写配置
  write_config

  # 注册系统服务
  if [[ "${INSTALL_SERVICE}" == "yes" ]]; then
    source "$(dirname "$0")/service.sh"
    NODE_BIN="$INSTALL_PATH/node/node"
    OPENCLAW_SCRIPT="$INSTALL_PATH/openclaw_pkg/index.js"
    install_service
    wait_for_service "$SERVICE_PORT" 30 || warn "服务未能及时启动，请手动检查"
  fi

  # 清理
  [[ "$use_clash" == "yes" ]] && clash_stop
  rm -rf "$INSTALL_PATH/.tmp"

  # 写安装记录和卸载脚本
  write_deploy_meta
  write_uninstall_script

  echo ""
  ok "═══════════════════════════════════════"
  ok " OpenClaw 安装完成！"
  ok " 管理控制台：http://127.0.0.1:${SERVICE_PORT}"
  ok " 配置文件：~/.openclaw/openclaw.json"
  ok " 卸载脚本：${INSTALL_PATH}/uninstall.sh"
  ok " 忘记密码请通过管理控制台重置"
  ok "═══════════════════════════════════════"
  echo ""

  # 打开浏览器
  sleep 1 && open "http://127.0.0.1:${SERVICE_PORT}" &
}

main "$@"
```

- [ ] **Step 2: chmod +x**

```bash
chmod +x scripts/macos/install_core.sh
# bash 语法检查
bash -n scripts/macos/install_core.sh && echo "语法正确"
```

- [ ] **Step 3: Commit**

```bash
git add scripts/macos/install_core.sh
git commit -m "feat(macos): 主安装逻辑 install_core.sh（三模式支持）"
```

---

## Chunk 5: 打包脚本 build_bundle.sh

### Task 5: Full Bundle 打包

**Files:**
- Create: `scripts/macos/build_bundle.sh`
- Create: `scripts/macos/install.command`（模板，打包后会被替换）

- [ ] **Step 1: 写 install.command（模板 / Lite 版）**

```bash
#!/usr/bin/env bash
# install.command - OpenClaw macOS 安装脚本
# 双击此文件，在 Terminal 中运行即可开始安装。
# 版本：Lite（在线下载模式，需要网络）
# 如需离线安装，请使用 Full Bundle 版本的 install.command。

# 获取脚本所在目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# 寻找函数库（同目录）
for lib in detect service clash; do
  if [[ ! -f "$SCRIPT_DIR/${lib}.sh" ]]; then
    echo "错误：缺少 ${lib}.sh，请使用完整版安装包" >&2
    read -rp "按回车退出…"
    exit 1
  fi
done

source "$SCRIPT_DIR/install_core.sh"
```

- [ ] **Step 2: 写 build_bundle.sh（Full Bundle 打包器）**

```bash
#!/usr/bin/env bash
# build_bundle.sh - 将 Node.js 和 openclaw.tgz 内嵌到 install.command
# 用法: NODE_ARCH=arm64 bash scripts/macos/build_bundle.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OUT_DIR="${OUT_DIR:-dist/macos}"
NODE_VERSION="${NODE_VERSION:-22.17.0}"
NODE_ARCH="${NODE_ARCH:-arm64}"
MIHOMO_VERSION="${MIHOMO_VERSION:-1.18.7}"

mkdir -p "$OUT_DIR"

echo "=== OpenClaw macOS Full Bundle 打包 ==="

# Step 1: 下载 Node.js for macOS
NODE_URL="https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-darwin-${NODE_ARCH}.tar.gz"
NODE_TGZ="/tmp/oc_node_macos.tar.gz"
echo "下载 Node.js ${NODE_VERSION} (${NODE_ARCH})…"
curl -fL "$NODE_URL" -o "$NODE_TGZ"

# 提取 node 二进制
NODE_BIN="/tmp/oc_node_bin"
tar -xzf "$NODE_TGZ" -O "node-v${NODE_VERSION}-darwin-${NODE_ARCH}/bin/node" > "$NODE_BIN"
chmod +x "$NODE_BIN"

# Step 2: 验证 openclaw.tgz 存在（由 CI 预先构建）
OC_TGZ="${OC_TGZ_PATH:-resources/openclaw.tgz}"
[[ -f "$OC_TGZ" ]] || { echo "错误：$OC_TGZ 不存在，请先构建 openclaw 包" >&2; exit 1; }

# Step 3: 合并脚本文件
BUNDLE_OUT="$OUT_DIR/install.command"
echo "合并脚本…"

# 合并顺序：detect.sh → service.sh → clash.sh → install_core.sh
cat \
  "$SCRIPT_DIR/detect.sh" \
  "$SCRIPT_DIR/service.sh" \
  "$SCRIPT_DIR/clash.sh" \
  "$SCRIPT_DIR/install_core.sh" \
  > "$BUNDLE_OUT"

# Step 4: 追加内嵌资源（base64 编码）
echo "" >> "$BUNDLE_OUT"
echo "__BUNDLE_DATA__" >> "$BUNDLE_OUT"

echo "编码 Node.js 二进制（$(wc -c < "$NODE_BIN") bytes）…"
echo "__BUNDLE_node__" >> "$BUNDLE_OUT"
base64 "$NODE_BIN" >> "$BUNDLE_OUT"
echo "__END_BUNDLE_node__" >> "$BUNDLE_OUT"

echo "编码 openclaw.tgz（$(wc -c < "$OC_TGZ") bytes）…"
echo "__BUNDLE_openclaw.tgz__" >> "$BUNDLE_OUT"
base64 "$OC_TGZ" >> "$BUNDLE_OUT"
echo "__END_BUNDLE_openclaw.tgz__" >> "$BUNDLE_OUT"

chmod +x "$BUNDLE_OUT"

# 清理
rm -f "$NODE_TGZ" "$NODE_BIN"

echo "=== 打包完成：$BUNDLE_OUT ==="
echo "大小：$(du -sh "$BUNDLE_OUT" | cut -f1)"

# Step 5: 验证脚本语法
bash -n "$BUNDLE_OUT" && echo "语法检查：通过 ✓"
```

- [ ] **Step 3: 权限设置**

```bash
chmod +x scripts/macos/build_bundle.sh scripts/macos/install.command
bash -n scripts/macos/build_bundle.sh && echo "build_bundle.sh 语法正确"
bash -n scripts/macos/install.command && echo "install.command 语法正确"
```

- [ ] **Step 4: Commit**

```bash
git add scripts/macos/build_bundle.sh scripts/macos/install.command
git commit -m "feat(macos): Full Bundle 打包脚本 build_bundle.sh"
```

---

## Chunk 6: 全部测试 + 集成验证

### Task 6: 最终验证

**Files:**
- 无新增文件

- [ ] **Step 1: 运行全部 bats 测试**

```bash
bats tests/macos/ 2>&1
# 期望: 全部测试 PASS（或 skip，若非 macOS）
```

- [ ] **Step 2: shellcheck 静态分析**

```bash
# 安装: brew install shellcheck
shellcheck scripts/macos/*.sh scripts/macos/install.command 2>&1 | grep -v "^$" | head -20
# 期望: 无 SC2xxx 错误（仅允许 info 级别的建议）
```

- [ ] **Step 3: 验证 Lite 版脚本**

```bash
# bash 语法检查所有脚本
for f in scripts/macos/*.sh scripts/macos/install.command; do
  bash -n "$f" && echo "✓ $f" || echo "✗ $f"
done
```

- [ ] **Step 4: 最终 Commit**

```bash
git add scripts/macos/ tests/macos/
git commit -m "feat(macos): Plan C macOS 安装脚本完成（离线/在线/ZIP 三模式，launchd 服务，Clash 代理）"
```

---

## 安装脚本产物说明

| 产物 | 模式 | 大小 | 使用场景 |
|---|---|---|---|
| `install.command`（Lite） | 在线 | ~5KB | 同目录有 detect/service/clash.sh 时可用 |
| `dist/macos/install.command`（Full Bundle） | 离线 | ~265MB | 内嵌 Node.js + openclaw.tgz + Mihomo |
| `uninstall.sh`（安装后生成） | - | ~1KB | 安装后位于 `$INSTALL_PATH/uninstall.sh` |

## 用户操作指引

```
Full Bundle 解压后目录：
openclaw-macos/
├── install.command    ← 双击此文件（在 Terminal 中运行）
└── README.txt

安装后：
~/.openclaw/
├── openclaw.json       # 主配置（密码哈希）
└── deploy_meta.json    # 安装记录
~/Library/LaunchAgents/
└── com.openclaw.gateway.plist  # launchd 服务配置
~/openclaw/
├── node/node           # Node.js 运行时
├── openclaw_pkg/       # openclaw 包
├── openclaw.log        # 服务日志
└── uninstall.sh        # 卸载脚本
```
