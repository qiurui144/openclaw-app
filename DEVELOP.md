# DEVELOP - 开发与构建指南

## 环境要求

| 组件 | 版本要求 |
|------|---------|
| CMake | ≥ 3.16 |
| Qt | 6.4+（回退支持 Qt 5.15） |
| C++ 编译器 | GCC 10+ / Clang 12+ / MSVC 2022 |
| Ninja | 任意版本 |

## 本地开发构建（Linux）

```bash
# 安装依赖（Ubuntu 22.04/24.04）
sudo apt install cmake ninja-build qt6-base-dev qt6-tools-dev \
                 qt6-wayland libqt6network6 librsvg2-bin

# 配置并编译
cmake -B build/debug -G Ninja -DCMAKE_BUILD_TYPE=Debug
cmake --build build/debug

# 运行
./build/debug/openclaw
```

## 打包 AppImage（Linux）

```bash
# 推荐在 Ubuntu 20.04 Docker 环境中运行，以保证 glibc 兼容性
docker run --rm -v "$PWD:/app" -w /app ubuntu:20.04 bash -c "
  apt-get update -q && \
  apt-get install -y cmake ninja-build qt6-base-dev qt6-tools-dev && \
  bash scripts/build_appimage.sh
"

# 或在宿主机直接运行（需已安装 linuxdeploy）
bash scripts/build_appimage.sh
```

输出：`dist/openclaw-1.0.0-x86_64.AppImage`

## 打包 Windows EXE

### 方式一：在 Windows 上原生编译（推荐）

1. 安装 Qt 6.x（选择 MSVC 2022 64-bit 组件）
2. 安装 Visual Studio 2022 + NSIS
3. 执行脚本（PowerShell）：

```powershell
cmake -B build\windows -G "Visual Studio 17 2022" -A x64 `
      -DCMAKE_PREFIX_PATH="C:\Qt\6.x.x\msvc2022_64"
cmake --build build\windows --config Release
cd build\windows\Release
windeployqt --release --dir deploy openclaw.exe
copy openclaw.exe deploy\
makensis ..\..\scripts\openclaw.nsi
```

### 方式二：Linux 交叉编译（实验性）

```bash
# 安装 MXE（需要约 5GB 磁盘空间）
# 参考：https://mxe.cc/#tutorial
bash scripts/build_windows.sh
```

输出：`dist/openclaw-1.0.0-win64.exe`

## 内置二进制文件

服务程序二进制需手动放置后在 `resources/resources.qrc` 中取消注释：

```
resources/binaries/
├── linux/
│   ├── openclaw-server      # Linux 服务主程序
│   └── openclaw-config      # Linux 配置工具
└── windows/
    ├── openclaw-server.exe  # Windows 服务主程序
    └── openclaw-config.exe  # Windows 配置工具
```

详见 [resources/binaries/README.md](resources/binaries/README.md)。

## 代码结构

```
src/
├── main.cpp                     # 入口：QApplication + DeployWizard
├── DeployWizard.h/cpp           # 主向导窗口（QWizard）
├── core/
│   ├── DeployEngine.h/cpp       # 文件提取、服务安装（异步）
│   └── SystemCheck.h/cpp        # OS 版本、磁盘、端口检查
└── pages/
    ├── WelcomePage              # 欢迎页
    ├── SystemCheckPage          # 环境检查
    ├── InstallConfigPage        # 安装路径
    ├── ServiceConfigPage        # 端口/密码配置
    ├── PlatformIntegrationPage  # 企业微信/QQ/钉钉/飞书
    ├── DeploymentPage           # 部署进度
    └── FinishPage               # 完成 + 打开控制台
```

## 添加新平台集成

在 `PlatformIntegrationPage.cpp` 中调用 `addPlatformRow()` 添加新行，并在 `DeployEngine.cpp` 的 `writePlatformConfig()` 中写入对应配置字段。

## 发布检查清单

- [ ] 更新 `CMakeLists.txt` 中的 `VERSION`
- [ ] 更新 `resources/windows/openclaw.rc` 中的版本号
- [ ] 内置二进制通过安全扫描
- [ ] 在 Ubuntu 20.04 上测试 AppImage
- [ ] 在 Windows 10 和 Windows 11 上测试 EXE
- [ ] Windows EXE 使用代码签名证书签名
