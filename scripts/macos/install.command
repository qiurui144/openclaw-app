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

# shellcheck source=install_core.sh
source "$SCRIPT_DIR/install_core.sh"
