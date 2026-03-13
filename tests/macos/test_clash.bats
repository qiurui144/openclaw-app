#!/usr/bin/env bats

setup() {
  export TMPDIR_TEST="$(mktemp -d)"
  export CLASH_DIR="$TMPDIR_TEST/clash"
  export CLASH_SUB_FILE="$TMPDIR_TEST/proxy.json"
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
