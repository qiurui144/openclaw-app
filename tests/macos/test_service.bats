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
