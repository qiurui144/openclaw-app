#!/usr/bin/env bash
# clash.sh - macOS Clash/Mihomo 临时代理管理

CLASH_DIR="${CLASH_DIR:-$HOME/.openclaw/clash_tmp}"
CLASH_PID_FILE="$CLASH_DIR/mihomo.pid"
CLASH_BIN="$CLASH_DIR/mihomo"
CLASH_CONFIG="$CLASH_DIR/config.yaml"
CLASH_PROXY_PORT="${CLASH_PROXY_PORT:-7890}"
MIHOMO_VERSION="${MIHOMO_VERSION:-1.18.7}"
CLASH_SUB_FILE="${CLASH_SUB_FILE:-${HOME}/.openclaw/proxy.json}"

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
  echo "https://github.com/MetaCubeX/mihomo/releases/download/v${MIHOMO_VERSION}/mihomo-${os_tag}-${arch_tag}-v${MIHOMO_VERSION}.gz"
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

# 从 bundle 提取 Mihomo（如果内嵌了 __BUNDLE_mihomo__）
clash_extract_bundled_mihomo() {
  local script_path="${BASH_SOURCE[0]:-$0}"
  if grep -q "__BUNDLE_mihomo__" "$script_path" 2>/dev/null; then
    mkdir -p "$CLASH_DIR"
    echo "从 Bundle 提取 Mihomo…"
    sed -n '/^__BUNDLE_mihomo__$/,/^__END_BUNDLE_mihomo__$/p' "$script_path" \
      | sed '1d;$d' | base64 -d > "$CLASH_BIN"
    chmod +x "$CLASH_BIN"
    echo "Mihomo 已从 Bundle 提取"
    return 0
  fi
  return 1
}

# 下载 Mihomo 二进制（在线回退）
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
  # 获取二进制：优先 bundle 提取，回退在线下载
  [[ -x "$CLASH_BIN" ]] || clash_extract_bundled_mihomo || clash_download_mihomo || return 1
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
  if curl -fsSL -x "$proxy_url" --max-time 8 "https://github.com" -o /dev/null 2>/dev/null; then
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
