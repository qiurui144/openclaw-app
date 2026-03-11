#!/usr/bin/env bash
# ============================================================
# build_appimage.sh
# 构建 OpenClaw AppImage（兼容 Ubuntu 20.04+）
# 依赖：cmake, ninja, linuxdeploy, linuxdeploy-plugin-qt
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
    for cmd in cmake ninja-build pkg-config; do
        command -v "$cmd" &>/dev/null || missing+=("$cmd")
    done

    # Qt6 或 Qt5
    if ! pkg-config --exists Qt6Core 2>/dev/null && \
       ! pkg-config --exists Qt5Core 2>/dev/null; then
        missing+=("qt6-base-dev 或 qtbase5-dev")
    fi

    if [[ ${#missing[@]} -gt 0 ]]; then
        die "缺少依赖：${missing[*]}\n请运行：sudo apt install cmake ninja-build qt6-base-dev qt6-tools-dev"
    fi

    # 下载 linuxdeploy 工具（如不存在）
    for tool in linuxdeploy linuxdeploy-plugin-qt; do
        if ! command -v "$tool" &>/dev/null; then
            warn "$tool 未找到，尝试自动下载…"
            download_linuxdeploy_tool "$tool"
        fi
    done
    info "依赖检查通过 ✓"
}

download_linuxdeploy_tool() {
    local tool="$1"
    local url
    # 使用国内可访问的 GitHub releases 镜像
    local mirror="${GITHUB_MIRROR:-https://github.com}"
    case "$tool" in
        linuxdeploy)
            url="$mirror/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage"
            ;;
        linuxdeploy-plugin-qt)
            url="$mirror/linuxdeploy/linuxdeploy-plugin-qt/releases/download/continuous/linuxdeploy-plugin-qt-x86_64.AppImage"
            ;;
        *)
            die "未知工具：$tool"
            ;;
    esac

    local dest="/usr/local/bin/$tool"
    info "下载 $tool 到 $dest …"
    if command -v wget &>/dev/null; then
        wget -q --show-progress -O "$dest" "$url"
    elif command -v curl &>/dev/null; then
        curl -fsSL -o "$dest" "$url"
    else
        die "需要 wget 或 curl 来下载 $tool"
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

    # 使用 resources 目录中的 SVG 转换为 PNG（如有 rsvg-convert 或 convert）
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
        warn "未找到 rsvg-convert/convert，跳过图标转换（AppImage 仍可构建）"
        # 复制一个 1x1 透明 PNG 作为占位
        cp "$PROJECT_ROOT/resources/icons/app.svg" \
           "$APPDIR/usr/share/icons/hicolor/64x64/apps/${APP_NAME}.png" 2>/dev/null || true
    fi

    info "运行 linuxdeploy 打包…"
    OUTPUT="${APP_NAME}-${APP_VERSION}-${ARCH}.AppImage"
    DEPLOY_QT_HOOK=1 \
    VERSION="$APP_VERSION" \
    ARCH="$ARCH" \
    linuxdeploy \
        --appdir "$APPDIR" \
        --plugin qt \
        --output appimage

    # linuxdeploy 默认在当前目录生成 AppImage
    local appimage_file
    appimage_file=$(ls ./*.AppImage 2>/dev/null | head -1)
    if [[ -n "$appimage_file" ]]; then
        mkdir -p "$PROJECT_ROOT/dist"
        mv "$appimage_file" "$PROJECT_ROOT/dist/${OUTPUT}"
        info "AppImage 已生成：dist/${OUTPUT}"
    else
        die "未找到生成的 AppImage 文件"
    fi
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
