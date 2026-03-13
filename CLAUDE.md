# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 构建命令

### 依赖安装（Ubuntu 22.04/24.04）

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
# AppImage 额外需要：
sudo apt install squashfs-tools libfuse2
```

### 开发模式

```bash
npm install          # 安装前端依赖
npm run tauri dev    # 启动 Tauri 开发服务器（需要显示环境）
```

### 构建（每次打包必须同时产出 Lite + Full Bundle 两个版本）

```bash
# 1. Lite 版（在线下载模式，~76MB AppImage）
OC_BUILD_BUNDLED=0 npm run tauri build

# 2. 下载 Full Bundle 所需资源
bash scripts/fetch_resources.sh
# 如果缺少 openclaw.tgz 占位文件：
tar czf resources/binaries/linux/openclaw.tgz --files-from=/dev/null
tar czf resources/binaries/windows/openclaw.tgz --files-from=/dev/null

# 3. Full Bundle 版（离线安装，~265MB AppImage）
OC_BUILD_BUNDLED=1 npm run tauri build -- --features bundled

# AppImage 构建失败时加环境变量：
APPIMAGE_EXTRACT_AND_RUN=1 npm run tauri build -- --bundles appimage
```

### 测试

```bash
npm test                                  # 前端 Vitest（25 tests）
cd src-tauri && cargo test                # Rust 单元测试（23 tests）
bats tests/macos/                         # macOS bash 脚本测试（12 tests）
bash -n scripts/macos/*.sh                # bash 语法检查
```

### 单独运行某个 Rust 测试

```bash
cd src-tauri && cargo test test_name -- --exact
```

## 架构总览

Tauri 2.x（Rust 后端 + Vue 3 前端）部署向导，替代旧 Qt/C++ 实现。

### 后端模块（src-tauri/src/）

- **deploy.rs**：11 步部署引擎，三模式（Bundled/Online/LocalZip），`DeployConfigDto`（IPC 边界）→ `DeployConfig`（`secrecy::Secret<String>` 密码）
- **system_check.rs**：OS/磁盘/端口/网络检查，跨平台
- **clash_proxy.rs**：Mihomo 临时代理（OnceLock<Mutex<Option<Child>>>），订阅 URL 持久化
- **skills_manager.rs**：Skills .tgz 原子更新（同文件系统 rename），淘宝镜像
- **updater.rs**：GitHub Release ZIP 更新 + 回滚机制
- **session_state.rs**：断点续传状态（严格不含密码）
- **platform_config.rs**：企业微信/QQ Work/钉钉/飞书 webhook 配置

### 前端（src/）

- **pages/**：10 个向导页面，路由在 `router/index.ts`
- **stores/**：`wizard.ts`（向导状态）、`config.ts`（部署配置 + DTO 转换）
- **composables/**：`useTauri.ts`（IPC 包装）、`useWizardNav.ts`（动态路由，在线模式插入 Clash 页）
- **components/**：WizardLayout、StepIndicator、CheckItem、PasswordStrength、LogPanel、QrCodeModal

### macOS 脚本（scripts/macos/）

macOS 不使用 Tauri，使用纯 bash `.command` 脚本。`build_bundle.sh` 将 Node.js + openclaw.tgz base64 编码内嵌脚本尾部。

## 内置二进制资源

Full Bundle 构建需要预先下载资源到 `resources/binaries/`：
- `resources/binaries/linux/node`：Node.js Linux x64 二进制
- `resources/binaries/linux/openclaw.tgz`：openclaw 服务包
- `resources/binaries/windows/node.exe`：Node.js Windows x64 二进制
- `resources/binaries/windows/openclaw.tgz`：openclaw 服务包

通过 `scripts/fetch_resources.sh` 自动下载 Node.js，openclaw.tgz 由 CI 构建产出。

## 版本发布

发布前同步更新：
- `src-tauri/Cargo.toml` 中的 `version`
- `src-tauri/tauri.conf.json` 中的 `version`
- `package.json` 中的 `version`

CI 通过 tag push（`v*`）触发 `.github/workflows/release.yml`，自动构建三平台 Lite + Full Bundle。
