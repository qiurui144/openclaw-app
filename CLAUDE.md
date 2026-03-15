# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Git 权限

本项目允许 `git push`，包括推送 tag（用于触发 CI 自动构建）。

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

# 3. Full Bundle 版（离线安装，~169MB AppImage）
OC_BUILD_BUNDLED=1 npm run tauri build -- --features bundled

# AppImage 构建失败时加环境变量：
APPIMAGE_EXTRACT_AND_RUN=1 npm run tauri build -- --bundles appimage
```

### 测试

```bash
npm test                                                     # 前端 Vitest（27 tests）
cd src-tauri && DEP_TAURI_DEV=true cargo test                # Rust 单元测试（46 tests）
bats tests/macos/                                            # macOS bash 脚本测试（12 tests）
```

### 单独运行某个 Rust 测试

```bash
cd src-tauri && DEP_TAURI_DEV=true cargo test test_name -- --exact
```

## 架构总览

Tauri 2.x（Rust 后端 + Vue 3 前端）部署向导，替代旧 Qt/C++ 实现。

### 后端模块（src-tauri/src/）

- **deploy.rs**：11 步部署引擎，三模式（Bundled/Online/LocalZip），`DeployConfigDto`（IPC 边界）→ `DeployConfig`（`secrecy::Secret<String>` 密码）
- **system_check.rs**：OS/磁盘/端口/网络检查，跨平台（Linux 用 libc::statvfs，Windows 用 sysinfo::Disks）
- **clash_proxy.rs**：Mihomo 临时代理（OnceLock<Mutex<Option<Child>>>），订阅 URL 持久化
- **skills_manager.rs**：Skills .tgz 原子更新（同文件系统 rename），淘宝镜像
- **license.rs**：JWT 离线验证（RSA 公钥内嵌）+ 机器指纹 + 授权码模式，支持多客户 `OC_LICENSE_API` 注入
- **activation.rs**：公众号关注门槛（QR 请求/轮询/本地 JWT 标记），`OC_CLIENT_ID` 构建时注入
- **skills_registry.rs**：Skill 索引获取 + AES-256-GCM 加密内容解密 + 密钥缓存 + 支付订单
- **updater.rs**：GitHub Release ZIP 更新 + 回滚机制
- **session_state.rs**：断点续传状态（严格不含密码）
- **platform_config.rs**：企业微信/QQ Work/钉钉/飞书 webhook 配置

### 前端（src/）

- **pages/**：11 个向导页面（含 ActivationGatePage），路由在 `router/index.ts`
- **stores/**：`wizard.ts`（向导状态）、`config.ts`（部署配置 + DTO 转换）、`license.ts`（许可证 + Skills 商店状态）
- **composables/**：`useTauri.ts`（IPC 包装）、`useWizardNav.ts`（动态路由，在线模式插入 Clash 页）
- **components/**：WizardLayout、StepIndicator、CheckItem、PasswordStrength、LogPanel、QrCodeModal、SkillCard、LoginModal、PaymentModal

### 授权服务（services/license-server/）

Node.js + SQLite 授权服务，部署在 `license.openclaw.cn`。每客户独立部署一套实例。

- **纯授权码体系**：无用户注册/登录，授权码是唯一的付费激活方式
- **公众号关注门槛**：微信服务号 API 集成，生成带参二维码验证关注
- 授权码模式 JWT `machine_id = "*"`，客户端跳过指纹校验
- RSA 密钥对：公钥内嵌客户端（`src-tauri/keys/license_pub.pem`），私钥仅在服务端
- 付费 Skill 加密下发：AES-256-GCM 加密 + 零宽字符水印
- IP 限流中间件 + 管理操作审计日志
- 支付完成自动生成授权码（支付→授权码→客户端激活）
- 开发模式：`DEV_SKIP_SIGNATURE=1` 环境变量跳过 JWT 签名验证
- 启动：`cd services/license-server && npm install && ADMIN_KEY=xxx node src/index.js`
- 多客户构建：`OC_CLIENT_ID=acme OC_LICENSE_API=https://license.acme.com/api npm run tauri build`

### macOS 脚本（scripts/macos/）

macOS 不使用 Tauri，使用纯 bash `.command` 脚本。`build_bundle.sh` 将 Node.js + openclaw.tgz base64 编码内嵌脚本尾部。

## 内置二进制资源

Full Bundle 构建需要预先下载资源到 `resources/binaries/`：
- `resources/binaries/linux/node`：Node.js Linux x64 二进制
- `resources/binaries/linux/openclaw.tgz`：openclaw 服务包
- `resources/binaries/windows/node.exe`：Node.js Windows x64 二进制
- `resources/binaries/windows/openclaw.tgz`：openclaw 服务包

通过 `scripts/fetch_resources.sh` 自动从 npm registry 下载（淘宝镜像优先）。

## 版本发布

发布前同步更新：
- `src-tauri/Cargo.toml` 中的 `version`
- `src-tauri/tauri.conf.json` 中的 `version`
- `package.json` 中的 `version`

CI 通过 tag push（`v*`）触发 `.github/workflows/release.yml`，自动构建三平台 Lite + Full Bundle。
