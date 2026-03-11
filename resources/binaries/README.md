# 内置二进制文件目录

将要捆绑的服务程序二进制文件放置到此目录，并在 `resources.qrc` 中取消对应注释。

## 目录结构

```
binaries/
├── linux/
│   ├── openclaw-server      # Linux x86_64 主服务程序（ELF, 静态链接推荐）
│   └── openclaw-config      # Linux 配置工具
└── windows/
    ├── openclaw-server.exe  # Windows x64 主服务程序
    └── openclaw-config.exe  # Windows 配置工具
```

## 构建注意事项

- **Linux 二进制**：建议使用 musl/静态链接，或在 Ubuntu 20.04 环境下动态链接，确保 glibc >= 2.31 兼容性。
- **Windows 二进制**：建议使用 MSVC 2022 编译，目标平台 x64，需包含 VC++ 运行时或静态链接。
- 文件大小限制：Qt 资源系统支持大文件，但建议使用 zstd/gz 压缩后嵌入，在 `DeployEngine` 中解压。
- 签名：发布前对 EXE 使用代码签名证书签名，避免 SmartScreen 告警。
