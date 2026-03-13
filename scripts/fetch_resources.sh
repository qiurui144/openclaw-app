#!/usr/bin/env bash
# fetch_resources.sh - 下载 Full Bundle 所需的二进制资源
set -euo pipefail

NODE_VERSION="${NODE_VERSION:-22.17.0}"
RESOURCES_DIR="${RESOURCES_DIR:-resources/binaries}"

mkdir -p "$RESOURCES_DIR/linux" "$RESOURCES_DIR/windows"

echo "下载 Linux Node.js ${NODE_VERSION}..."
curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-x64.tar.xz" \
  | tar -xJ --strip-components=2 -C "$RESOURCES_DIR/linux" "node-v${NODE_VERSION}-linux-x64/bin/node"

echo "下载 Windows Node.js ${NODE_VERSION}..."
curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-win-x64.zip" \
  -o /tmp/node_win.zip
unzip -p /tmp/node_win.zip "node-v${NODE_VERSION}-win-x64/node.exe" > "$RESOURCES_DIR/windows/node.exe"
rm -f /tmp/node_win.zip

echo "资源下载完成"
ls -lh "$RESOURCES_DIR/linux/node" "$RESOURCES_DIR/windows/node.exe"
