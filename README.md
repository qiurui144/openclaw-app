# OpenClaw 一键部署向导

基于 Qt6 构建的跨平台部署工具，支持 Linux AppImage（Ubuntu 20.04+）和 Windows EXE（Win10/11）。

## 功能特点

- **全量内置**：所有服务二进制预置在安装包内，国内网络无需翻墙
- **平台集成**：一键配置企业微信、QQ Work、钉钉、飞书 Webhook
- **系统服务**：支持注册为 systemd 服务（Linux）或 Windows 服务
- **自动跳转**：关键配置步骤自动打开官方文档页面

## 快速开始

### Linux

```bash
chmod +x OpenClaw-*.AppImage
./OpenClaw-*.AppImage
```

### Windows

双击 `openclaw-setup.exe`，右键以管理员身份运行。

## 构建

详见 [DEVELOP.md](DEVELOP.md)。
