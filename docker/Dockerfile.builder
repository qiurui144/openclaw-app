# ============================================================
# Dockerfile.builder
# OpenClaw 编译镜像：Ubuntu 20.04 + Qt 5.12 + CMake + Ninja
# 使用清华 apt 镜像加速国内下载
# ============================================================
FROM ubuntu:20.04

ARG DEBIAN_FRONTEND=noninteractive

# 替换为清华 apt 镜像（国内加速）
RUN sed -i 's|http://archive.ubuntu.com|http://mirrors.tuna.tsinghua.edu.cn|g' /etc/apt/sources.list && \
    sed -i 's|http://security.ubuntu.com|http://mirrors.tuna.tsinghua.edu.cn|g' /etc/apt/sources.list

# 安装编译依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    ninja-build \
    git \
    pkg-config \
    # Qt5 编译依赖
    qtbase5-dev \
    qttools5-dev \
    # OpenGL（Qt GUI 需要）
    libgl1-mesa-dev \
    libgles2-mesa-dev \
    # 图标转换
    librsvg2-bin \
    # patchelf（appimage-builder 用）
    patchelf \
    # fetch_resources.sh 所需: 下载 Node.js 和执行 npm install
    curl \
    xz-utils \
    unzip \
    python3 \
    # node-gyp 构建原生模块需要 python3
    python3-distutils \
    # 工具
    file \
    && rm -rf /var/lib/apt/lists/*

# 验证关键工具版本
RUN cmake --version | head -1 && \
    dpkg -l libqt5core5a | awk '/^ii/{print "Qt5:", $3}'

WORKDIR /project
