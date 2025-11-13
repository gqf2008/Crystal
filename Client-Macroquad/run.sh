#!/bin/bash
# macOS 启动脚本 - 确保窗口正确激活

cd "$(dirname "$0")"

# 编译 release 版本
echo "🔨 编译中..."
cargo build --release

if [ $? -eq 0 ]; then
    echo "✅ 编译成功"
    echo "🚀 启动程序..."
    # 使用 open 命令启动，确保窗口获得焦点
    open target/release/client_macroquad
else
    echo "❌ 编译失败"
    exit 1
fi
