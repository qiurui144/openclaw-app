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
