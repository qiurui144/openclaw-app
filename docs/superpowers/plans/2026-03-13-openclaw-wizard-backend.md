# OpenClaw 向导重写 Plan A：项目脚手架 + Rust 后端

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用 Tauri 2.x + Rust 替换现有 Qt/C++ 实现，建立项目骨架并实现所有后端 Rust 模块（系统检查、部署引擎、Clash 代理、Skills 更新、GitHub ZIP 更新）。

**Architecture:** 保留现有 `resources/`、`scripts/` 和文档目录；删除 Qt 源码（`src/*.cpp/h`、`CMakeLists.txt`）；新建 Tauri 项目（`src-tauri/` 为 Rust 后端，`src/` 重用为 Vue 前端）。所有 Rust 模块通过 Tauri IPC command 暴露给前端，进度事件通过 `window.emit()` 推送。

**Tech Stack:** Rust 1.75+、Tauri 2.x、Cargo、tokio（async）、serde/serde_json、secrecy、reqwest、flate2（tar 解压）

**Spec:** `docs/superpowers/specs/2026-03-13-openclaw-wizard-redesign.md`

---

## Chunk 1：清理 Qt 遗产 + 初始化 Tauri 项目

### 文件变更总览

| 操作 | 路径 |
|---|---|
| 删除 | `src/` 目录下所有 `.cpp`、`.h` 文件 |
| 删除 | `CMakeLists.txt` |
| 删除 | `resources/styles/wizard.qss` |
| 保留 | `resources/icons/`、`resources/binaries/`、`resources/resources.qrc` |
| 新建 | `src-tauri/Cargo.toml` |
| 新建 | `src-tauri/src/main.rs` |
| 新建 | `src-tauri/tauri.conf.json` |
| 新建 | `src-tauri/build.rs` |
| 新建 | `src-tauri/capabilities/default.json` |
| 新建 | `package.json` |
| 新建 | `vite.config.ts` |
| 新建 | `index.html` |
| 新建 | `src/main.ts`（Vue 入口，Plan B 实现内容） |
| 修改 | `.gitignore` |

### Task 1：删除 Qt 文件

- [ ] **Step 1：删除 Qt 源码**
```bash
rm -f src/*.cpp src/*.h
rm -rf src/core src/pages
rm -f CMakeLists.txt
rm -f resources/styles/wizard.qss
rm -f build/debug/CMakeCache.txt build/release/CMakeCache.txt
```

- [ ] **Step 2：确认删除正确**
```bash
ls src/       # 应为空目录或不存在
ls CMakeLists.txt 2>&1  # 应报 No such file
```

- [ ] **Step 3：Commit 清理**
```bash
git add -A
git commit -m "chore: 移除 Qt/C++ 源码，为 Tauri 重写准备"
```

---

### Task 2：初始化 Tauri 2.x 项目骨架

- [ ] **Step 1：安装 Tauri CLI 和 Node 依赖**
```bash
# 确认 Node.js 可用（需 18+）
node --version

# 安装 Tauri CLI
cargo install tauri-cli --version "^2.0" 2>/dev/null || true

# 创建 package.json
cat > package.json << 'EOF'
{
  "name": "openclaw",
  "version": "1.0.0",
  "private": true,
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "tauri": "tauri"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0",
    "vite": "^5.0",
    "vue": "^3.4",
    "@vitejs/plugin-vue": "^5.0",
    "typescript": "^5.0"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0",
    "pinia": "^2.1",
    "qrcode": "^1.5"
  }
}
EOF
npm install
```

- [ ] **Step 2：创建 Vite 配置**
```bash
cat > vite.config.ts << 'EOF'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: ['es2021', 'chrome100', 'safari13'],
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
})
EOF
```

- [ ] **Step 3：创建 HTML 入口**
```bash
cat > index.html << 'EOF'
<!DOCTYPE html>
<html lang="zh">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>OpenClaw 部署向导</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
EOF
```

- [ ] **Step 4：创建 Vue 前端占位入口（Plan B 将实现完整内容）**
```bash
mkdir -p src
cat > src/main.ts << 'EOF'
// Plan B 将实现完整前端
import { createApp } from 'vue'
import { createPinia } from 'pinia'

const app = createApp({ template: '<div>OpenClaw Wizard - Loading...</div>' })
app.use(createPinia())
app.mount('#app')
EOF
```

- [ ] **Step 5：创建 src-tauri 目录和 Cargo.toml**
```bash
mkdir -p src-tauri/src src-tauri/capabilities
```

新建 `src-tauri/Cargo.toml`：
```toml
[package]
name = "openclaw"
version = "1.0.0"
edition = "2021"

[lib]
name = "openclaw_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[[bin]]
name = "openclaw"
path = "src/main.rs"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri-plugin-shell = "2"
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["stream", "json"] }
secrecy = { version = "0.8", features = ["serde"] }
flate2 = "1"
tar = "0.4"
xz2 = "0.1"  # G5 fix: 解压 Node.js Linux .tar.xz
sha2 = "0.10"
hex = "0.4"
semver = "1"
anyhow = "1"
bcrypt = "0.15"
chrono = { version = "0.4", features = ["serde"] }
dirs = "5"
sysinfo = "0.30"
zip = "2"
libc = "0.2"  # B3 fix: macOS SIGHUP via PID

# S1 fix: tauri 必须在各平台分别声明，不能在 [dependencies] 中重复定义
[target.'cfg(target_os = "linux")'.dependencies]
tauri = { version = "2", features = ["webkit2gtk-4-0"] }

[target.'cfg(target_os = "windows")'.dependencies]
tauri = { version = "2", features = [] }

[target.'cfg(target_os = "macos")'.dependencies]
tauri = { version = "2", features = [] }

[profile.release]
opt-level = 3
strip = true
lto = true
codegen-units = 1
```

- [ ] **Step 6：创建 build.rs**
```bash
cat > src-tauri/build.rs << 'EOF'
fn main() {
    tauri_build::build()
}
EOF
```

- [ ] **Step 7：创建 tauri.conf.json**

新建 `src-tauri/tauri.conf.json`：
```json
{
  "productName": "OpenClaw",
  "version": "1.0.0",
  "identifier": "io.openclaw.wizard",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "windows": [
      {
        "title": "OpenClaw 部署向导",
        "width": 860,
        "height": 620,
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
    "icon": ["../resources/icons/app.svg"],
    "linux": {
      "appimage": {
        "bundleMediaFramework": true
      }
    },
    "windows": {
      "webviewInstallMode": {
        "type": "embedBootstrapper",
        "silent": true
      },
      "nsis": {
        "minimumWebview2Version": "100.0.0.0",
        "installMode": "perMachine"
      }
    }
  }
}
```

- [ ] **Step 8：创建 capabilities 配置**

新建 `src-tauri/capabilities/default.json`：
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default capabilities",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "shell:allow-execute",
    "dialog:allow-open",
    "fs:allow-read-dir",
    "fs:allow-read-file",
    "fs:allow-write-file",
    "fs:allow-mkdir",
    "fs:allow-remove"
  ]
}
```

- [ ] **Step 9：验证 Tauri 项目可编译**
```bash
cd src-tauri
# 先写最小 main.rs（正式内容在 Task 3 完成）
cat > src/main.rs << 'EOF'
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
EOF
cargo check 2>&1 | tail -5
# 期望：无 error（可能有 warning）
cd ..
```

- [ ] **Step 10：更新 .gitignore**
```bash
cat >> .gitignore << 'EOF'

# Tauri
src-tauri/target/
src-tauri/gen/
dist/
node_modules/
.env
EOF
```

- [ ] **Step 11：Commit**
```bash
git add -A
git commit -m "feat: 初始化 Tauri 2.x 项目骨架，替换 Qt 构建系统"
```

---

## Chunk 2：Rust 系统检查模块（system_check.rs）

### 文件

| 操作 | 路径 |
|---|---|
| 新建 | `src-tauri/src/system_check.rs` |
| 修改 | `src-tauri/src/main.rs` |

### Task 3：实现 system_check.rs

- [ ] **Step 1：写失败测试**

新建 `src-tauri/src/system_check.rs`（测试先行）：
```rust
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct CheckItem {
    pub name: String,
    pub detail: String,
    pub passed: bool,
    pub required: bool,
}

pub async fn run_all_checks() -> Vec<CheckItem> {
    vec![
        check_os_version(),
        check_disk_space(),
        check_port_available(18789),
    ]
}

fn check_os_version() -> CheckItem { todo!() }
fn check_disk_space() -> CheckItem { todo!() }
fn check_port_available(_port: u16) -> CheckItem { todo!() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_item_serializable() {
        let item = CheckItem {
            name: "测试".into(),
            detail: "详情".into(),
            passed: true,
            required: true,
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("测试"));
    }

    #[test]
    fn test_disk_space_check_returns_item() {
        let item = check_disk_space();
        assert!(!item.name.is_empty());
        assert!(item.required);
        // 磁盘空间 >= 1MB 时必然通过（测试环境）
        assert!(item.passed, "测试环境磁盘应有足够空间");
    }

    #[test]
    fn test_port_check_high_port_likely_free() {
        // 用一个极不常用的高端口测试"未占用"分支
        let item = check_port_available(59999);
        assert_eq!(item.required, false);
        assert!(item.passed, "59999 端口应该未被占用");
    }
}
```

- [ ] **Step 2：运行测试，确认 todo!() 导致 panic**
```bash
cd src-tauri
cargo test system_check 2>&1 | grep -E "FAILED|error|todo"
# 期望：test_disk_space_check_returns_item 失败（todo!()）
cd ..
```

- [ ] **Step 3：实现 check_os_version（Linux）**

在 `system_check.rs` 替换 `check_os_version`：
```rust
fn check_os_version() -> CheckItem {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        // 读取 /etc/os-release 获取版本信息
        let detail = fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|s| {
                let version_id = s.lines()
                    .find(|l| l.starts_with("VERSION_ID="))
                    .map(|l| l.trim_start_matches("VERSION_ID=").trim_matches('"').to_string());
                let pretty = s.lines()
                    .find(|l| l.starts_with("PRETTY_NAME="))
                    .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string());
                pretty.or(version_id)
            })
            .unwrap_or_else(|| "未知 Linux 发行版".into());

        // 读取内核版本（uname -r）
        let kernel = std::process::Command::new("uname")
            .arg("-r")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();

        // 解析内核主版本号，Ubuntu 20.04 默认内核 5.4.x
        let major: u32 = kernel.split('.').next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let passed = major >= 5;

        CheckItem {
            name: "操作系统版本".into(),
            detail: format!("{} (kernel {})", detail, kernel),
            passed,
            required: true,
        }
    }
    #[cfg(target_os = "windows")]
    {
        // 读注册表 CurrentBuildNumber
        // Windows 10 1803 = build 17134
        let build: u32 = {
            use std::process::Command;
            let out = Command::new("reg")
                .args(["query",
                    r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion",
                    "/v", "CurrentBuildNumber"])
                .output()
                .ok();
            out.and_then(|o| {
                String::from_utf8(o.stdout).ok()
                    .and_then(|s| s.lines()
                        .find(|l| l.contains("CurrentBuildNumber"))
                        .and_then(|l| l.split_whitespace().last()
                            .and_then(|v| v.parse().ok())))
            }).unwrap_or(0)
        };
        let passed = build >= 17134; // Windows 10 1803
        let label = if build >= 22000 { "Windows 11" } else { "Windows 10" };
        CheckItem {
            name: "操作系统版本".into(),
            detail: format!("{} (Build {})", label, build),
            passed,
            required: true,
        }
    }
    #[cfg(target_os = "macos")]
    {
        CheckItem {
            name: "操作系统版本".into(),
            detail: "macOS（bash 脚本模式，不适用）".into(),
            passed: true,
            required: true,
        }
    }
}
```

- [ ] **Step 4：实现 check_disk_space**
```rust
fn check_disk_space() -> CheckItem {
    let path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/"));

    let available_mb = {
        #[cfg(unix)]
        {
            use std::mem::MaybeUninit;
            let path_cstr = std::ffi::CString::new(
                path.to_str().unwrap_or("/")
            ).unwrap();
            let mut stat: MaybeUninit<libc::statvfs> = MaybeUninit::uninit();
            let ret = unsafe { libc::statvfs(path_cstr.as_ptr(), stat.as_mut_ptr()) };
            if ret == 0 {
                let s = unsafe { stat.assume_init() };
                (s.f_bavail * s.f_frsize) / (1024 * 1024)
            } else { 0 }
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            let wide: Vec<u16> = path.as_os_str().encode_wide()
                .chain(std::iter::once(0)).collect();
            let mut free: u64 = 0;
            let mut total: u64 = 0;
            let mut total_free: u64 = 0;
            unsafe {
                windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
                    wide.as_ptr(), &mut free, &mut total, &mut total_free
                );
            }
            free / (1024 * 1024)
        }
    };

    let passed = available_mb >= 512;
    CheckItem {
        name: "磁盘空间".into(),
        detail: format!("可用 {} MB（需要 ≥ 512 MB）", available_mb),
        passed,
        required: true,
    }
}
```

> **注意**：Linux 需在 `Cargo.toml` 添加 `libc = "0.2"`；Windows 需添加 `windows-sys = { version = "0.52", features = ["Win32_Storage_FileSystem"] }`。

- [ ] **Step 5：实现 check_port_available**
```rust
fn check_port_available(port: u16) -> CheckItem {
    use std::net::TcpListener;
    let passed = TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok();
    CheckItem {
        name: format!("端口 {} 可用", port),
        detail: if passed {
            format!("端口 {} 未被占用", port)
        } else {
            format!("端口 {} 已被占用，将在部署时尝试停止冲突服务", port)
        },
        passed,
        required: false,
    }
}
```

- [ ] **Step 6：更新 Cargo.toml 添加新依赖**

在 `src-tauri/Cargo.toml` 的 `[dependencies]` 下追加：
```toml
libc = "0.2"

[target.'cfg(target_os = "windows")'.dependencies]
windows-sys = { version = "0.52", features = [
    "Win32_Storage_FileSystem",
    "Win32_Foundation",
] }
```

- [ ] **Step 7：运行测试，确认全部通过**
```bash
cd src-tauri
cargo test system_check 2>&1 | grep -E "test .* (ok|FAILED)"
# 期望：3 个 ok
cd ..
```

- [ ] **Step 8：在 main.rs 注册 Tauri command**

修改 `src-tauri/src/main.rs`：
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod system_check;

#[tauri::command]
async fn run_system_check() -> Vec<system_check::CheckItem> {
    system_check::run_all_checks().await
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![run_system_check])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 9：cargo check 确认编译无误**
```bash
cd src-tauri && cargo check 2>&1 | grep "^error" | head -5
# 期望：无 error
cd ..
```

- [ ] **Step 10：Commit**
```bash
git add src-tauri/src/system_check.rs src-tauri/src/main.rs src-tauri/Cargo.toml
git commit -m "feat(backend): 实现系统检查模块（OS版本/磁盘/端口）"
```

---

## Chunk 3：会话状态 + 平台配置模块

### 文件

| 操作 | 路径 |
|---|---|
| 新建 | `src-tauri/src/session_state.rs` |
| 新建 | `src-tauri/src/platform_config.rs` |
| 修改 | `src-tauri/src/main.rs` |

### Task 4：session_state.rs

- [ ] **Step 1：写测试**

新建 `src-tauri/src/session_state.rs`：
```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::Result;

/// 持久化安装会话，供断点续传使用。
/// 严格不包含 admin_password 字段。
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct SessionState {
    pub source_mode_tag: String,       // "Bundled" | "Online" | "LocalZip"
    pub local_zip_path: Option<String>,
    pub proxy_url: Option<String>,     // Clash 实际代理地址，非订阅 URL
    pub downloaded_files: Vec<DownloadedFile>,
    pub completed_step: u32,           // 已完成的部署步骤编号（0 = 未开始）
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DownloadedFile {
    pub key: String,    // "node" | "openclaw"
    pub path: String,
    pub sha256: String,
}

fn session_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw")
        .join("install_session.json")
}

pub fn load() -> Option<SessionState> {
    let path = session_path();
    if !path.exists() { return None; }
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save(state: &SessionState) -> Result<()> {
    let path = session_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(state)?)?;
    Ok(())
}

pub fn clear(also_cleanup_tmp: Option<&str>) -> Result<()> {
    let path = session_path();
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    // 清理临时下载目录
    if let Some(install_path) = also_cleanup_tmp {
        let tmp = PathBuf::from(install_path).join(".tmp");
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn with_temp_home<F: FnOnce()>(f: F) {
        let tmp = env::temp_dir().join(format!("oc_test_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        // 重定向 HOME 环境变量（Unix）
        #[cfg(unix)]
        env::set_var("HOME", &tmp);
        f();
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        with_temp_home(|| {
            let state = SessionState {
                source_mode_tag: "Bundled".into(),
                completed_step: 3,
                ..Default::default()
            };
            save(&state).unwrap();
            let loaded = load().expect("should load");
            assert_eq!(loaded.source_mode_tag, "Bundled");
            assert_eq!(loaded.completed_step, 3);
        });
    }

    #[test]
    fn test_clear_removes_file() {
        with_temp_home(|| {
            let state = SessionState::default();
            save(&state).unwrap();
            clear(None).unwrap();
            assert!(load().is_none());
        });
    }

    #[test]
    fn test_load_returns_none_when_no_file() {
        with_temp_home(|| {
            assert!(load().is_none());
        });
    }

    #[test]
    fn test_no_password_field_in_serialized() {
        let state = SessionState::default();
        let json = serde_json::to_string(&state).unwrap();
        // 确保序列化结果中绝对不含 password 相关字段
        assert!(!json.to_lowercase().contains("password"));
        assert!(!json.to_lowercase().contains("passwd"));
    }
}
```

- [ ] **Step 2：运行测试**
```bash
cd src-tauri
cargo test session_state 2>&1 | grep -E "test .* (ok|FAILED)"
# 期望：4 个 ok
cd ..
```

- [ ] **Step 3：Commit**
```bash
git add src-tauri/src/session_state.rs
git commit -m "feat(backend): 实现 session_state 模块（断点续传，密码字段安全排除）"
```

---

### Task 5：platform_config.rs

- [ ] **Step 1：写测试**

新建 `src-tauri/src/platform_config.rs`：
```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    WeWork,
    QqWork,
    DingTalk,
    Feishu,
}

impl Platform {
    pub fn channel_key(&self) -> &'static str {
        match self {
            Platform::WeWork => "wecom",
            Platform::QqWork => "qqwork",
            Platform::DingTalk => "dingtalk",
            Platform::Feishu => "feishu",
        }
    }
    /// 各平台机器人配置官方文档 URL（用于 QR 码）
    pub fn doc_url(&self) -> &'static str {
        match self {
            Platform::WeWork =>
                "https://work.weixin.qq.com/api/doc/90000/90136/91770",
            Platform::QqWork =>
                "https://work.qq.com/",
            Platform::DingTalk =>
                "https://open.dingtalk.com/document/robots/custom-robot-access",
            Platform::Feishu =>
                "https://open.feishu.cn/document/client-docs/bot-v3/add-custom-bot",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlatformEntry {
    pub platform: Platform,
    pub webhook_url: String,
}

/// 向 openclaw.json 写入 channels 配置节
pub fn write_platform_config(
    install_path: &str,
    platforms: &[PlatformEntry],
) -> Result<()> {
    if platforms.is_empty() { return Ok(()); }

    let config_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw")
        .join("openclaw.json");

    let mut config: serde_json::Value = if config_path.exists() {
        let data = std::fs::read_to_string(&config_path)?;
        serde_json::from_str(&data)?
    } else {
        serde_json::json!({})
    };

    let channels = config["channels"]
        .as_object_mut()
        .cloned()
        .unwrap_or_default();
    let mut channels_map = channels;

    for entry in platforms {
        channels_map.insert(
            entry.platform.channel_key().to_string(),
            serde_json::json!({ "webhookUrl": entry.webhook_url }),
        );
    }

    config["channels"] = serde_json::Value::Object(channels_map);
    let _ = install_path; // 仅用于日志，实际写到 ~/.openclaw/

    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn with_temp_home<F: FnOnce()>(f: F) {
        let tmp = env::temp_dir()
            .join(format!("oc_pf_test_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        #[cfg(unix)]
        env::set_var("HOME", &tmp);
        f();
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_platform_channel_key() {
        assert_eq!(Platform::Feishu.channel_key(), "feishu");
        assert_eq!(Platform::DingTalk.channel_key(), "dingtalk");
        assert_eq!(Platform::WeWork.channel_key(), "wecom");
    }

    #[test]
    fn test_doc_url_not_empty() {
        for p in [Platform::WeWork, Platform::QqWork,
                  Platform::DingTalk, Platform::Feishu] {
            assert!(!p.doc_url().is_empty());
            assert!(p.doc_url().starts_with("https://"));
        }
    }

    #[test]
    fn test_write_platform_config_creates_channels() {
        with_temp_home(|| {
            // 先创建基础配置文件
            let dir = dirs::home_dir().unwrap().join(".openclaw");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("openclaw.json"),
                r#"{"gateway":{"port":18789}}"#
            ).unwrap();

            let entries = vec![
                PlatformEntry {
                    platform: Platform::Feishu,
                    webhook_url: "https://open.feishu.cn/hook/test".into(),
                },
            ];
            write_platform_config("/tmp/install", &entries).unwrap();

            let data = std::fs::read_to_string(
                dir.join("openclaw.json")
            ).unwrap();
            let v: serde_json::Value = serde_json::from_str(&data).unwrap();
            assert_eq!(
                v["channels"]["feishu"]["webhookUrl"],
                "https://open.feishu.cn/hook/test"
            );
            // 原有配置保留
            assert_eq!(v["gateway"]["port"], 18789);
        });
    }

    #[test]
    fn test_write_empty_platforms_is_noop() {
        with_temp_home(|| {
            // 无平台配置时不创建文件
            let result = write_platform_config("/tmp/install", &[]);
            assert!(result.is_ok());
            let cfg = dirs::home_dir().unwrap()
                .join(".openclaw").join("openclaw.json");
            assert!(!cfg.exists());
        });
    }
}
```

- [ ] **Step 2：运行测试**
```bash
cd src-tauri
cargo test platform_config 2>&1 | grep -E "test .* (ok|FAILED)"
# 期望：4 个 ok
cd ..
```

- [ ] **Step 3：Commit**
```bash
git add src-tauri/src/platform_config.rs src-tauri/src/session_state.rs
git commit -m "feat(backend): 实现 platform_config 模块（channels 写入，含 QR 码 URL）"
```

---

## Chunk 4：部署引擎（deploy.rs）

### 文件

| 操作 | 路径 |
|---|---|
| 新建 | `src-tauri/src/deploy.rs` |
| 修改 | `src-tauri/src/main.rs` |

### Task 6：DeployConfig 数据结构 + IPC DTO

- [ ] **Step 1：新建 deploy.rs，定义数据结构并写测试**

新建 `src-tauri/src/deploy.rs`：
```rust
use anyhow::Result;
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Window;

use crate::platform_config::PlatformEntry;
use crate::session_state::{SessionState, DownloadedFile};

// ── IPC DTO：从前端接收，立即转换为内部类型 ──────────────────────
#[derive(Debug, Clone, Deserialize)]
pub struct DeployConfigDto {
    pub install_path: String,
    pub service_port: u16,
    pub admin_password: String,   // 仅在 IPC boundary 使用
    pub domain_name: Option<String>,
    pub install_service: bool,
    pub start_on_boot: bool,
    pub source_mode: SourceModeDto,
    pub platforms: Vec<PlatformEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum SourceModeDto {
    Bundled,
    Online { proxy_url: Option<String> },
    LocalZip { path: String },
}

// ── 内部安全类型：admin_password 使用 Secret<String> ─────────────
pub struct DeployConfig {
    pub install_path: String,
    pub service_port: u16,
    pub admin_password: Secret<String>,
    pub domain_name: Option<String>,
    pub install_service: bool,
    pub start_on_boot: bool,
    pub source_mode: SourceMode,
    pub platforms: Vec<PlatformEntry>,
}

pub enum SourceMode {
    Bundled,
    /// proxy_url = "http://127.0.0.1:{clash_port}"（Clash 启动后填入）
    Online { proxy_url: Option<String> },
    LocalZip(PathBuf),
}

impl From<DeployConfigDto> for DeployConfig {
    fn from(dto: DeployConfigDto) -> Self {
        DeployConfig {
            install_path: dto.install_path,
            service_port: dto.service_port,
            admin_password: Secret::new(dto.admin_password),
            domain_name: dto.domain_name,
            install_service: dto.install_service,
            start_on_boot: dto.start_on_boot,
            source_mode: match dto.source_mode {
                SourceModeDto::Bundled => SourceMode::Bundled,
                SourceModeDto::Online { proxy_url } =>
                    SourceMode::Online { proxy_url },
                SourceModeDto::LocalZip { path } =>
                    SourceMode::LocalZip(PathBuf::from(path)),
            },
            platforms: dto.platforms,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct DeployProgress {
    pub step: u32,
    pub total: u32,
    pub percent: u32,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dto_to_config_conversion() {
        let dto = DeployConfigDto {
            install_path: "/opt/openclaw".into(),
            service_port: 18789,
            admin_password: "Secret123".into(),
            domain_name: None,
            install_service: true,
            start_on_boot: true,
            source_mode: SourceModeDto::Bundled,
            platforms: vec![],
        };
        let config = DeployConfig::from(dto);
        // 密码已包装为 Secret，不可直接读取（需 expose_secret()）
        assert_eq!(config.admin_password.expose_secret(), "Secret123");
        assert_eq!(config.service_port, 18789);
    }

    #[test]
    fn test_deploy_progress_serializable() {
        let p = DeployProgress {
            step: 1,
            total: 11,
            percent: 9,
            message: "创建安装目录".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("创建安装目录"));
        assert!(json.contains("\"percent\":9"));
    }

    #[test]
    fn test_source_mode_local_zip_path() {
        let dto = DeployConfigDto {
            install_path: "/opt/openclaw".into(),
            service_port: 18789,
            admin_password: "Secret123".into(),
            domain_name: None,
            install_service: false,
            start_on_boot: false,
            source_mode: SourceModeDto::LocalZip {
                path: "/tmp/openclaw.zip".into()
            },
            platforms: vec![],
        };
        let config = DeployConfig::from(dto);
        assert!(matches!(config.source_mode, SourceMode::LocalZip(_)));
    }
}
```

- [ ] **Step 2：运行测试**
```bash
cd src-tauri
cargo test deploy 2>&1 | grep -E "test .* (ok|FAILED)"
# 期望：3 个 ok
cd ..
```

- [ ] **Step 3：Commit**
```bash
git add src-tauri/src/deploy.rs
git commit -m "feat(backend): 定义 DeployConfig DTO + 内部安全类型（secrecy::Secret）"
```

---

### Task 7：部署引擎核心逻辑（11 步）

- [ ] **Step 1：在 deploy.rs 添加 start_deploy 函数骨架**

在 `deploy.rs` 末尾（tests 模块之前）追加：
```rust
/// 向前端发送进度事件
fn emit_progress(window: &Window, step: u32, total: u32, msg: &str) {
    let _ = window.emit("deploy:progress", DeployProgress {
        step,
        total,
        percent: (step * 100) / total,
        message: msg.to_string(),
    });
}

pub async fn start_deploy(config: DeployConfig, window: Window) -> Result<()> {
    const TOTAL: u32 = 11;

    // Step 1: 创建安装目录
    emit_progress(&window, 1, TOTAL, "创建安装目录…");
    std::fs::create_dir_all(&config.install_path)?;

    // Step 2-4: 获取并解包资源（因 SourceMode 不同行为差异）
    emit_progress(&window, 2, TOTAL, "获取 Node.js 运行时…");
    acquire_node(&config).await?;

    emit_progress(&window, 3, TOTAL, "获取 OpenClaw 安装包…");
    acquire_openclaw_package(&config).await?;

    emit_progress(&window, 4, TOTAL, "解包 OpenClaw（含所有依赖，请稍候）…");
    extract_openclaw(&config)?;

    // Step 5: 写入主配置
    emit_progress(&window, 5, TOTAL, "写入配置文件…");
    write_main_config(&config)?;

    // Step 6: 写入平台集成配置
    emit_progress(&window, 6, TOTAL, "写入平台集成配置…");
    crate::platform_config::write_platform_config(
        &config.install_path, &config.platforms)?;

    // Step 7: 注册系统服务
    if config.install_service {
        emit_progress(&window, 7, TOTAL, "注册系统服务…");
        install_service(&config)?;
    }

    // Step 8: 启动服务
    emit_progress(&window, 8, TOTAL, "启动 OpenClaw 服务…");
    start_service(&config)?;

    // Step 9: 健康检查
    emit_progress(&window, 9, TOTAL, "等待服务就绪…");
    health_check(config.service_port).await?;

    // Step 10: 生成卸载脚本
    emit_progress(&window, 10, TOTAL, "生成卸载脚本…");
    write_uninstall_script(&config)?;

    // Step 11: 写入安装记录
    emit_progress(&window, 11, TOTAL, "完成！");
    write_deploy_meta(&config)?;
    crate::session_state::clear(Some(&config.install_path))?;

    let _ = window.emit("deploy:done", ());
    Ok(())
}

// ── 各步骤实现 ────────────────────────────────────────────────────

async fn acquire_node(config: &DeployConfig) -> Result<()> {
    let node_dir = PathBuf::from(&config.install_path).join("node");
    std::fs::create_dir_all(&node_dir)?;
    let dest = node_dir.join(if cfg!(windows) { "node.exe" } else { "node" });

    match &config.source_mode {
        SourceMode::Bundled => {
            // S4 fix: include_bytes! 是编译期宏，仅在 "bundled" feature 启用时使用。
            // 在 Cargo.toml 中声明：[features] bundled = []
            // Full Bundle 构建命令：cargo build --features bundled
            // 开发/CI 无资源时用 Online 或 LocalZip 模式。
            #[cfg(feature = "bundled")]
            {
                #[cfg(target_os = "linux")]
                let data = include_bytes!("../../resources/binaries/linux/node");
                #[cfg(target_os = "windows")]
                let data = include_bytes!("..\\..\\resources\\binaries\\windows\\node.exe");
                std::fs::write(&dest, &data[..])?;
            }
            #[cfg(not(feature = "bundled"))]
            anyhow::bail!("Bundled 模式需要使用 --features bundled 编译（资源文件未内嵌）");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&dest,
                    std::fs::Permissions::from_mode(0o755))?;
            }
        }
        SourceMode::Online { proxy_url } => {
            download_node_binary(&dest, proxy_url.as_deref()).await?;
        }
        SourceMode::LocalZip(zip_path) => {
            extract_from_zip(zip_path, "node", &dest)?;
        }
    }
    Ok(())
}

async fn acquire_openclaw_package(config: &DeployConfig) -> Result<()> {
    let dest = PathBuf::from(&config.install_path).join("openclaw.tgz");
    match &config.source_mode {
        SourceMode::Bundled => {
            #[cfg(feature = "bundled")]
            {
                #[cfg(target_os = "linux")]
                let data = include_bytes!("../../resources/binaries/linux/openclaw.tgz");
                #[cfg(target_os = "windows")]
                let data = include_bytes!("..\\..\\resources\\binaries\\windows\\openclaw.tgz");
                std::fs::write(&dest, &data[..])?;
            }
            #[cfg(not(feature = "bundled"))]
            anyhow::bail!("Bundled 模式需要使用 --features bundled 编译");
        }
        SourceMode::Online { proxy_url } => {
            download_openclaw_package(&dest, proxy_url.as_deref()).await?;
        }
        SourceMode::LocalZip(zip_path) => {
            extract_from_zip(zip_path, "openclaw.tgz", &dest)?;
        }
    }
    Ok(())
}

fn extract_openclaw(config: &DeployConfig) -> Result<()> {
    let tgz = PathBuf::from(&config.install_path).join("openclaw.tgz");
    let dest = PathBuf::from(&config.install_path).join("openclaw_pkg");
    std::fs::create_dir_all(&dest)?;

    let file = std::fs::File::open(&tgz)?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);
    archive.unpack(&dest)?;
    std::fs::remove_file(&tgz)?;
    Ok(())
}

fn write_main_config(config: &DeployConfig) -> Result<()> {
    use secrecy::ExposeSecret;
    let config_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw");
    std::fs::create_dir_all(&config_dir)?;

    let password_hash = bcrypt::hash(
        config.admin_password.expose_secret(), bcrypt::DEFAULT_COST)?;

    let mut gateway = serde_json::json!({
        "port": config.service_port,
        "auth": {
            "mode": "password",
            "passwordHash": password_hash
        }
    });
    if let Some(domain) = &config.domain_name {
        gateway["publicUrl"] = serde_json::json!(format!("https://{}", domain));
    }

    let cfg = serde_json::json!({ "gateway": gateway });
    std::fs::write(
        config_dir.join("openclaw.json"),
        serde_json::to_string_pretty(&cfg)?
    )?;
    Ok(())
}

fn install_service(config: &DeployConfig) -> Result<()> {
    let node_bin = PathBuf::from(&config.install_path)
        .join("node")
        .join(if cfg!(windows) { "node.exe" } else { "node" });
    let script = PathBuf::from(&config.install_path)
        .join("openclaw_pkg")
        .join("node_modules")
        .join("openclaw")
        .join("openclaw.mjs");
    let port = config.service_port;

    #[cfg(target_os = "linux")]
    {
        let unit_dir = dirs::home_dir().unwrap().join(".config/systemd/user");
        std::fs::create_dir_all(&unit_dir)?;
        let unit = format!(
            "[Unit]\nDescription=OpenClaw Gateway\nAfter=network.target\n\n\
             [Service]\nType=simple\n\
             ExecStart={} {} gateway --port {}\n\
             Restart=on-failure\nRestartSec=10\n\
             Environment=NODE_ENV=production\n\
             StandardOutput=journal\nStandardError=journal\n\n\
             [Install]\nWantedBy=default.target\n",
            node_bin.display(), script.display(), port
        );
        std::fs::write(unit_dir.join("openclaw.service"), unit)?;
        std::process::Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status()?;
        if config.start_on_boot {
            std::process::Command::new("systemctl")
                .args(["--user", "enable", "openclaw.service"])
                .status()?;
        }
    }
    #[cfg(target_os = "windows")]
    {
        let cmd = format!("\"{}\" \"{}\" gateway --port {}",
            node_bin.display(), script.display(), port);
        std::process::Command::new("schtasks")
            .args(["/Create", "/F",
                "/SC", "ONLOGON",
                "/TN", "OpenClaw Gateway",
                "/TR", &cmd])
            .status()?;
    }
    Ok(())
}

fn start_service(config: &DeployConfig) -> Result<()> {
    #[cfg(target_os = "linux")]
    std::process::Command::new("systemctl")
        .args(["--user", "start", "openclaw.service"])
        .status()?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("schtasks")
        .args(["/Run", "/TN", "OpenClaw Gateway"])
        .status()?;
    let _ = config;
    Ok(())
}

pub async fn health_check(port: u16) -> Result<()> {
    use std::net::TcpStream;
    let url = format!("http://127.0.0.1:{}/health", port);
    for i in 0..15 {
        // has_listener = true 表示端口有进程在监听（TcpStream::connect 成功）
        let has_listener = TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok();
        if has_listener {
            // 端口有监听者，尝试 HTTP 健康检查
            if reqwest::get(&url).await
                .map(|r| r.status().is_success())
                .unwrap_or(false)
            {
                return Ok(());
            }
        }
        if i < 14 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
    // 30s 超时后检查端口占用情况，给出更精准的错误信息
    let has_listener = TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok();
    if has_listener {
        // 端口有响应但健康检查失败，可能是其他进程占用
        use sysinfo::{System, ProcessesToUpdate};
        let mut sys = System::new();
        sys.refresh_processes(ProcessesToUpdate::All);
        let occupant = sys.processes().values()
            .find(|p| p.name().to_string_lossy().contains("openclaw"))
            .map(|p| format!("PID {}", p.pid()))
            .unwrap_or_else(|| "其他进程".into());
        anyhow::bail!(
            "端口 {} 被 {} 占用但未响应健康检查，请检查端口冲突", port, occupant
        )
    }
    anyhow::bail!("服务在 30 秒内未能正常启动，请查看日志")
}

fn write_uninstall_script(config: &DeployConfig) -> Result<()> {
    #[cfg(unix)]
    {
        let script = format!(
            "#!/bin/bash\nset -e\n\
             systemctl --user stop openclaw.service 2>/dev/null || true\n\
             systemctl --user disable openclaw.service 2>/dev/null || true\n\
             rm -f ~/.config/systemd/user/openclaw.service\n\
             systemctl --user daemon-reload\n\
             rm -rf {}\n\
             rm -rf ~/.openclaw\n\
             echo '✓ OpenClaw 已卸载'\n",
            config.install_path
        );
        let path = PathBuf::from(&config.install_path).join("uninstall.sh");
        std::fs::write(&path, script)?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(windows)]
    {
        let script = format!(
            "@echo off\r\n\
             schtasks /End /TN \"OpenClaw Gateway\" 2>nul\r\n\
             schtasks /Delete /TN \"OpenClaw Gateway\" /F 2>nul\r\n\
             rmdir /S /Q \"{}\"\r\n\
             rmdir /S /Q \"%USERPROFILE%\\.openclaw\"\r\n\
             echo OpenClaw 已卸载\r\n",
            config.install_path
        );
        std::fs::write(
            PathBuf::from(&config.install_path).join("uninstall.bat"),
            script
        )?;
    }
    Ok(())
}

fn write_deploy_meta(config: &DeployConfig) -> Result<()> {
    let meta_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw");
    let meta = serde_json::json!({
        "install_path": config.install_path,
        "version": env!("CARGO_PKG_VERSION"),
        "deployed_at": chrono::Utc::now().to_rfc3339(),
        "service_port": config.service_port,
    });
    std::fs::write(
        meta_dir.join("deploy_meta.json"),
        serde_json::to_string_pretty(&meta)?
    )?;
    Ok(())
}

// ── 网络下载辅助（Online 模式）────────────────────────────────────

async fn download_node_binary(dest: &PathBuf, proxy: Option<&str>) -> Result<()> {
    let version = "22.17.0";
    #[cfg(target_os = "linux")]
    let url = format!(
        "https://nodejs.org/dist/v{}/node-v{}-linux-x64.tar.xz", version, version);
    #[cfg(target_os = "windows")]
    let url = format!(
        "https://nodejs.org/dist/v{}/node-v{}-win-x64.zip", version, version);

    let client = build_client(proxy)?;
    let bytes = client.get(&url).send().await?.bytes().await?;
    // G5 fix: 下载到临时文件，解压后将 node 二进制写到 dest
    let archive_tmp = dest.with_file_name("node_archive.tmp");
    std::fs::write(&archive_tmp, &bytes)?;
    extract_node_from_archive(&archive_tmp, dest)?;
    std::fs::remove_file(&archive_tmp).ok();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dest, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

fn extract_node_from_archive(archive: &PathBuf, dest: &PathBuf) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        // .tar.xz: 找 bin/node 条目
        let data = std::fs::read(archive)?;
        let xz = xz2::read::XzDecoder::new(std::io::Cursor::new(data));
        let mut tar = tar::Archive::new(xz);
        for entry in tar.entries()? {
            let mut e = entry?;
            let path = e.path()?.into_owned();
            if path.ends_with("bin/node") {
                e.unpack(dest)?;
                return Ok(());
            }
        }
        anyhow::bail!("Node.js 压缩包中未找到 bin/node");
    }
    #[cfg(target_os = "windows")]
    {
        // .zip: 找 node.exe 条目
        let data = std::fs::read(archive)?;
        let cursor = std::io::Cursor::new(data);
        let mut zip = zip::ZipArchive::new(cursor)?;
        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            if file.name().ends_with("node.exe") {
                let mut out = std::fs::File::create(dest)?;
                std::io::copy(&mut file, &mut out)?;
                return Ok(());
            }
        }
        anyhow::bail!("Node.js 压缩包中未找到 node.exe");
    }
}

async fn download_openclaw_package(dest: &PathBuf, proxy: Option<&str>) -> Result<()> {
    // 从 GitHub Release 下载 openclaw.tgz
    let url = "https://github.com/openclaw/openclaw/releases/latest/download/openclaw.tgz";
    let client = build_client(proxy)?;
    let bytes = client.get(url).send().await?.bytes().await?;
    std::fs::write(dest, &bytes)?;
    Ok(())
}

fn build_client(proxy: Option<&str>) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300));
    if let Some(proxy_url) = proxy {
        builder = builder.proxy(reqwest::Proxy::all(proxy_url)?);
    }
    Ok(builder.build()?)
}

fn extract_from_zip(zip_path: &PathBuf, entry_name: &str, dest: &PathBuf) -> Result<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.name().ends_with(entry_name) {
            let mut data = Vec::new();
            std::io::copy(&mut entry, &mut data)?;
            std::fs::write(dest, data)?;
            return Ok(());
        }
    }
    anyhow::bail!("ZIP 中未找到 {}", entry_name)
}
```

- [ ] **Step 2：运行测试**
```bash
cd src-tauri
cargo test deploy 2>&1 | grep -E "test .* (ok|FAILED)"
# 期望：3 个 ok
cd ..
```

- [ ] **Step 3：在 main.rs 注册 command**

修改 `src-tauri/src/main.rs`，添加：
```rust
mod deploy;
mod platform_config;
mod session_state;

#[tauri::command]
async fn start_deploy(
    config: deploy::DeployConfigDto,
    window: tauri::Window
) -> Result<(), String> {
    let cfg = deploy::DeployConfig::from(config);
    deploy::start_deploy(cfg, window).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn health_check(port: u16) -> Result<(), String> {
    deploy::health_check(port).await.map_err(|e| e.to_string())
}

#[tauri::command]
fn load_session() -> Option<session_state::SessionState> {
    session_state::load()
}

#[tauri::command]
fn clear_session(install_path: Option<String>) -> Result<(), String> {
    session_state::clear(install_path.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_platform_doc_url(platform: String) -> String {
    use platform_config::Platform;
    let p = match platform.as_str() {
        "WeWork" => Platform::WeWork,
        "QqWork" => Platform::QqWork,
        "DingTalk" => Platform::DingTalk,
        _ => Platform::Feishu,
    };
    p.doc_url().to_string()
}
```

更新 `generate_handler!` 宏：
```rust
.invoke_handler(tauri::generate_handler![
    run_system_check,
    start_deploy,
    health_check,
    load_session,
    clear_session,
    get_platform_doc_url,
])
```

- [ ] **Step 4：cargo check**
```bash
cd src-tauri && cargo check 2>&1 | grep "^error" | head -5
cd ..
```

- [ ] **Step 5：Commit**
```bash
git add src-tauri/src/deploy.rs src-tauri/src/main.rs
git commit -m "feat(backend): 实现部署引擎（11步，三模式，健康检查，卸载脚本）"
```

---

## Chunk 5：Clash 代理模块（clash_proxy.rs）

### 文件

| 操作 | 路径 |
|---|---|
| 新建 | `src-tauri/src/clash_proxy.rs` |
| 修改 | `src-tauri/src/main.rs` |

### Task 8：Mihomo 进程管理

- [ ] **Step 1：写测试**

新建 `src-tauri/src/clash_proxy.rs`：
```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::process::Child;

static CLASH_PROCESS: OnceLock<Mutex<Option<Child>>> = OnceLock::new();

fn process_lock() -> &'static Mutex<Option<Child>> {
    CLASH_PROCESS.get_or_init(|| Mutex::new(None))
}

#[derive(Debug, Serialize)]
pub struct ClashTestResult {
    pub success: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

/// Mihomo 二进制路径（内置资源）
fn mihomo_bin_path() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));

    #[cfg(target_os = "linux")]
    return exe_dir.join("clash").join("mihomo-linux-amd64");
    #[cfg(target_os = "windows")]
    return exe_dir.join("clash").join("mihomo-windows-amd64.exe");
    #[cfg(target_os = "macos")]
    {
        // B2 fix: 运行期检测 CPU 架构（Intel x86_64 vs Apple Silicon arm64）
        let arch = match std::env::consts::ARCH {
            "x86_64" => "amd64",
            _ => "arm64",
        };
        return exe_dir.join("clash").join(format!("mihomo-darwin-{}", arch));
    }
}

/// 保存订阅 URL 供 Skills 更新复用
fn save_subscription_url(url: &str) -> Result<()> {
    let path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw")
        .join("proxy.json");
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::write(&path, serde_json::json!({
        "subscription_url": url
    }).to_string())?;
    Ok(())
}

pub fn load_subscription_url() -> Option<String> {
    let path = dirs::home_dir()?
        .join(".openclaw")
        .join("proxy.json");
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<serde_json::Value>(&data).ok()?
        ["subscription_url"].as_str().map(String::from)
}

/// 启动 Mihomo，返回代理地址 "http://127.0.0.1:7890"
pub async fn start(subscription_url: &str) -> Result<String> {
    // 1. 下载订阅配置
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let config_data = client.get(subscription_url).send().await?.text().await?;

    // 2. 写临时配置文件
    let config_dir = std::env::temp_dir().join("openclaw_clash");
    std::fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.yaml");
    std::fs::write(&config_path, &config_data)?;

    // 3. 启动 Mihomo 进程
    let bin = mihomo_bin_path();
    anyhow::ensure!(bin.exists(),
        "Mihomo 二进制不存在: {}", bin.display());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin,
            std::fs::Permissions::from_mode(0o755))?;
    }

    let child = std::process::Command::new(&bin)
        .args(["-d", config_dir.to_str().unwrap()])
        .spawn()?;

    *process_lock().lock().unwrap() = Some(child);

    // 4. 等待 Mihomo 启动（最多 5s）
    let proxy_addr = "http://127.0.0.1:7890".to_string();
    for _ in 0..10 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if test_proxy(&proxy_addr).await.success { break; }
    }

    save_subscription_url(subscription_url)?;
    Ok(proxy_addr)
}

/// 停止 Mihomo 进程并删除二进制
pub fn stop() -> Result<()> {
    if let Ok(mut guard) = process_lock().lock() {
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    // 删除 Mihomo 二进制
    let bin = mihomo_bin_path();
    if bin.exists() {
        std::fs::remove_file(&bin).ok();
    }
    // 清理临时配置
    let config_dir = std::env::temp_dir().join("openclaw_clash");
    std::fs::remove_dir_all(&config_dir).ok();
    Ok(())
}

/// 测试代理延迟（连接 https://github.com）
pub async fn test_proxy(proxy_url: &str) -> ClashTestResult {
    let start = std::time::Instant::now();
    let result = reqwest::Client::builder()
        .proxy(match reqwest::Proxy::all(proxy_url) {
            Ok(p) => p,
            Err(e) => return ClashTestResult {
                success: false,
                latency_ms: None,
                error: Some(e.to_string()),
            },
        })
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .and_then(|c| Ok(c))
        .map_err(|e| e.to_string());

    match result {
        Err(e) => ClashTestResult {
            success: false, latency_ms: None, error: Some(e)
        },
        Ok(client) => {
            match client.head("https://github.com").send().await {
                Ok(_) => ClashTestResult {
                    success: true,
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                    error: None,
                },
                Err(e) => ClashTestResult {
                    success: false,
                    latency_ms: None,
                    error: Some(e.to_string()),
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clash_test_result_serializable() {
        let r = ClashTestResult {
            success: true,
            latency_ms: Some(100),
            error: None,
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("latency_ms"));
    }

    #[test]
    fn test_save_and_load_subscription_url() {
        // 设置临时 HOME
        let tmp = std::env::temp_dir()
            .join(format!("oc_clash_test_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        #[cfg(unix)]
        std::env::set_var("HOME", &tmp);

        save_subscription_url("https://example.com/sub").unwrap();
        let loaded = load_subscription_url();
        assert_eq!(loaded.as_deref(), Some("https://example.com/sub"));

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_stop_is_idempotent() {
        // 没有进程时 stop 不应 panic
        let result = stop();
        assert!(result.is_ok());
    }
}
```

- [ ] **Step 2：运行测试**
```bash
cd src-tauri
cargo test clash_proxy 2>&1 | grep -E "test .* (ok|FAILED)"
# 期望：3 个 ok
cd ..
```

- [ ] **Step 3：在 main.rs 注册 commands**
```rust
mod clash_proxy;

#[tauri::command]
async fn clash_test(subscription_url: String) -> clash_proxy::ClashTestResult {
    // G2 fix: 先启动 Mihomo 获取代理地址，再测试代理连通性
    match clash_proxy::start(&subscription_url).await {
        Ok(proxy_addr) => clash_proxy::test_proxy(&proxy_addr).await,
        Err(e) => clash_proxy::ClashTestResult {
            success: false,
            latency_ms: None,
            error: Some(e.to_string()),
        },
    }
}

#[tauri::command]
async fn clash_start(subscription_url: String) -> Result<String, String> {
    clash_proxy::start(&subscription_url).await.map_err(|e| e.to_string())
}

#[tauri::command]
fn clash_stop() -> Result<(), String> {
    clash_proxy::stop().map_err(|e| e.to_string())
}
```

更新 `generate_handler!` 添加 `clash_test, clash_start, clash_stop`。

- [ ] **Step 4：cargo check**
```bash
cd src-tauri && cargo check 2>&1 | grep "^error" | head -5
cd ..
```

- [ ] **Step 5：Commit**
```bash
git add src-tauri/src/clash_proxy.rs src-tauri/src/main.rs
git commit -m "feat(backend): 实现 Clash/Mihomo 临时代理模块（启动/停止/测试）"
```

---

## Chunk 6：Skills 更新模块（skills_manager.rs）

### 文件

| 操作 | 路径 |
|---|---|
| 新建 | `src-tauri/src/skills_manager.rs` |
| 修改 | `src-tauri/src/main.rs` |

### Task 9：Skills 列表查询 + .tgz 原子更新

- [ ] **Step 1：写测试**

新建 `src-tauri/src/skills_manager.rs`：
```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Clone)]
pub struct SkillInfo {
    pub name: String,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
}

/// 从 node_modules 读取已安装的 @openclaw/* Skills
pub fn list_installed(install_path: &str) -> Vec<SkillInfo> {
    let modules = PathBuf::from(install_path)
        .join("openclaw_pkg")
        .join("node_modules");

    let scope_dir = modules.join("@openclaw");
    if !scope_dir.exists() { return vec![]; }

    std::fs::read_dir(&scope_dir)
        .ok()
        .map(|entries| {
            entries.filter_map(|e| {
                let entry = e.ok()?;
                let pkg_json = entry.path().join("package.json");
                let data = std::fs::read_to_string(&pkg_json).ok()?;
                let v: serde_json::Value = serde_json::from_str(&data).ok()?;
                let name = v["name"].as_str()?.to_string();
                let version = v["version"].as_str()?.to_string();
                Some(SkillInfo {
                    name,
                    current_version: version,
                    latest_version: None,
                    update_available: false,
                })
            })
            .collect()
        })
        .unwrap_or_default()
}

/// 查询 npmmirror 获取最新版本
pub async fn fetch_latest_version(
    skill_name: &str,
    proxy_url: Option<&str>
) -> Result<String> {
    let url = format!(
        "https://registry.npmmirror.com/{}/latest",
        skill_name
    );
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10));
    if let Some(proxy) = proxy_url {
        builder = builder.proxy(reqwest::Proxy::all(proxy)?);
    }
    let client = builder.build()?;
    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;
    resp["version"].as_str()
        .map(String::from)
        .ok_or_else(|| anyhow::anyhow!("无法解析版本"))
}

/// 下载 .tgz 并原子替换 node_modules 中的目录
pub async fn update_skill(
    install_path: &str,
    skill_name: &str,
    version: &str,
    proxy_url: Option<&str>,
) -> Result<()> {
    // 清理 @openclaw/ 前缀，得到 short_name
    let short_name = skill_name.trim_start_matches("@openclaw/");
    let url = format!(
        "https://registry.npmmirror.com/@openclaw/{0}/-/{0}-{1}.tgz",
        short_name, version
    );

    // 下载
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120));
    if let Some(proxy) = proxy_url {
        builder = builder.proxy(reqwest::Proxy::all(proxy)?);
    }
    let client = builder.build()?;
    let bytes = client.get(&url).send().await?.bytes().await?;

    // 解压到同文件系统的临时目录（确保 rename 原子性）
    let modules = PathBuf::from(install_path)
        .join("openclaw_pkg")
        .join("node_modules");
    let tmp_dir = PathBuf::from(install_path)
        .join(".tmp")
        .join("skills")
        .join(format!("{}-{}", short_name, version));
    std::fs::create_dir_all(&tmp_dir)?;

    // 解压 .tgz
    let gz = flate2::read::GzDecoder::new(std::io::Cursor::new(&bytes));
    let mut archive = tar::Archive::new(gz);
    archive.unpack(&tmp_dir)?;

    // npm tgz 解压后内容在 package/ 子目录
    let extracted = tmp_dir.join("package");
    let target = modules.join("@openclaw").join(short_name);
    let backup = modules.join("@openclaw")
        .join(format!("{}.bak", short_name));

    // 原子替换（同文件系统 rename）
    if target.exists() {
        std::fs::rename(&target, &backup)?;
    }
    std::fs::rename(&extracted, &target)?;
    if backup.exists() {
        std::fs::remove_dir_all(&backup)?;
    }

    // 清理临时目录
    std::fs::remove_dir_all(&tmp_dir.parent().unwrap().parent().unwrap().join("skills")).ok();
    Ok(())
}

/// Windows：通过 HTTP 管理接口热重载（回退到重启服务）
#[cfg(target_os = "windows")]
pub async fn reload_skills_windows(port: u16, admin_password: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    let result = client
        .post(format!("http://127.0.0.1:{}/admin/reload-skills", port))
        .header("Authorization", format!("Bearer {}", admin_password))
        .send().await;

    if result.is_err() {
        // HTTP 接口不可用，重启服务
        std::process::Command::new("schtasks")
            .args(["/End", "/TN", "OpenClaw Gateway"])
            .status()?;
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        std::process::Command::new("schtasks")
            .args(["/Run", "/TN", "OpenClaw Gateway"])
            .status()?;
    }
    Ok(())
}

/// Linux/macOS：发送 SIGHUP
#[cfg(unix)]
pub fn send_reload_signal(install_path: &str) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("systemctl")
            .args(["--user", "kill", "-s", "HUP", "openclaw.service"])
            .status()?;
    }
    #[cfg(target_os = "macos")]
    {
        // B3 fix: macOS 使用 launchctl kickstart -k 重载 launchd 服务
        let label = "com.openclaw.gateway";
        let result = std::process::Command::new("launchctl")
            .args(["kickstart", "-k", &format!("gui/{}/{}", unsafe { libc::getuid() }, label)])
            .status();
        if result.is_err() {
            // 回退：读取 PID 文件发送 SIGHUP
            let pid_path = PathBuf::from(install_path).join("openclaw.pid");
            if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
                if let Ok(pid) = pid_str.trim().parse::<i32>() {
                    unsafe { libc::kill(pid, libc::SIGHUP); }
                }
            }
        }
    }
    let _ = install_path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_installed_returns_empty_for_missing_dir() {
        let result = list_installed("/nonexistent/path");
        assert!(result.is_empty());
    }

    #[test]
    fn test_skill_info_serializable() {
        let info = SkillInfo {
            name: "@openclaw/feishu".into(),
            current_version: "1.0.0".into(),
            latest_version: Some("1.1.0".into()),
            update_available: true,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("update_available"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_list_installed_reads_package_json() {
        // 创建模拟的 node_modules 目录结构
        let tmp = std::env::temp_dir()
            .join(format!("oc_skills_test_{}", std::process::id()));
        let skill_dir = tmp
            .join("openclaw_pkg")
            .join("node_modules")
            .join("@openclaw")
            .join("feishu");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("package.json"),
            r#"{"name":"@openclaw/feishu","version":"1.2.3"}"#
        ).unwrap();

        let skills = list_installed(tmp.to_str().unwrap());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "@openclaw/feishu");
        assert_eq!(skills[0].current_version, "1.2.3");

        std::fs::remove_dir_all(&tmp).ok();
    }
}
```

- [ ] **Step 2：运行测试**
```bash
cd src-tauri
cargo test skills_manager 2>&1 | grep -E "test .* (ok|FAILED)"
# 期望：3 个 ok
cd ..
```

- [ ] **Step 3：注册 commands**

在 `main.rs` 追加：
```rust
mod skills_manager;

#[tauri::command]
fn list_skills(install_path: String) -> Vec<skills_manager::SkillInfo> {
    skills_manager::list_installed(&install_path)
}

#[tauri::command]
async fn update_skills(
    install_path: String,
    skill_names: Vec<String>,
    proxy_url: Option<String>,
) -> Result<(), String> {
    for skill in &skill_names {
        let version = skills_manager::fetch_latest_version(
            skill, proxy_url.as_deref()
        ).await.map_err(|e| e.to_string())?;
        skills_manager::update_skill(
            &install_path, skill, &version, proxy_url.as_deref()
        ).await.map_err(|e| e.to_string())?;
    }
    #[cfg(unix)]
    skills_manager::send_reload_signal(&install_path)
        .map_err(|e| e.to_string())?;
    Ok(())
}
```

更新 `generate_handler!` 添加 `list_skills, update_skills`。

- [ ] **Step 4：cargo check + commit**
```bash
cd src-tauri && cargo check 2>&1 | grep "^error" | head -5 && cd ..
git add src-tauri/src/skills_manager.rs src-tauri/src/main.rs
git commit -m "feat(backend): 实现 Skills 更新模块（.tgz 原子替换，无 npm 依赖）"
```

---

## Chunk 7：GitHub ZIP 更新模块（updater.rs）

### 文件

| 操作 | 路径 |
|---|---|
| 新建 | `src-tauri/src/updater.rs` |
| 修改 | `src-tauri/src/main.rs` |

### Task 10：版本检查 + ZIP 下载替换

- [ ] **Step 1：写测试**

新建 `src-tauri/src/updater.rs`：
```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
    pub release_notes: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    body: Option<String>,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

/// 比较版本号，返回 server > local
/// B6 fix: 使用 semver crate 进行语义版本比较，正确处理预发布标签等边缘情况
pub fn is_newer(server: &str, local: &str) -> bool {
    let strip = |s: &str| s.trim_start_matches('v');
    match (
        semver::Version::parse(strip(server)),
        semver::Version::parse(strip(local)),
    ) {
        (Ok(sv), Ok(lv)) => sv > lv,
        _ => false, // 解析失败时保守处理：不升级
    }
}

pub async fn check_update(proxy_url: Option<&str>) -> Result<Option<UpdateInfo>> {
    let current = env!("CARGO_PKG_VERSION");
    let url = "https://api.github.com/repos/openclaw/openclaw/releases/latest";

    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("openclaw-wizard");
    if let Some(proxy) = proxy_url {
        builder = builder.proxy(reqwest::Proxy::all(proxy)?);
    }
    let client = builder.build()?;
    let release: GithubRelease = client.get(url).send().await?.json().await?;

    if !is_newer(&release.tag_name, current) {
        return Ok(None);
    }

    // 查找 openclaw.tgz asset
    let asset = release.assets.iter()
        .find(|a| a.name == "openclaw.tgz")
        .ok_or_else(|| anyhow::anyhow!("Release 中未找到 openclaw.tgz"))?;

    Ok(Some(UpdateInfo {
        version: release.tag_name,
        download_url: asset.browser_download_url.clone(),
        release_notes: release.body.unwrap_or_default(),
        sha256: None,
    }))
}

pub async fn apply_update(
    install_path: &str,
    download_url: &str,
    sha256: Option<&str>,
    proxy_url: Option<&str>,
    window: &tauri::Window,
) -> Result<()> {
    // 1. 停止服务
    let _ = window.emit("update:progress", "正在停止服务…");
    stop_service()?;

    // 2. 下载 ZIP
    let _ = window.emit("update:progress", "正在下载新版本…");
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300));
    if let Some(proxy) = proxy_url {
        builder = builder.proxy(reqwest::Proxy::all(proxy)?);
    }
    let client = builder.build()?;
    let bytes = client.get(download_url).send().await?.bytes().await?;

    // 3. SHA256 校验
    if let Some(expected) = sha256 {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let actual = hex::encode(hasher.finalize());
        anyhow::ensure!(actual == expected,
            "SHA256 校验失败：期望 {}, 实际 {}", expected, actual);
    }

    // 4. 备份旧版本
    let _ = window.emit("update:progress", "正在备份旧版本…");
    let pkg_dir = PathBuf::from(install_path).join("openclaw_pkg");
    let backup_dir = PathBuf::from(install_path).join("openclaw_pkg.bak");
    if pkg_dir.exists() {
        if backup_dir.exists() {
            std::fs::remove_dir_all(&backup_dir)?;
        }
        std::fs::rename(&pkg_dir, &backup_dir)?;
    }

    // 5. 解压新版本
    let _ = window.emit("update:progress", "正在安装新版本…");
    let tmp_dir = PathBuf::from(install_path).join(".tmp").join("update");
    std::fs::create_dir_all(&tmp_dir)?;

    let gz = flate2::read::GzDecoder::new(std::io::Cursor::new(&bytes));
    let mut archive = tar::Archive::new(gz);
    archive.unpack(&tmp_dir)?;

    std::fs::rename(&tmp_dir, &pkg_dir)?;

    // 6. 重启服务
    let _ = window.emit("update:progress", "正在重启服务…");
    match start_service() {
        Ok(_) => {
            // 7. 健康检查通过后删除备份
            // S6 fix: 从 deploy_meta.json 读取端口，而非硬编码
            let port: u16 = {
                let meta_path = dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".openclaw/deploy_meta.json");
                let meta: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&meta_path)?)?;
                meta["service_port"].as_u64().unwrap_or(18789) as u16
            };
            if crate::deploy::health_check(port).await.is_ok() {
                std::fs::remove_dir_all(&backup_dir).ok();
            }
        }
        Err(e) => {
            // 回滚
            let _ = window.emit("update:progress", "启动失败，正在回滚…");
            std::fs::remove_dir_all(&pkg_dir).ok();
            if backup_dir.exists() {
                std::fs::rename(&backup_dir, &pkg_dir)?;
            }
            // S7 fix: 回滚启动失败不应 propagate（避免掩盖原始错误）
            let _ = start_service();
            anyhow::bail!("更新失败，已回滚到旧版本: {}", e);
        }
    }

    let _ = window.emit("update:done", ());
    Ok(())
}

fn stop_service() -> Result<()> {
    #[cfg(target_os = "linux")]
    std::process::Command::new("systemctl")
        .args(["--user", "stop", "openclaw.service"])
        .status()?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("schtasks")
        .args(["/End", "/TN", "OpenClaw Gateway"])
        .status()?;
    Ok(())
}

fn start_service() -> Result<()> {
    #[cfg(target_os = "linux")]
    std::process::Command::new("systemctl")
        .args(["--user", "start", "openclaw.service"])
        .status()?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("schtasks")
        .args(["/Run", "/TN", "OpenClaw Gateway"])
        .status()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_basic() {
        assert!(is_newer("1.1.0", "1.0.0"));
        assert!(is_newer("v1.1.0", "1.0.0"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("0.9.9", "1.0.0"));
    }

    #[test]
    fn test_is_newer_patch() {
        assert!(is_newer("1.0.1", "1.0.0"));
        assert!(!is_newer("1.0.0", "1.0.1"));
    }

    #[test]
    fn test_update_info_serializable() {
        let info = UpdateInfo {
            version: "1.1.0".into(),
            download_url: "https://example.com/openclaw.tgz".into(),
            release_notes: "Bug fixes".into(),
            sha256: Some("abc123".into()),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("1.1.0"));
    }
}
```

- [ ] **Step 2：运行测试**
```bash
cd src-tauri
cargo test updater 2>&1 | grep -E "test .* (ok|FAILED)"
# 期望：3 个 ok
cd ..
```

- [ ] **Step 3：注册 commands 并最终 cargo check**
```rust
// main.rs 追加：
mod updater;

#[tauri::command]
async fn check_openclaw_update(proxy_url: Option<String>)
    -> Result<Option<updater::UpdateInfo>, String>
{
    updater::check_update(proxy_url.as_deref()).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn apply_openclaw_update(
    install_path: String,
    download_url: String,
    sha256: Option<String>,
    proxy_url: Option<String>,
    window: tauri::Window,
) -> Result<(), String> {
    updater::apply_update(
        &install_path, &download_url,
        sha256.as_deref(), proxy_url.as_deref(), &window
    ).await.map_err(|e| e.to_string())
}
```

更新 `generate_handler!` 添加 `check_openclaw_update, apply_openclaw_update`。

- [ ] **Step 4：全量 cargo test**
```bash
cd src-tauri
cargo test 2>&1 | tail -10
# 期望：全部 ok，0 failed
cd ..
```

- [ ] **Step 5：Commit**
```bash
git add src-tauri/src/updater.rs src-tauri/src/main.rs
git commit -m "feat(backend): 实现 GitHub ZIP 更新模块（版本检查/下载/回滚）"
```

---

## 最终验证

- [ ] **全量 cargo test**
```bash
cd src-tauri
cargo test -- --nocapture 2>&1 | tail -20
# 期望：所有测试通过，0 failed
cd ..
```

- [ ] **cargo clippy（代码质量检查）**
```bash
cd src-tauri
cargo clippy -- -D warnings 2>&1 | grep "^error" | head -10
# 期望：无 error
cd ..
```

- [ ] **最终 commit**
```bash
git add -A
git commit -m "feat(backend): Plan A 完成 - Rust 后端全部模块实现"
```

---

## Tauri Command 完整清单（供 Plan B Vue 前端参考）

| Command | 参数 | 返回值 | 用途 |
|---|---|---|---|
| `run_system_check` | 无 | `Vec<CheckItem>` | SystemCheckPage |
| `start_deploy` | `DeployConfigDto`, window | `void` | DeploymentPage |
| `health_check` | `port: u16` | `void/err` | DeploymentPage |
| `load_session` | 无 | `Option<SessionState>` | WelcomePage |
| `clear_session` | `install_path?` | `void` | SourcePage |
| `get_platform_doc_url` | `platform: String` | `String` | PlatformIntegrationPage |
| `clash_test` | `subscription_url` | `ClashTestResult` | ClashConfigPage |
| `clash_start` | `subscription_url` | `String`（代理地址）| ClashConfigPage |
| `clash_stop` | 无 | `void` | DeploymentPage（部署后清理）|
| `list_skills` | `install_path` | `Vec<SkillInfo>` | FinishPage |
| `update_skills` | `install_path`, `skill_names`, `proxy_url?` | `void` | FinishPage |
| `check_openclaw_update` | `proxy_url?` | `Option<UpdateInfo>` | FinishPage/WelcomePage |
| `apply_openclaw_update` | `install_path`, `download_url`, `sha256?`, `proxy_url?`, window | `void` | FinishPage |

**事件（Rust → 前端 emit）：**

| 事件名 | Payload | 监听页面 |
|---|---|---|
| `deploy:progress` | `DeployProgress` | DeploymentPage |
| `deploy:done` | 无 | DeploymentPage |
| `update:progress` | `String`（消息）| FinishPage |
| `update:done` | 无 | FinishPage |
