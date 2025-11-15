#!/usr/bin/env bash
# 测试纹理加载

cd "$(dirname "$0")"

echo "🧪 测试纹理加载..."
echo "================================"

cargo build --bin test_select 2>&1 | tail -3

echo ""
echo "📸 启动测试程序（按 ESC 退出）..."
cargo run --bin test_select 2>&1 | grep -E "Loaded|纹理|✓|✗|错误|error" || echo "程序正常运行"
