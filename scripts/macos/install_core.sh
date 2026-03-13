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
    read -rsp "管理员密码（>=8位，含字母+数字）: " ADMIN_PASSWORD
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

[[ "${BASH_SOURCE[0]}" == "$0" ]] && main "$@"
