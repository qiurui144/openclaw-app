#!/usr/bin/env bash
# ============================================================
# docker_build.sh — 在 openclaw-agents-builder 容器内执行
# 镜像已预装: cmake / ninja / qtbase5-dev / libqt5network-dev
# 用法: bash /project/scripts/docker_build.sh [版本号]
# ============================================================
set -euo pipefail

APP_VERSION="${1:-1.0.0}"
export DEBIAN_FRONTEND=noninteractive

echo "[docker_build] CMake: $(cmake --version | head -1)"
echo "[docker_build] Qt5:   $(dpkg -l libqt5core5a | awk '/^ii/{print $3}')"

echo "[docker_build] 获取离线资源 (openclaw + node_modules + skills)..."
bash /project/scripts/fetch_resources.sh
echo "[docker_build] 资源获取完成"

echo "[docker_build] 清理旧构建..."
rm -rf /tmp/ocbuild
rm -rf /project/AppDir

echo "[docker_build] CMake 配置..."
cmake -S /project -B /tmp/ocbuild -G Ninja \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX=/usr \
    -DOC_VERSION="${APP_VERSION}"

echo "[docker_build] 编译 ($(nproc) 线程)..."
cmake --build /tmp/ocbuild --parallel "$(nproc)" --verbose

echo "[docker_build] 安装到 /project/AppDir..."
DESTDIR=/project/AppDir cmake --install /tmp/ocbuild

echo "[docker_build] AppDir 内容:"
find /project/AppDir -type f | head -20

echo "[docker_build] 完成."
