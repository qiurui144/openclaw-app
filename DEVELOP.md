# DEVELOP - 开发与构建指南

## 环境要求

| 组件 | 版本要求 |
|------|---------|
| Rust | stable（通过 rustup 安装）|
| Node.js | 22+ |
| npm | 10+ |

### Linux 额外依赖（Ubuntu 22.04/24.04）

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
# AppImage 打包额外需要：
sudo apt install squashfs-tools libfuse2
```

## 本地开发

```bash
npm install          # 安装前端依赖
npm run tauri dev    # 启动开发服务器（需要显示环境）
```

## 构建

每次打包必须同时产出 Lite + Full Bundle 两个版本。

### Lite 版（在线下载模式）

```bash
OC_BUILD_BUNDLED=0 npm run tauri build
```

### Full Bundle 版（离线安装模式）

```bash
# 1. 下载内嵌资源（Node.js + openclaw.tgz）
bash scripts/fetch_resources.sh

# 2. 构建 Full Bundle
OC_BUILD_BUNDLED=1 npm run tauri build -- --features bundled

# AppImage 构建失败时加环境变量：
APPIMAGE_EXTRACT_AND_RUN=1 npm run tauri build -- --bundles appimage
```

### macOS 打包

macOS 不使用 Tauri，使用纯 bash `.command` 脚本：

```bash
NODE_ARCH=arm64 bash scripts/macos/build_bundle.sh   # Apple Silicon
NODE_ARCH=x86_64 bash scripts/macos/build_bundle.sh  # Intel
```

## 测试

```bash
npm test                                             # 前端 Vitest（27 tests）
cd src-tauri && DEP_TAURI_DEV=true cargo test        # Rust 单元测试（25 tests）
bats tests/macos/                                    # macOS bash 脚本测试（12 tests）

# 单独运行某个 Rust 测试
cd src-tauri && DEP_TAURI_DEV=true cargo test test_name -- --exact
```

## 内置二进制资源

Full Bundle 构建需要预先下载资源到 `resources/binaries/`：

```
resources/binaries/
├── linux/
│   ├── node            # Node.js Linux x64 二进制
│   └── openclaw.tgz    # openclaw 服务包（npm pack）
└── windows/
    ├── node.exe        # Node.js Windows x64 二进制
    └── openclaw.tgz    # openclaw 服务包（与 Linux 相同）
```

通过 `scripts/fetch_resources.sh` 自动从 npm registry 下载。

## 代码结构

```
src-tauri/src/           # Rust 后端
├── main.rs              # 入口 + 21 个 Tauri IPC commands
├── deploy.rs            # 11 步部署引擎（三模式）
├── system_check.rs      # OS/磁盘/端口检查（跨平台）
├── clash_proxy.rs       # Mihomo 临时代理管理
├── skills_manager.rs    # Skills .tgz 原子更新
├── updater.rs           # GitHub Release 更新 + 回滚
├── session_state.rs     # 断点续传状态
├── platform_config.rs   # 企业微信/钉钉/飞书配置
├── service_ctrl.rs      # 服务启停控制（systemctl / 直接进程）
└── tray.rs              # 系统托盘图标 + 30s 状态轮询

src/                     # Vue 3 前端
├── pages/               # 10 个向导页面
├── stores/              # Pinia stores（wizard + config）
├── composables/         # useTauri、useWizardNav
└── components/          # 公共组件

scripts/macos/           # macOS 纯 bash 安装脚本
```

## 版本发布

发布前同步更新三处版本号：
- `src-tauri/Cargo.toml` → `version`
- `src-tauri/tauri.conf.json` → `version`
- `package.json` → `version`

推送 `v*` tag 触发 `.github/workflows/release.yml`，自动构建三平台产物并创建 GitHub Release。
