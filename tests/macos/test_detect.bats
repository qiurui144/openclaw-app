#!/usr/bin/env bats

setup() {
  source "$(dirname "$BATS_TEST_FILENAME")/../../scripts/macos/detect.sh"
}

@test "check_macos_version: 当前 macOS 应通过（>= 11）" {
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
