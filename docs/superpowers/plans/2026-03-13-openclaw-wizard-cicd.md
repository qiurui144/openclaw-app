# OpenClaw 构建系统与 CI/CD 实现计划

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 配置 GitHub Actions CI/CD 流水线，自动构建 Linux AppImage、Windows NSIS EXE、macOS ZIP 三端产物，并完成 Qt 代码清理。

**Architecture:** 矩阵构建（3 runner），Linux 锁定 `ubuntu-20.04`（glibc 2.31），Windows 使用 `windows-latest`，macOS 仅打包 bash 脚本（无需 Tauri）。构建产物上传至 GitHub Release。

**Tech Stack:** GitHub Actions、Tauri CLI 2.x、cargo（Rust）、npm（Node.js）、NSIS（Windows 打包）、AppImage 工具链

---

## 文件结构

```
.github/
└── workflows/
    ├── ci.yml          # PR 验证：cargo check + cargo test + npm test
    └── release.yml     # tag 推送时构建全平台产物并发布 Release

scripts/
├── build_appimage.sh   # Linux AppImage 打包（已有，需更新）
├── macos/              # macOS 脚本（Plan C 已完成）
└── fetch_resources.sh  # 下载嵌入资源（Node.js binaries for bundled build）

src-tauri/
└── tauri.conf.json     # 确认 bundle 配置（WebView2 + AppImage）

（Qt 相关文件清理）
src/                    # 删除旧 Qt 页面（保留 core/ 中 Rust 重写前的参考）
CMakeLists.txt          # 可选保留（历史参考）
```

---

## Chunk 1: Qt 清理

### Task 1: 移除旧 Qt 代码

**Files:**
- Delete: `src/pages/` 下旧 Qt 页面（.cpp/.h）
- Delete: `src/DeployWizard.cpp`、`src/DeployWizard.h`
- Delete: `src/main.cpp`（Qt main）
- Modify: `.gitignore`

**注意**：保留 `src/core/DeployEngine.*`、`src/core/SystemCheck.*`、`src/core/UpdateChecker.*` 作为 Rust 重写时的参考文档，待 Plan A 实现完成后再删除。

- [ ] **Step 1: 创建 Qt 代码归档**

```bash
# 移动到 legacy/ 目录保留（不影响新构建）
mkdir -p legacy/qt_src
git mv src/pages legacy/qt_src/pages
git mv src/DeployWizard.cpp src/DeployWizard.h legacy/qt_src/
git mv src/main.cpp legacy/qt_src/
git mv src/core legacy/qt_src/core
# CMakeLists.txt 保留在根目录（ci.yml 不会触发 Qt 构建）
```

- [ ] **Step 2: 更新 .gitignore**

在 `.gitignore` 追加：

```
# Tauri / Node.js
node_modules/
dist/
.tauri/
src-tauri/target/

# Qt Build（legacy）
build/
*.pro.user

# macOS
*.DS_Store
dist/macos/

# AppImage
AppDir/
appimage-build/
*.AppImage
*.AppImage.zsync
AppImageBuilder.yml
```

- [ ] **Step 3: Commit**

```bash
git add legacy/ .gitignore
git commit -m "refactor: 归档 Qt 源码到 legacy/（保留 Rust 重写参考）"
```

---

## Chunk 2: tauri.conf.json 配置

### Task 2: Tauri 打包配置

**Files:**
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: 写 src-tauri/tauri.conf.json**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "openclaw-wizard",
  "version": "1.0.0",
  "identifier": "com.openclaw.wizard",
  "build": {
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420"
  },
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "OpenClaw 部署向导",
        "width": 760,
        "height": 560,
        "resizable": false,
        "center": true
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "linux": {
      "appimage": {
        "bundleMediaFramework": false
      }
    },
    "windows": {
      "webviewInstallMode": {
        "type": "embedBootstrapper",
        "silent": true
      },
      "nsis": {
        "minimumWebview2Version": "100.0.0.0",
        "displayLanguageSelector": false,
        "languages": ["SimpChinese"]
      }
    }
  },
  "plugins": {
    "shell": {
      "open": true
    },
    "fs": {
      "scope": {
        "allow": ["$HOME/.openclaw/**", "$APPDATA/openclaw/**"]
      }
    },
    "dialog": {}
  }
}
```

- [ ] **Step 2: 验证 JSON 语法**

```bash
python3 -m json.tool src-tauri/tauri.conf.json > /dev/null && echo "JSON 语法正确"
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "feat(build): tauri.conf.json - WebView2 bootstrapper + AppImage 配置"
```

---

## Chunk 3: CI 验证工作流

### Task 3: .github/workflows/ci.yml

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: 写 .github/workflows/ci.yml**

```yaml
name: CI

on:
  push:
    branches: [main, master]
  pull_request:
    branches: [main, master]

jobs:
  # ── Rust 后端（Linux/Windows/macOS 三平台）──────────────
  rust:
    name: Rust (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          # Linux 锁定 ubuntu-20.04（保证 glibc 2.31，兼容 Ubuntu 20.04+）
          - os: ubuntu-20.04
            tauri_features: webkit2gtk-4-0
          - os: windows-latest
            tauri_features: ""
    steps:
      - uses: actions/checkout@v4

      - name: 安装 Linux 系统依赖
        if: matrix.os == 'ubuntu-20.04'
        run: |
          sudo apt-get update -q
          sudo apt-get install -y --no-install-recommends \
            libwebkit2gtk-4.0-dev \
            libgtk-3-dev \
            libayatana-appindicator3-dev \
            librsvg2-dev

      - name: 安装 Rust 工具链
        uses: dtolnay/rust-toolchain@stable

      - name: 缓存 Cargo 依赖
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: "src-tauri -> target"

      - name: cargo check
        working-directory: src-tauri
        env:
          TAURI_FEATURES: ${{ matrix.tauri_features }}
        run: cargo check --no-default-features

      - name: cargo test
        working-directory: src-tauri
        run: cargo test --no-default-features 2>&1

  # ── Vue 前端测试 ─────────────────────────────────────────
  frontend:
    name: Frontend Tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: 安装 Node.js 22
        uses: actions/setup-node@v4
        with:
          node-version: "22"
          cache: "npm"

      - name: 安装依赖
        run: npm ci

      - name: 运行 Vitest
        run: npm test

      - name: 构建检查
        run: npm run build

  # ── macOS bash 脚本测试 ──────────────────────────────────
  macos-scripts:
    name: macOS Scripts (shellcheck + bats)
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - name: 安装 shellcheck 和 bats-core
        run: brew install shellcheck bats-core

      - name: shellcheck
        run: shellcheck scripts/macos/*.sh scripts/macos/install.command

      - name: bats 测试
        run: bats tests/macos/
```

- [ ] **Step 2: Commit**

```bash
mkdir -p .github/workflows
git add .github/workflows/ci.yml
git commit -m "ci: 添加 PR 验证工作流（Rust + Vitest + shellcheck + bats）"
```

---

## Chunk 4: Release 工作流

### Task 4: .github/workflows/release.yml

**Files:**
- Create: `.github/workflows/release.yml`
- Create: `scripts/fetch_resources.sh`

- [ ] **Step 1: 写 scripts/fetch_resources.sh（下载 Bundled 版资源）**

```bash
#!/usr/bin/env bash
# fetch_resources.sh - 下载 Full Bundle 所需的二进制资源
# 在 CI 构建 Full Bundle 时调用

set -euo pipefail

NODE_VERSION="${NODE_VERSION:-22.17.0}"
RESOURCES_DIR="${RESOURCES_DIR:-resources/binaries}"

mkdir -p "$RESOURCES_DIR/linux" "$RESOURCES_DIR/windows"

# Linux node
echo "下载 Linux Node.js ${NODE_VERSION}…"
curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-x64.tar.xz" \
  | tar -xJ --strip-components=2 -C "$RESOURCES_DIR/linux" "node-v${NODE_VERSION}-linux-x64/bin/node"

# Windows node.exe
echo "下载 Windows Node.js ${NODE_VERSION}…"
curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-win-x64.zip" \
  -o /tmp/node_win.zip
unzip -p /tmp/node_win.zip "node-v${NODE_VERSION}-win-x64/node.exe" > "$RESOURCES_DIR/windows/node.exe"
rm -f /tmp/node_win.zip

echo "资源下载完成"
ls -lh "$RESOURCES_DIR/linux/node" "$RESOURCES_DIR/windows/node.exe"
```

- [ ] **Step 2: 写 .github/workflows/release.yml**

```yaml
name: Release

on:
  push:
    tags:
      - "v*"

permissions:
  contents: write

jobs:
  # ── Linux AppImage ───────────────────────────────────────
  build-linux:
    name: Build Linux AppImage
    runs-on: ubuntu-20.04  # 锁定 glibc 2.31
    steps:
      - uses: actions/checkout@v4

      - name: 安装 Linux 系统依赖
        run: |
          sudo apt-get update -q
          sudo apt-get install -y --no-install-recommends \
            libwebkit2gtk-4.0-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
            squashfs-tools fuse libfuse2

      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: "src-tauri -> target"

      - uses: actions/setup-node@v4
        with:
          node-version: "22"
          cache: "npm"

      - run: npm ci

      # Lite 版（无内嵌资源）
      - name: 构建 Lite AppImage
        run: npm run tauri build -- --target x86_64-unknown-linux-gnu
        env:
          OC_BUILD_BUNDLED: "0"
          OC_UPDATE_URL: ${{ secrets.OC_UPDATE_URL }}

      # Full Bundle 版（内嵌 Node.js + openclaw.tgz）
      - name: 获取内嵌资源
        run: |
          chmod +x scripts/fetch_resources.sh
          bash scripts/fetch_resources.sh
        env:
          NODE_VERSION: "22.17.0"
          RESOURCES_DIR: resources/binaries

      - name: 构建 Full Bundle AppImage
        run: npm run tauri build -- --features bundled
        env:
          OC_BUILD_BUNDLED: "1"
          OC_UPDATE_URL: ${{ secrets.OC_UPDATE_URL }}

      - name: 重命名产物
        run: |
          cd src-tauri/target/release/bundle/appimage/
          mv openclaw-wizard_*.AppImage openclaw-wizard-${{ github.ref_name }}-linux-x64-lite.AppImage || true
          # Full Bundle 需区分（通过文件大小/名称约定）
          ls -lh *.AppImage

      - uses: actions/upload-artifact@v4
        with:
          name: linux-appimage
          path: src-tauri/target/release/bundle/appimage/*.AppImage

  # ── Windows NSIS ─────────────────────────────────────────
  build-windows:
    name: Build Windows NSIS
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: "src-tauri -> target"

      - uses: actions/setup-node@v4
        with:
          node-version: "22"
          cache: "npm"

      - run: npm ci

      # Lite 版
      - name: 构建 Lite NSIS
        run: npm run tauri build
        env:
          OC_BUILD_BUNDLED: "0"

      # Full Bundle 版
      - name: 获取 Windows Node.js
        shell: bash
        run: |
          mkdir -p resources/binaries/windows
          curl -fsSL "https://nodejs.org/dist/v22.17.0/node-v22.17.0-win-x64.zip" -o node_win.zip
          unzip -p node_win.zip "node-v22.17.0-win-x64/node.exe" > resources/binaries/windows/node.exe
          rm node_win.zip

      - name: 构建 Full Bundle NSIS
        run: npm run tauri build -- --features bundled
        env:
          OC_BUILD_BUNDLED: "1"

      - uses: actions/upload-artifact@v4
        with:
          name: windows-nsis
          path: src-tauri/target/release/bundle/nsis/*.exe

  # ── macOS 脚本打包 ───────────────────────────────────────
  build-macos:
    name: Build macOS Bundle
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - name: 安装打包依赖
        run: brew install coreutils

      - name: 下载 openclaw.tgz（最新 Release）
        run: |
          curl -fsSL \
            "https://github.com/openclaw/openclaw/releases/latest/download/openclaw.tgz" \
            -o resources/openclaw.tgz

      # Full Bundle（arm64）
      - name: 打包 macOS Full Bundle (arm64)
        run: |
          bash scripts/macos/build_bundle.sh
        env:
          NODE_ARCH: arm64
          OUT_DIR: dist/macos

      # Full Bundle（x86_64）
      - name: 打包 macOS Full Bundle (x86_64)
        run: |
          bash scripts/macos/build_bundle.sh
        env:
          NODE_ARCH: x86_64
          OUT_DIR: dist/macos/x64

      # 创建 ZIP 分发包
      - name: 打包 ZIP
        run: |
          TAG=${{ github.ref_name }}
          cp scripts/macos/{detect.sh,service.sh,clash.sh} dist/macos/
          cd dist/macos
          zip -r "../../openclaw-wizard-${TAG}-macos-arm64.zip" install.command *.sh
          cd x64
          zip -r "../../../openclaw-wizard-${TAG}-macos-x64.zip" install.command *.sh

      - uses: actions/upload-artifact@v4
        with:
          name: macos-bundle
          path: "*.zip"

  # ── 发布 GitHub Release ──────────────────────────────────
  publish:
    name: Publish Release
    needs: [build-linux, build-windows, build-macos]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions/download-artifact@v4
        with:
          merge-multiple: true
          path: artifacts/

      - name: 列出产物
        run: ls -lh artifacts/

      - name: 创建 GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          generate_release_notes: true
          files: artifacts/**
          body: |
            ## OpenClaw 部署向导 ${{ github.ref_name }}

            ### 下载

            | 平台 | 类型 | 下载 |
            |---|---|---|
            | Linux (x64) | Lite（在线） | `*-linux-x64-lite.AppImage` |
            | Linux (x64) | Full Bundle（离线） | `*-linux-x64-full.AppImage` |
            | Windows 10/11 | Lite（在线） | `*-windows-lite.exe` |
            | Windows 10/11 | Full Bundle（离线） | `*-windows-full.exe` |
            | macOS (Apple Silicon) | 一键脚本 | `*-macos-arm64.zip` |
            | macOS (Intel) | 一键脚本 | `*-macos-x64.zip` |

            ### 最低系统要求
            - Linux: Ubuntu 20.04+（glibc ≥ 2.31）
            - Windows: Windows 10 1803+（WebView2 自动安装）
            - macOS: macOS 11 Big Sur+
```

- [ ] **Step 3: chmod 和 commit**

```bash
chmod +x scripts/fetch_resources.sh
git add .github/workflows/release.yml scripts/fetch_resources.sh
git commit -m "ci: 添加多平台 Release 构建工作流（Linux/Windows/macOS）"
```

---

## Chunk 5: 本地构建验证

### Task 5: 本地快速验证构建

**Files:**
- 无新增文件

- [ ] **Step 1: 验证 Rust 编译（Linux）**

```bash
cd src-tauri
# 不含 bundled feature（Lite 版）
cargo check --no-default-features 2>&1 | grep "^error" | head -5
# 期望: 无 error
cd ..
```

- [ ] **Step 2: 验证前端构建**

```bash
npm run build 2>&1 | grep -E "error|✓ built" | head -5
# 期望: ✓ built
```

- [ ] **Step 3: Tauri dev 模式（仅有 display 时）**

```bash
# 注意：需要有显示器/X session
# 本地测试时运行：
# npm run tauri dev
echo "如需测试 Tauri GUI，请在有显示的环境运行: npm run tauri dev"
```

- [ ] **Step 4: macOS 脚本语法验证**

```bash
for f in scripts/macos/*.sh scripts/macos/install.command; do
  bash -n "$f" && echo "✓ $f"
done
```

- [ ] **Step 5: 最终 Commit**

```bash
git add .
git status
git commit -m "feat: Plan D 构建系统完成 - CI/CD + 三平台打包流水线" || echo "无变更需要提交"
```

---

## 构建矩阵总结

| 任务 | Runner | 产物 | 触发条件 |
|---|---|---|---|
| Rust CI | ubuntu-20.04 / windows-latest | - | PR / push main |
| Frontend CI | ubuntu-latest | - | PR / push main |
| macOS Scripts CI | macos-latest | - | PR / push main |
| Linux AppImage | ubuntu-20.04 | Lite + Full Bundle AppImage | tag push |
| Windows NSIS | windows-latest | Lite + Full Bundle .exe | tag push |
| macOS Bundle | macos-latest | arm64 + x64 .zip | tag push |

## 关键注意事项

1. **ubuntu-20.04 锁定**：Linux runner 必须为 `ubuntu-20.04`，确保 glibc 2.31，否则产物无法在 Ubuntu 20.04 上运行
2. **webkit2gtk-4-0**：Linux 系统依赖必须安装 `libwebkit2gtk-4.0-dev`（非 4.1），与 Cargo.toml 中的 feature flag 对应
3. **WebView2 Bootstrapper**：tauri.conf.json 中 `embedBootstrapper` 确保 Windows 10 1803+ 用户首次运行时自动安装 WebView2
4. **macOS 无 Tauri**：macOS runner 仅运行 bash 脚本打包，不安装 Rust/Tauri，节省 CI 时间
5. **OC_UPDATE_URL secret**：GitHub Actions secrets 中配置更新服务器 URL，Lite 版构建时注入
