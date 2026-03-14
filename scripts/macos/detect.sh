#!/usr/bin/env bash
# detect.sh - macOS 系统环境检测函数库

# 检查 macOS 版本 >= 11（Big Sur）
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
    return 2
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
  check_port_free "${SERVICE_PORT:-18789}" || true
  check_curl             || fatal=1
  echo "════════════════════"
  return $fatal
}
