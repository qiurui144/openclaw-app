# OpenClaw 一键部署向导

基于 Tauri 2.x（Rust + Vue 3）构建的跨平台部署工具，支持 Linux AppImage、Windows NSIS 安装包和 macOS 一键脚本。

## 功能特点

- **双版本发布**：Lite（在线下载 ~76MB）+ Full Bundle（离线安装 ~169MB）
- **三种安装模式**：内置资源 / 在线下载 / 本地 ZIP
- **平台集成**：一键配置企业微信、QQ Work、钉钉、飞书 Webhook
- **系统服务**：支持注册为 systemd（Linux）/ launchd（macOS）/ Windows 服务
- **代理支持**：内置 Mihomo 临时代理，解决国内网络问题
- **断点续传**：安装中断后可从上次进度继续

## 快速开始

### Linux

```bash
chmod +x openclaw-wizard-*.AppImage
./openclaw-wizard-*.AppImage
```

### Windows

双击 `openclaw-wizard-*-setup.exe`，安装向导自动引导。

### macOS

```bash
# 解压后双击 install.command，或在终端运行：
bash install.command
```

## 构建

详见 [DEVELOP.md](DEVELOP.md)。

## 最低系统要求

| 平台 | 要求 |
|------|------|
| Linux | Ubuntu 20.04+（glibc ≥ 2.31）|
| Windows | Windows 10 1803+（WebView2 自动安装）|
| macOS | macOS 11 Big Sur+ |
