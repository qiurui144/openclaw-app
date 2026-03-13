# OpenClaw 部署向导重设计规格文档

**日期**：2026-03-13
**状态**：修订版 v2（已修复严重问题）
**范围**：完整重写现有 Qt 向导，迁移至 Tauri + Vue 3，新增离线/在线/ZIP 三模式、Clash 临时代理、QR 码平台集成、GitHub ZIP 更新、Skills 网络更新

---

## 1. 背景与目标

### 1.1 现状问题

当前实现基于 Qt6/C++，存在以下局限：

- 显示二维码、内嵌平台授权页需要引入 Qt WebEngine（额外 ~100MB）
- QSS 样式能力有限，难以实现现代化 UI 动效
- macOS 支持需要额外适配，签名/公证流程复杂
- 平台集成页仅支持 Webhook URL 输入，无引导流程

### 1.2 目标

1. **UI 现代化**：Material Design 风格，中文友好，小白一键完成
2. **三模式安装**：离线全包 / 在线下载 / 本地 ZIP 导入，三端统一
3. **平台集成增强**：QR 码扫码 + 网页跳转引导用户获取 Webhook
4. **Clash 临时代理**：用户提供订阅 URL，仅下载期间启用，完成后清除
5. **GitHub ZIP 更新**：openclaw 服务通过 GitHub Release ZIP 升级
6. **Skills 网络更新**：插件/Skills 独立更新，支持淘宝镜像源
7. **免责声明**：Clash 使用前强制展示并要求用户确认

---

## 2. 技术选型

| 层 | 选型 | 理由 |
|---|---|---|
| 前端框架 | Vue 3 + Vite | 轻量、组件化、QR 码/动效生态完善 |
| 后端框架 | Rust（Tauri 2.x） | 系统调用、进程管理、跨平台打包 |
| 状态管理 | Pinia | 跨向导页面共享配置 |
| QR 码生成 | qrcode.js（前端） | 纯离线生成，无需网络 |
| 打包工具 | Tauri Bundler（Linux/Win）+ bash 脚本（macOS） | 见下方说明 |

### 2.1 跨平台分发形式

| 平台 | Full Bundle（离线） | Lite（在线） | 签名 | 实现方式 |
|---|---|---|---|---|
| Linux | AppImage（~265MB） | AppImage（~20MB） | 无需 | Tauri |
| Windows | NSIS EXE（~265MB） | NSIS EXE（~20MB） | 可选 EV 证书 | Tauri |
| macOS | ZIP 含 .command + resources/（~265MB） | ZIP 含 .command（~1MB） | 无需 | 纯 bash 脚本 |

**macOS 技术说明**：macOS 完全不使用 Tauri，使用独立的 bash 脚本方案（`install.command`）。原因：规避 Apple 开发者账号（$99/年）的签名/公证要求，`.command` 文件双击即可在 Terminal 运行，无需任何签名。Tauri Bundler 仅用于 Linux 和 Windows 的 GUI 向导打包。macOS 脚本与 Tauri 应用无任何代码共享，单独维护（见第 7 节）。

---

## 3. 向导页面设计

### 3.1 页面流程

```
WelcomePage (0)
  └─→ SystemCheckPage (1)
        └─→ SourcePage (2)  ★新增
              ├─ [离线包] → InstallConfigPage (3)
              ├─ [在线下载] → ClashDisclaimerPage (3a) → ClashConfigPage (3b) → InstallConfigPage (3)
              └─ [本地ZIP] → InstallConfigPage (3)
                              └─→ ServiceConfigPage (4)
                                    └─→ PlatformIntegrationPage (5)  ★增强
                                          └─→ DeploymentPage (6)
                                                └─→ FinishPage (7)
```

**页面跳转规则**：
- Full Bundle 启动时检测到内置资源 → SourcePage 自动选中"离线包"并锁定，跳过 Clash 相关页
- Lite 启动时 → SourcePage 默认"在线下载"，显示 Clash 配置入口
- 用户选择"本地 ZIP" → 文件选择器 / 拖拽，校验后直接进入 InstallConfigPage
- ClashDisclaimerPage 和 ClashConfigPage 仅在用户主动选择"配置代理"时出现

### 3.2 各页面详细设计

#### WelcomePage
- 应用 Logo + 名称 + 版本号
- 功能亮点卡片（4个）：全量内置 / 平台集成 / 开机自启 / 网页控制台
- 检查是否有新版向导可用（异步，不阻塞）
- **重复安装检测**：启动时读取 `~/.openclaw/deploy_meta.json`（或 Windows/macOS 对应路径）：
  - 文件存在 → 在 WelcomePage 底部显示黄色提示条："检测到已安装版本 X.Y.Z（安装于 /path/to），继续将进行覆盖升级"，提供两个选项：**"升级安装"**（保留配置，仅替换程序）和 **"全新安装"**（清除已有数据）
  - 文件不存在 → 正常全新安装流程
  - 检测到旧版 openclaw 服务正在运行且占用目标端口 → 在 SystemCheckPage 端口检查项给出专项提示："openclaw 服务已在运行，将在部署步骤中自动停止后替换"

#### SystemCheckPage
检查项：

| 项目 | 必需 | 标准 |
|---|---|---|
| 操作系统版本 | ✓ | Linux kernel ≥ 4.x / Win 10+ / macOS 11+ |
| 磁盘空间 | ✓ | ≥ 512MB 可用 |
| 端口 18789 | ✗ | 未被占用（仅警告） |
| 网络连通性 | ✗ | 仅在线模式检查（仅警告） |

所有必需项通过才启用"下一步"。

#### SourcePage（新增）

```
● 📦 使用内置离线包（推荐）
  无需网络，直接安装

○ 🌐 从网络下载最新版本
  需要访问 GitHub Release

○ 📁 导入本地安装包
  [ 拖拽 ZIP 到此处，或点击选择 ]
```

- Full Bundle 时第一项自动选中且禁止切换
- 选择"本地 ZIP"后显示文件拖拽区，校验 ZIP 内容结构（必须包含 node + openclaw.tgz）

#### ClashDisclaimerPage（新增，仅在线模式）

强制展示免责声明（见第 6 节），用户必须勾选复选框才能继续，提供"跳过代理直连"按钮。

#### ClashConfigPage（新增，仅在线模式且同意声明后）

```
订阅链接：[ https://your-clash-sub-url... ] [测试连接]
状态：✓ 连接成功，延迟 123ms

ℹ️ 代理仅在资源下载期间临时启用，完成后自动停止并删除相关文件。
```

- 测试连接：启动 Clash → 尝试访问 https://github.com → 显示延迟/失败
- 订阅 URL 本地存储（用于后续 Skills 更新）

#### InstallConfigPage

- 安装路径选择（平台默认：Linux `/opt/openclaw`，Windows `%LOCALAPPDATA%\openclaw`，macOS `~/openclaw`）
  - Windows 使用 `%LOCALAPPDATA%` 而非 `%APPDATA%`：前者不受漫游配置影响，企业组策略限制较少，且用户更容易定位
- 注册系统服务（默认勾选）
- 开机自启（依赖上一项）
- 路径不存在时询问是否创建

#### ServiceConfigPage

- 监听端口（默认 18789，范围 1024-65535）
- 绑定域名（可选）
- 管理员密码（≥8位，字母+数字，带强度可视化色条）
- 确认密码

#### PlatformIntegrationPage（增强）

每个平台结构：

```
[启用复选框] 平台名称

第一步：创建机器人，获取 Webhook 地址
  [📱 手机扫码]（显示 QR 码弹窗）  [💻 网页打开]

第二步：将 Webhook 地址粘贴到此处
  [ https://... ] （实时校验，✓绿/⚠橙）
```

支持平台：企业微信 / QQ Work / 钉钉 / 飞书

QR 码内容：各平台机器人配置官方文档固定 URL（纯前端生成，无需联网）：

| 平台 | QR 码目标 URL | 备注 |
|---|---|---|
| 企业微信 | `https://work.weixin.qq.com/api/doc/90000/90136/91770` | 群机器人配置文档 |
| QQ Work | `https://work.qq.com/` | 首页（文档入口易变，定期校验） |
| 钉钉 | `https://open.dingtalk.com/document/robots/custom-robot-access` | 自定义机器人文档 |
| 飞书 | `https://open.feishu.cn/document/client-docs/bot-v3/add-custom-bot` | 自定义机器人文档 |

> ⚠️ QQ Work 文档 URL 历史上多次变更，建议在发布 CI 中加一步 curl 检测，URL 失效时报警而非静默失败。

页面为可选（isComplete 始终 true）。

#### DeploymentPage

- 状态标签（实时）
- 进度条 0-100%
- 日志区默认折叠，展开显示时间戳 + 详细消息（等宽字体）
- setCommitPage(true)，部署中不可返回
- 失败时：进度条变红，显示人性化错误描述 + 建议操作

部署步骤（11步），各步骤因 SourceMode 不同行为如下：

| 步骤 | Bundled（离线） | Online（在线） | LocalZip（本地） |
|---|---|---|---|
| 1. 创建安装目录 | mkdirp | mkdirp | mkdirp |
| 2. 获取 Node.js | 从内置 QRC 资源提取 | 下载（走 Clash 代理，若配置） | 从用户 ZIP 提取 |
| 3. 获取 openclaw 包 | 从内置 QRC 资源提取 openclaw.tgz | 下载 GitHub Release ZIP | 从用户 ZIP 提取 openclaw.tgz |
| 4. 解包到 installPath/ | tar xzf | 解压 ZIP | tar xzf |
| 5-11. | 三种模式完全相同 | | |

后续步骤（模式无关）：
5. 写入 `~/.openclaw/openclaw.json` 主配置
6. 写入平台集成配置（channels 节）
7. 注册系统服务（systemd / schtasks；macOS 不适用，由脚本处理）
8. 启动服务
9. HTTP 健康检查（轮询 `http://127.0.0.1:{port}/health`，最多 30s，间隔 2s）
   - 通过 → 继续
   - **每轮**轮询失败后立即检查端口占用（而非等 30s 后统一检查）：
     - 端口被**非 openclaw 进程**占用 → 立即终止轮询，错误消息标注"端口 {port} 已被进程 {pid}/{name} 占用，请更换端口后重试"（避免无效等待 30s）
     - 端口未被占用但连接被拒（服务启动中）→ 继续轮询直至超时
   - 30s 超时（服务崩溃/启动过慢）→ 展示"服务未能正常启动"，给出两个选项："查看日志"/ "忽略并继续"
10. 生成卸载脚本（`installPath/uninstall.sh` 或 `uninstall.bat`）
11. 写入安装记录（`~/.openclaw/deploy_meta.json`，含版本、安装路径、时间戳）

#### FinishPage

- 大号成功图标 + 完成提示
- "🌐 打开管理控制台"（1秒后自动打开浏览器）
- **安装信息摘要**（一键复制）：服务地址、配置文件路径（`~/.openclaw/openclaw.json`）
  - openclaw.json 中密码字段存储为 **bcrypt 哈希**，无法反推原文；FinishPage 提示文案为"忘记密码请通过管理控制台重置"，不引导用户查看配置文件
- "下一步做什么"引导卡片：创建第一个机器人 / 查看文档 / 邀请团队
- GitHub 问题反馈链接
- 如健康检查未通过（忽略并继续的场景）：顶部显示橙色警告横幅"服务尚未运行，请查看日志或手动启动"

---

## 4. 核心 Rust 模块设计

### 4.1 模块划分

```
src-tauri/src/
├── main.rs                  # Tauri app 入口，注册所有 command
├── deploy.rs                # 部署引擎（复用现有逻辑，Rust 重写）
├── system_check.rs          # 系统检查（OS/磁盘/端口/网络）
├── updater.rs               # GitHub ZIP 更新（openclaw 服务 + 向导自身）
├── clash_proxy.rs           # Clash（Mihomo）临时代理管理
├── skills_manager.rs        # Skills/插件网络更新（直接下载 .tgz，不依赖 npm）
├── platform_config.rs       # 平台配置读写
└── session_state.rs         # 下载断点续传状态持久化
```

**session_state.rs 职责**：
- 持久化文件：`~/.openclaw/install_session.json`（安装成功后删除）
- 记录：SourceMode 选择、已下载文件路径及 SHA256、当前完成步骤编号
- **严格排除 admin_password**：session JSON 中不得出现任何密码字段
- 向导启动时自动检测残留 session → 提示"上次安装未完成，是否继续？"
- 用户选择"全新安装"或切换 SourceMode 时：删除 session 文件 + 清理 `installPath/.tmp/` 下的残留临时文件
- 向导被强制关闭（Ctrl+C / 窗口关闭）时：session 文件保留，下次启动时提示续传

### 4.2 Tauri Command 接口（前后端边界）

```rust
// system_check.rs
#[tauri::command] async fn run_system_check() -> Vec<CheckItem>

// deploy.rs
#[tauri::command] async fn start_deploy(config: DeployConfig, window: Window)
  // 进度通过 window.emit("deploy:progress", payload) 推送

// clash_proxy.rs
#[tauri::command] async fn clash_test(subscription_url: String) -> ClashTestResult
#[tauri::command] async fn clash_start(subscription_url: String) -> Result<(), String>
#[tauri::command] async fn clash_stop() -> Result<(), String>

// updater.rs
#[tauri::command] async fn check_openclaw_update() -> Option<UpdateInfo>
#[tauri::command] async fn apply_openclaw_update(download_url: String, sha256: String, window: Window)

// skills_manager.rs
#[tauri::command] async fn list_installed_skills() -> Vec<SkillInfo>
#[tauri::command] async fn update_skills(skill_ids: Vec<String>, window: Window)
```

### 4.3 DeployConfig 数据结构

```rust
// 注意：DeployConfig 通过 Tauri IPC 从前端传入，不得序列化到磁盘。
// session_state.rs 持久化时必须排除 admin_password 字段。
pub struct DeployConfig {
    pub install_path: String,
    pub service_port: u16,                    // 默认 18789
    pub admin_password: SecretString,         // 使用 secrecy crate 防止明文落盘和日志泄漏
    pub domain_name: Option<String>,
    pub install_service: bool,
    pub start_on_boot: bool,
    pub source_mode: SourceMode,

    // 平台集成
    pub platforms: Vec<PlatformConfig>,
}
// SecretString = secrecy::Secret<String>，析构时自动归零内存，
// 且 #[derive(Debug)] 输出为 "Secret([REDACTED])"，不会写入日志。

pub enum SourceMode {
    Bundled,
    // proxy_url 存储实际代理地址（如 "http://127.0.0.1:7890"），
    // 由 clash_start() 成功后填入，而非订阅 URL。
    // 订阅 URL 仅由 clash_proxy.rs 持有，持久化到 ~/.openclaw/proxy.json。
    Online { proxy_url: Option<String> },
    LocalZip(PathBuf),
}

pub struct PlatformConfig {
    pub platform: Platform,         // WeWork | QQWork | DingTalk | Feishu
    pub webhook_url: String,
}
```

---

## 5. 更新系统设计

### 5.1 openclaw 服务更新（GitHub ZIP）

```
触发方式：FinishPage "检查更新" 按钮 / 设置页手动触发

流程：
1. GET https://api.github.com/repos/openclaw/openclaw/releases/latest
   （通过 Clash 代理，若已配置）
2. 解析最新版本号，与 deploy_meta.json 中记录的版本对比
3. 若有更新：显示版本说明，用户确认
4. 下载 Release ZIP（显示进度）→ SHA256 校验
5. 解压到临时目录
6. systemctl --user stop openclaw（或 launchd / schtasks 等效命令）
7. 备份旧 openclaw_pkg/ → openclaw_pkg.bak/
8. 替换 openclaw_pkg/
9. 重启服务
10. 健康检查通过 → 删除备份
    健康检查失败 → 回滚备份 → 提示用户
```

### 5.2 Skills 更新

**不依赖 npm CLI**：分发包内仅含 `node` 单文件可执行，不含 `npm`，离线/受限网络环境下无法执行 `npm install`。Skills 更新改为直接下载预打包 `.tgz` 文件后手动解压，完全不调用 npm。

```
触发方式：FinishPage "更新 Skills" 按钮 / 设置页手动触发

流程：
1. 读取 installPath/openclaw_pkg/node_modules/ 中 @openclaw/* 包的 package.json，获取当前版本
2. 批量查询淘宝镜像元数据 API：
   GET https://registry.npmmirror.com/@openclaw/{skill-name}/latest
   （通过 HTTP_PROXY=http://127.0.0.1:7890，若 Clash 已配置）
3. 对比版本，列出可更新项，用户勾选确认
4. 下载 .tgz：
   GET https://registry.npmmirror.com/@openclaw/{skill-name}/-/{skill-name}-{version}.tgz
5. 解压 .tgz 到 **与 installPath 同文件系统的临时目录**
   `installPath/.tmp/skills/{skill-name}-{version}/`（同挂载点确保 rename 原子性）
6. 原子替换：`rename(tmp_dir, node_modules/@openclaw/{skill-name})`
   - rename 是单个 syscall，openclaw 进程看到的始终是完整目录
   - 若旧目录存在，先 rename 旧目录为 `.bak` 备份，新目录 rename 到位后删除 `.bak`
7. 热重载通知（平台差异）：
   - Linux / macOS：发送 `SIGHUP` 到 openclaw 进程
   - Windows：**不支持 SIGHUP**，改为通过 HTTP 调用 openclaw 管理接口 `POST /admin/reload-skills`（需管理员密码鉴权）；若接口不可用则重启服务（`schtasks /End` + `/Run`）
8. 清理 `.tmp/skills/` 目录
```

### 5.3 向导自身更新

- Linux/Windows：Tauri 内置 updater，检查 GitHub Release 中的 `latest.json`
- macOS：重新 curl 拉取最新 `install.command` 并提示用户重新运行

---

## 6. Clash 免责声明全文

> **关于内置代理工具的使用声明**
>
> OpenClaw 安装向导可选集成 Clash 代理工具，以辅助在网络受限环境下完成资源下载。
>
> 使用前请知悉：
>
> 1. 本向导内置的代理内核为 **Mihomo**（原 Clash Meta，GPL-3.0 许可证），版本 {MIHOMO_VERSION}，为第三方开源软件，与 OpenClaw 项目无隶属关系，其许可证及版权归原作者所有。
> 2. 代理订阅链接由用户自行提供，本工具不提供、不推荐任何代理服务。
> 3. 代理功能仅在资源下载期间临时启用，完成后自动停止进程并删除相关二进制文件。
> 4. 用户须自行确保代理工具及订阅服务的使用符合所在地区的法律法规；由此产生的任何法律责任由用户自行承担，与 OpenClaw 项目无关。
>
> [ □ 我已阅读并同意上述声明 ]

---

## 7. macOS 安装脚本设计

### 7.1 分发包结构

```
openclaw-macos-full.zip          # 离线版 (~265MB)
├── install.command
└── resources/
    ├── node-macos-arm64
    ├── node-macos-x64
    ├── openclaw.tgz
    └── clash-macos

openclaw-macos-lite.zip          # 在线版 (~1MB)
└── install.command
```

### 7.2 脚本执行流程

```bash
install.command 启动后：

0. [可跳过] 检查新版脚本（超时 5s 自动跳过，不阻塞主流程）：
   curl --max-time 5 -fsSL https://github.com/openclaw/openclaw/releases/latest/download/version.txt
   若检测到新版本，提示"有新版本可用，建议重新下载后安装"，用户可选择继续或退出。

1. 检测 macOS 版本（≥ 11 Big Sur）和 CPU 架构（arm64 / x86_64）
2. 检测已有安装（~/.openclaw/deploy_meta.json）→ 提示升级或全新安装
3. 检测 resources/ 目录 → 决定 [离线] 或 [在线] 模式
4. 磁盘空间检查（≥ 512MB）
5. 交互式确认安装路径（默认 ~/openclaw）
6. [在线模式] 询问是否配置代理（输入订阅 URL），展示免责声明文本，要求输入 "yes" 确认
7. 获取 Node.js（离线：从 resources/ 复制；在线：curl 下载，走代理，带 SHA256 校验）
8. 解包 openclaw.tgz
9. [可选] 交互式平台集成配置（输入 Webhook URL）—— 在浏览器打开前完成
   - macOS 脚本**不支持 QR 码**（Terminal 无法渲染图片），改为在提示中 echo 各平台文档 URL 并用 `open` 命令打开浏览器：
     `echo "请访问：https://open.feishu.cn/..." && open "https://open.feishu.cn/..."`
   - 此为与 Linux/Windows GUI 向导的已知功能差异，文档明确记录
10. 写入 ~/.openclaw/openclaw.json（含平台配置）
11. 生成卸载脚本（~/openclaw/uninstall.sh）
12. 注册 launchd 服务（~/Library/LaunchAgents/io.openclaw.gateway.plist）
13. launchctl load + start
14. 等待健康检查（HTTP 轮询 http://127.0.0.1:18789/health，最多 30s，间隔 2s）
15. 打开浏览器 http://127.0.0.1:18789
16. 完成提示：显示服务状态 + 控制台地址 + 管理员密码提示 + 常用命令速查
```

---

## 8. 小白 UX 设计原则

1. **智能默认值**：所有配置项有合理默认，小白可一路点"下一步"完成安装
2. **错误人性化翻译**：不显示原始错误码，翻译为具体描述（"磁盘空间不足，请清理后重试"）
3. **日志默认折叠**：部署页仅显示进度条 + 状态文字，日志收起（技术用户可展开）
4. **密码强度可视化**：实时颜色条（红/橙/绿）+ 提示文字
5. **完成后引导**：FinishPage 显示"下一步"卡片（创建机器人 / 查看文档 / 邀请团队）
6. **断点续传**：下载中断后重启向导可继续，不从头开始（记录下载进度到临时文件）
7. **一键复制**：服务地址、密码、Webhook 配置等信息旁均有复制按钮

---

## 9. 构建产物与发布流程

### 9.1 资源准备脚本（fetch_resources.sh 升级）

复用现有脚本逻辑，新增：
- macOS arm64 / x86_64 Node.js 二进制下载
- macOS Clash（Mihomo）二进制打包
- 生成 `resources/manifest.json`（记录版本信息，供安装脚本校验）

**Clash（Mihomo）二进制来源规范**：

原 Clash 项目（clash-core）已于 2023 年删库。本项目使用其维护 fork **Mihomo**（原 clash-meta）：

| 字段 | 值 |
|---|---|
| 项目 | Mihomo（MetaCubX 维护） |
| 仓库 | `https://github.com/MetaCubX/mihomo` |
| 许可证 | GPL-3.0 |
| 固定版本 | 构建时由 `MIHOMO_VERSION` 环境变量指定（如 `v1.18.x`），不使用 latest |
| 下载源 | GitHub Release Assets（`mihomo-linux-amd64-{version}.gz` 等） |
| 完整性校验 | 每个平台二进制在 `resources/manifest.json` 中记录 SHA256，安装时校验 |
| 杀毒误报 | Windows 平台 Mihomo 二进制可能被部分杀毒软件误报；免责声明第 1 条中补充说明"所用代理内核为 Mihomo {version}" |

fetch_resources.sh 需下载的 Clash/Mihomo 二进制：

```
resources/clash/
├── mihomo-linux-amd64        # Linux x64
├── mihomo-windows-amd64.exe  # Windows x64
└── mihomo-darwin-arm64       # macOS Apple Silicon
└── mihomo-darwin-amd64       # macOS Intel
```

### 9.2 CI/CD 矩阵

| 构建目标 | Runner | 产物 |
|---|---|---|
| Linux Full/Lite | ubuntu-20.04 | AppImage |
| Windows Full/Lite | windows-2022 | NSIS EXE |
| macOS Full/Lite | macos-13 | ZIP + .command |

---

## 10. 不在本次范围内

- OpenClaw 服务端本身的功能开发
- 向导的单元测试（留待后续迭代）
- Windows EV 代码签名配置（可选，后续补充）
- Skills 商店/市场 UI（后续功能）
- **Skills 版本降级（downgrade）**：本期仅支持升级到最新版，回滚路径不在范围内
- macOS 向导静默自动更新：macOS 仅提示"有新版本"，用户需手动下载新包重新运行，当前已安装的 openclaw 服务不受影响（新脚本启动时通过 deploy_meta.json 检测到已安装，自动进入升级流程而非重新安装）
- Windows Skills 热重载通过 HTTP 管理接口实现，SIGHUP 不适用于 Windows

## 11. Rust 结构体 Derive 约束

`DeployConfig` 含 `SecretString`，**不能**直接实现 `Serialize`。需拆分为 DTO + 内部类型：

```rust
// ── 前端 → Rust IPC 传输用 DTO（可 Serialize/Deserialize）──────
#[derive(Debug, Clone, serde::Deserialize)]  // 只需 Deserialize（前端发来）
pub struct DeployConfigDto {
    pub install_path: String,
    pub service_port: u16,
    pub admin_password: String,   // 仅在 IPC boundary 用 plain String
    pub domain_name: Option<String>,
    pub install_service: bool,
    pub start_on_boot: bool,
    pub source_mode: SourceMode,
    pub platforms: Vec<PlatformConfig>,
}

// ── Rust 内部使用的安全类型（不可 Serialize，不落盘）────────────
pub struct DeployConfig {
    // ... 同字段，但 admin_password 换成 SecretString
    pub admin_password: secrecy::Secret<String>,
    // ... 其余字段相同
}

impl From<DeployConfigDto> for DeployConfig { ... }

// ── 其他类型（无敏感字段，可直接 Serialize/Deserialize）─────────
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SourceMode { ... }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlatformConfig { ... }

// CheckItem 仅从 Rust → 前端，只需 Serialize
#[derive(Debug, serde::Serialize)]
pub struct CheckItem { ... }
```

`main.rs` 中 command handler 接收 `DeployConfigDto`，立即转换为 `DeployConfig`，之后 `DeployConfigDto` 离开作用域即被销毁。
