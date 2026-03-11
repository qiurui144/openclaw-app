#!/usr/bin/env bash
# ============================================================
# build_appimage.sh
# 构建 OpenClaw AppImage（兼容 Ubuntu 20.04+）
# 依赖：cmake, ninja, linuxdeploy（无需 linuxdeploy-plugin-qt）
# Qt 库采用手动复制方式部署，AppImage runtime 从 linuxdeploy 自身提取
# 推荐在 Ubuntu 20.04 Docker 容器中运行以保证 glibc 兼容性
# ============================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_ROOT/build/linux"
APPDIR="$BUILD_DIR/AppDir"
APP_NAME="openclaw"
APP_VERSION="${VERSION:-1.0.0}"
ARCH="${ARCH:-x86_64}"

# ── 颜色输出 ──────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
info()    { echo -e "${GREEN}[INFO]${NC} $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC} $*"; }
die()     { echo -e "${RED}[ERROR]${NC} $*" >&2; exit 1; }

# ── 检查依赖 ──────────────────────────────────────────────────
check_deps() {
    info "检查构建依赖…"
    local missing=()
    for cmd in cmake ninja-build pkg-config patchelf python3; do
        command -v "$cmd" &>/dev/null || missing+=("$cmd")
    done

    # Qt6 或 Qt5
    if ! pkg-config --exists Qt6Core 2>/dev/null && \
       ! pkg-config --exists Qt5Core 2>/dev/null; then
        missing+=("qt6-base-dev 或 qtbase5-dev")
    fi

    if [[ ${#missing[@]} -gt 0 ]]; then
        die "缺少依赖：${missing[*]}\n请运行：sudo apt install cmake ninja-build qt6-base-dev qt6-tools-dev patchelf python3"
    fi

    # 只需要 linuxdeploy（不需要 linuxdeploy-plugin-qt）
    if ! command -v linuxdeploy &>/dev/null; then
        warn "linuxdeploy 未找到，尝试自动下载…"
        download_linuxdeploy
    fi
    info "依赖检查通过 ✓"
}

download_linuxdeploy() {
    local mirror="${GITHUB_MIRROR:-https://github.com}"
    local url="$mirror/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage"
    local dest="/usr/local/bin/linuxdeploy"
    info "下载 linuxdeploy 到 $dest …"
    if command -v wget &>/dev/null; then
        wget -q --show-progress -O "$dest" "$url"
    elif command -v curl &>/dev/null; then
        curl -fsSL -o "$dest" "$url"
    else
        die "需要 wget 或 curl 来下载 linuxdeploy"
    fi
    chmod +x "$dest"
}

# ── 编译 ──────────────────────────────────────────────────────
build() {
    info "配置 CMake…"
    mkdir -p "$BUILD_DIR"
    cmake -S "$PROJECT_ROOT" -B "$BUILD_DIR" \
        -G Ninja \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_INSTALL_PREFIX="$APPDIR/usr"

    info "编译中（$(nproc) 线程）…"
    cmake --build "$BUILD_DIR" --parallel "$(nproc)"

    info "安装到 AppDir…"
    cmake --install "$BUILD_DIR"
}

# ── 手动部署 Qt 库（不依赖 linuxdeploy-plugin-qt）────────────
deploy_qt_libs() {
    info "手动部署 Qt 运行时库…"

    # 探测 Qt 库/插件目录
    local qt_lib qt_plugin
    if pkg-config --exists Qt6Core 2>/dev/null; then
        qt_lib="$(pkg-config --variable=libdir Qt6Core)"
        qt_plugin="${qt_lib}/qt6/plugins"
    else
        qt_lib="$(pkg-config --variable=libdir Qt5Core)"
        qt_plugin="${qt_lib}/qt5/plugins"
    fi
    [[ -d "$qt_plugin" ]] || die "Qt 插件目录不存在：$qt_plugin"

    mkdir -p "$APPDIR/usr/lib"
    mkdir -p "$APPDIR/usr/plugins/platforms"
    mkdir -p "$APPDIR/usr/plugins/imageformats"
    mkdir -p "$APPDIR/usr/plugins/platformthemes"
    mkdir -p "$APPDIR/usr/plugins/xcbglintegrations"

    # 复制 Qt 核心共享库
    for lib in Qt6Core Qt6Gui Qt6Widgets Qt6Network Qt6DBus Qt5Core Qt5Gui Qt5Widgets Qt5Network Qt5DBus; do
        local sofile="${qt_lib}/lib${lib}.so.6"
        [[ -f "$sofile" ]] && cp -v "$sofile" "$APPDIR/usr/lib/" || true
        local sofile5="${qt_lib}/lib${lib}.so.5"
        [[ -f "$sofile5" ]] && cp -v "$sofile5" "$APPDIR/usr/lib/" || true
    done

    # 复制平台插件（GUI 运行必需）
    [[ -f "${qt_plugin}/platforms/libqxcb.so" ]] && \
        cp -v "${qt_plugin}/platforms/libqxcb.so" "$APPDIR/usr/plugins/platforms/"
    cp "${qt_plugin}/platforms/libqwayland"*.so "$APPDIR/usr/plugins/platforms/" 2>/dev/null || true
    cp "${qt_plugin}/imageformats/libqsvg.so" "$APPDIR/usr/plugins/imageformats/" 2>/dev/null || true
    cp "${qt_plugin}/imageformats/libqpng.so" "$APPDIR/usr/plugins/imageformats/" 2>/dev/null || true
    cp "${qt_plugin}/platformthemes/"*.so     "$APPDIR/usr/plugins/platformthemes/" 2>/dev/null || true
    cp "${qt_plugin}/xcbglintegrations/"*.so  "$APPDIR/usr/plugins/xcbglintegrations/" 2>/dev/null || true

    # 写入 qt.conf，让 Qt 在 AppImage 内找到插件路径
    cat > "$APPDIR/usr/bin/qt.conf" <<'QTCONF'
[Paths]
Prefix = ..
Plugins = plugins
Libraries = lib
QTCONF

    info "Qt 库部署完成 ✓"
}

# ── 从 linuxdeploy 提取 AppImage runtime ──────────────────────
extract_runtime() {
    local linuxdeploy_bin
    linuxdeploy_bin="$(command -v linuxdeploy)"
    info "从 $linuxdeploy_bin 提取 AppImage runtime…"

    local runtime_file="/tmp/runtime-${ARCH}"
    local offset
    offset=$(python3 -c "
import sys
with open('${linuxdeploy_bin}', 'rb') as f:
    data = f.read()
idx = data.find(b'hsqs')
if idx == -1:
    idx = data.find(b'sqsh')
if idx == -1:
    sys.exit(1)
print(idx)
") || die "无法在 linuxdeploy 中找到 squashfs 边界"

    dd if="$linuxdeploy_bin" of="$runtime_file" bs=1 count="$offset" 2>/dev/null
    echo "$runtime_file"
}

# ── 打包 AppImage ──────────────────────────────────────────────
package() {
    info "创建 desktop 文件…"
    mkdir -p "$APPDIR/usr/share/applications"
    mkdir -p "$APPDIR/usr/share/icons/hicolor/64x64/apps"

    cat > "$APPDIR/usr/share/applications/${APP_NAME}.desktop" <<EOF
[Desktop Entry]
Name=OpenClaw 部署向导
Comment=OpenClaw 一键部署应用
Exec=openclaw
Icon=openclaw
Type=Application
Categories=Utility;System;
StartupNotify=true
EOF

    # SVG → PNG 图标转换
    if command -v rsvg-convert &>/dev/null; then
        rsvg-convert -w 64 -h 64 \
            "$PROJECT_ROOT/resources/icons/app.svg" \
            -o "$APPDIR/usr/share/icons/hicolor/64x64/apps/${APP_NAME}.png"
    elif command -v convert &>/dev/null; then
        convert -background none \
            "$PROJECT_ROOT/resources/icons/app.svg" \
            -resize 64x64 \
            "$APPDIR/usr/share/icons/hicolor/64x64/apps/${APP_NAME}.png"
    else
        warn "未找到图标转换工具，使用 SVG 占位"
        cp "$PROJECT_ROOT/resources/icons/app.svg" \
           "$APPDIR/usr/share/icons/hicolor/64x64/apps/${APP_NAME}.png" 2>/dev/null || true
    fi

    # 手动部署 Qt（替代损坏或缺失的 linuxdeploy-plugin-qt）
    deploy_qt_libs

    # 临时禁用 linuxdeploy-plugin-qt（如存在但损坏）以防止自动调用
    local plugin_backup=""
    if command -v linuxdeploy-plugin-qt &>/dev/null; then
        local plugin_path
        plugin_path="$(command -v linuxdeploy-plugin-qt)"
        plugin_backup="${plugin_path}.disabled_$$"
        mv "$plugin_path" "$plugin_backup"
        warn "已临时禁用 linuxdeploy-plugin-qt（将在构建后恢复）"
    fi

    # 用 linuxdeploy 解析非 Qt 依赖，不输出 AppImage（仅部署 libs）
    info "解析系统依赖…"
    local extracted_apprun
    extracted_apprun="$(mktemp -d)/extracted"
    APPIMAGE_EXTRACT_AND_RUN=1 \
    LINUXDEPLOY_OUTPUT_VERSION="$APP_VERSION" \
    ARCH="$ARCH" \
    linuxdeploy --appdir "$APPDIR" 2>&1 || true

    # 恢复 plugin（如有备份）
    if [[ -n "$plugin_backup" && -f "$plugin_backup" ]]; then
        mv "$plugin_backup" "$(command -v linuxdeploy-plugin-qt 2>/dev/null || echo "${plugin_backup%.disabled_*}")"
    fi

    # 提取 AppImage runtime 并用 appimagetool 打包
    local runtime_file
    runtime_file="$(extract_runtime)"

    # appimagetool 由 linuxdeploy 内置的 plugin-appimage 提供
    local appimagetool
    appimagetool="$(find /tmp/appimage_extracted_* \
        -name appimagetool -path "*/linuxdeploy-plugin-appimage/*" 2>/dev/null | head -1)"
    if [[ -z "$appimagetool" ]]; then
        # 首次运行时需要先让 linuxdeploy 提取自身
        APPIMAGE_EXTRACT_AND_RUN=1 linuxdeploy --help >/dev/null 2>&1 || true
        appimagetool="$(find /tmp/appimage_extracted_* \
            -name appimagetool -path "*/linuxdeploy-plugin-appimage/*" 2>/dev/null | head -1)"
    fi
    [[ -n "$appimagetool" ]] || die "未找到 appimagetool（需先运行一次 linuxdeploy）"

    info "打包 AppImage…"
    local OUTPUT="${APP_NAME}-${APP_VERSION}-${ARCH}.AppImage"
    mkdir -p "$PROJECT_ROOT/dist"
    ARCH="$ARCH" \
    "$appimagetool" --runtime-file "$runtime_file" \
        "$APPDIR" "$PROJECT_ROOT/dist/${OUTPUT}" 2>&1

    info "AppImage 已生成：dist/${OUTPUT}"
}

# ── 主流程 ────────────────────────────────────────────────────
main() {
    cd "$PROJECT_ROOT"
    echo "=================================================="
    echo " OpenClaw AppImage 构建脚本  v${APP_VERSION}"
    echo " 目标：Ubuntu 20.04+  架构：${ARCH}"
    echo "=================================================="

    check_deps
    build
    package

    echo ""
    info "构建完成！输出文件：dist/${APP_NAME}-${APP_VERSION}-${ARCH}.AppImage"
    info "使用方式：chmod +x dist/*.AppImage && ./dist/*.AppImage"
}

main "$@"
