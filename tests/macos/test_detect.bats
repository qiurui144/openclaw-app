#!/usr/bin/env bats

setup() {
  source "$(dirname "$BATS_TEST_FILENAME")/../../scripts/macos/detect.sh"
}

@test "check_macos_version: current macOS should pass (>= 11)" {
  [[ "$(uname)" == "Darwin" ]] || skip "not macOS"
  run check_macos_version
  [ "$status" -eq 0 ]
}

@test "check_disk_space: /tmp has at least 1MB free" {
  run check_disk_space "/tmp" 1
  [ "$status" -eq 0 ]
}

@test "check_disk_space: requiring 999999999 MB should fail" {
  run check_disk_space "/tmp" 999999999
  [ "$status" -ne 0 ]
}

@test "check_port_free: 65432 should be available" {
  run check_port_free 65432
  [ "$status" -eq 0 ]
}

@test "get_arch: returns arm64 or x86_64" {
  run get_arch
  [[ "$output" == "arm64" || "$output" == "x86_64" ]]
}
