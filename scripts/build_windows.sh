#!/usr/bin/env bash
# ============================================================
# build_windows.sh
# 在 Linux 上交叉编译 Windows x64 EXE，或生成 Windows 原生构建说明
# 方案：
#   1. 使用 MXE (M cross environment) mingw-w64 Qt 交叉编译
#   2. 或在 Windows 上使用 MSVC + windeployqt + NSIS 打包
# ============================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_ROOT/build/windows"
APP_NAME="openclaw"
APP_VERSION="${VERSION:-1.0.0}"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
info() { echo -e "${GREEN}[INFO]${NC} $*"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
die()  { echo -e "${RED}[ERROR]${NC} $*" >&2; exit 1; }

# ── 检测构建方式 ──────────────────────────────────────────────
detect_build_mode() {
    if [[ "${FORCE_NATIVE:-0}" == "1" ]]; then
        echo "native"
        return
    fi

    # 检查 MXE 是否安装
    if [[ -d "/usr/lib/mxe" ]] || command -v x86_64-w64-mingw32.static-cmake &>/dev/null; then
        echo "mxe"
    elif command -v x86_64-w64-mingw32-g++ &>/dev/null; then
        echo "mingw"
    else
        echo "native"
    fi
}

# ── 方案A：MXE 交叉编译 ────────────────────────────────────────
build_with_mxe() {
    info "使用 MXE 进行 Windows 交叉编译…"

    local MXE_ROOT="${MXE_ROOT:-/usr/lib/mxe}"
    local MXE_TARGET="x86_64-w64-mingw32.static"
    local MXE_CMAKE="${MXE_ROOT}/usr/bin/${MXE_TARGET}-cmake"

    [[ -x "$MXE_CMAKE" ]] || die "MXE cmake 未找到：$MXE_CMAKE"

    mkdir -p "$BUILD_DIR"
    "$MXE_CMAKE" -S "$PROJECT_ROOT" -B "$BUILD_DIR" \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_SYSTEM_NAME=Windows

    cmake --build "$BUILD_DIR" --parallel "$(nproc)"

    mkdir -p "$PROJECT_ROOT/dist"
    local exe="$BUILD_DIR/${APP_NAME}.exe"
    [[ -f "$exe" ]] || die "编译产物未找到：$exe"
    cp "$exe" "$PROJECT_ROOT/dist/${APP_NAME}-${APP_VERSION}-win64.exe"
    info "Windows EXE 已生成：dist/${APP_NAME}-${APP_VERSION}-win64.exe"
}

# ── 方案B：MinGW-w64 交叉编译（需要 Qt for Windows 库）──────────
build_with_mingw() {
    info "使用 MinGW-w64 交叉编译…"
    warn "此方式需要预先准备 Qt Windows 库，参考 DEVELOP.md"

    local TOOLCHAIN="${SCRIPT_DIR}/toolchain-mingw64.cmake"
    mkdir -p "$BUILD_DIR"

    cmake -S "$PROJECT_ROOT" -B "$BUILD_DIR" \
        -DCMAKE_TOOLCHAIN_FILE="$TOOLCHAIN" \
        -DCMAKE_BUILD_TYPE=Release

    cmake --build "$BUILD_DIR" --parallel "$(nproc)"

    mkdir -p "$PROJECT_ROOT/dist"
    find "$BUILD_DIR" -name "${APP_NAME}.exe" -exec cp {} \
        "$PROJECT_ROOT/dist/${APP_NAME}-${APP_VERSION}-win64.exe" \;
}

# ── 方案C：原生 Windows 构建说明 ──────────────────────────────
print_native_build_guide() {
    warn "未检测到交叉编译工具链，请在 Windows 环境下进行构建。"
    cat <<'EOF'

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Windows 原生构建步骤（在 Windows 10/11 上执行）
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

1. 安装 Qt 6.x（在线安装器：https://www.qt.io/download-qt-installer）
   - 选择：Qt 6.x > MSVC 2022 64-bit
   - 安装 CMake 和 Ninja 组件

2. 安装 Visual Studio 2022（Community 版本即可）
   - 勾选"使用 C++ 的桌面开发"工作负载

3. 克隆源码并进入项目目录：
   git clone https://github.com/openclaw/openclaw
   cd openclaw

4. 配置并编译：
   cmake -B build\windows -G "Visual Studio 17 2022" -A x64 ^
         -DCMAKE_PREFIX_PATH="C:\Qt\6.x.x\msvc2022_64"
   cmake --build build\windows --config Release

5. 打包运行时：
   cd build\windows\Release
   windeployqt.exe --release --dir deploy openclaw.exe
   xcopy /E openclaw.exe deploy\ > nul

6. 制作 NSIS 安装包（可选）：
   安装 NSIS：https://nsis.sourceforge.io/
   makensis ..\..\scripts\openclaw.nsi

7. 输出文件：
   - dist\openclaw-1.0.0-win64.exe（NSIS 安装程序）
   - dist\openclaw-portable\（绿色运行目录）

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
EOF
}

# ── 生成 MinGW64 工具链文件 ────────────────────────────────────
generate_mingw_toolchain() {
    local QT_WIN_DIR="${QT_WIN_DIR:-/opt/qt6-win64}"
    cat > "${SCRIPT_DIR}/toolchain-mingw64.cmake" <<EOF
set(CMAKE_SYSTEM_NAME Windows)
set(CMAKE_SYSTEM_PROCESSOR x86_64)

set(CMAKE_C_COMPILER   x86_64-w64-mingw32-gcc)
set(CMAKE_CXX_COMPILER x86_64-w64-mingw32-g++)
set(CMAKE_RC_COMPILER  x86_64-w64-mingw32-windres)

set(CMAKE_FIND_ROOT_PATH ${QT_WIN_DIR})
set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)
set(CMAKE_PREFIX_PATH ${QT_WIN_DIR})
EOF
    info "工具链文件已生成：scripts/toolchain-mingw64.cmake"
}

# ── 生成 NSIS 打包脚本 ─────────────────────────────────────────
generate_nsis_script() {
    cat > "${SCRIPT_DIR}/openclaw.nsi" <<'EOF'
; OpenClaw NSIS 安装脚本
Unicode True
!define APP_NAME "OpenClaw"
!define APP_VERSION "1.0.0"
!define PUBLISHER "OpenClaw Team"
!define APP_EXE "openclaw.exe"

Name "${APP_NAME} ${APP_VERSION}"
OutFile "..\dist\openclaw-${APP_VERSION}-win64-setup.exe"
InstallDir "$PROGRAMFILES64\${APP_NAME}"
RequestExecutionLevel admin
SetCompressor /SOLID lzma

!include "MUI2.nsh"
!define MUI_ABORTWARNING
!define MUI_ICON "..\resources\icons\openclaw.ico"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"

Section "主程序" SecMain
  SetOutPath "$INSTDIR"
  File /r "build\windows\deploy\*.*"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; 开始菜单快捷方式
  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"
  CreateShortcut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"

  ; 注册表卸载信息
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "DisplayName" "${APP_NAME}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "Publisher" "${PUBLISHER}"
SectionEnd

Section "Uninstall"
  Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
  Delete "$DESKTOP\${APP_NAME}.lnk"
  RMDir  "$SMPROGRAMS\${APP_NAME}"
  RMDir /r "$INSTDIR"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"
SectionEnd
EOF
    info "NSIS 脚本已生成：scripts/openclaw.nsi"
}

# ── 主流程 ────────────────────────────────────────────────────
main() {
    echo "=================================================="
    echo " OpenClaw Windows EXE 构建脚本  v${APP_VERSION}"
    echo "=================================================="

    generate_nsis_script

    local mode
    mode=$(detect_build_mode)
    info "检测到构建模式：$mode"

    case "$mode" in
        mxe)    build_with_mxe ;;
        mingw)
            generate_mingw_toolchain
            build_with_mingw
            ;;
        native) print_native_build_guide ;;
        *)      die "未知构建模式：$mode" ;;
    esac
}

main "$@"
