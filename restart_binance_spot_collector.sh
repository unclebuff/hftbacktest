#!/bin/bash

# 币安现货collector重启脚本
# 用于应用订单簿深度数据收集的修复

set -e

echo "============================================"
echo "重启币安现货collector"
echo "============================================"

# 检查旧进程
echo ""
echo "1. 检查当前运行的现货collector进程..."
OLD_PID=$(pgrep -f "collector.*binancespot" || echo "")

if [ -z "$OLD_PID" ]; then
    echo "   ✓ 没有发现旧的binancespot进程"
else
    echo "   找到旧进程 PID: $OLD_PID"
    echo ""
    echo "2. 停止旧进程..."
    pkill -f "collector.*binancespot"
    sleep 2
    echo "   ✓ 旧进程已停止"
fi

# 确保使用最新编译的版本
echo ""
echo "3. 验证编译版本..."
cd /home/hft/hftbacktest/collector
BINARY_PATH="./target/release/collector"

if [ ! -f "$BINARY_PATH" ]; then
    echo "   ✗ 未找到编译后的可执行文件，开始编译..."
    cargo build --release
else
    echo "   ✓ 找到可执行文件: $BINARY_PATH"
fi

# 启动新进程
echo ""
echo "4. 启动新的collector进程..."
DATA_PATH="/data/shared/hft-trading-data/binance/spot"
EXCHANGE="binancespot"
SYMBOLS="BTCUSDT ETHUSDT SOLUSDT"

# 确保数据目录存在
mkdir -p "$DATA_PATH"

# 启动collector（后台运行）
nohup $BINARY_PATH $DATA_PATH $EXCHANGE $SYMBOLS > /tmp/binancespot_collector.log 2>&1 &
NEW_PID=$!

sleep 2

# 验证新进程
if ps -p $NEW_PID > /dev/null; then
    echo "   ✓ 新进程已启动 PID: $NEW_PID"
else
    echo "   ✗ 新进程启动失败，查看日志:"
    tail -20 /tmp/binancespot_collector.log
    exit 1
fi

# 显示日志输出
echo ""
echo "5. 查看最新日志 (5秒)..."
sleep 5
echo "----------------------------------------"
tail -20 /tmp/binancespot_collector.log
echo "----------------------------------------"

# 显示进程状态
echo ""
echo "6. 当前collector进程状态:"
ps aux | grep -E "collector.*(binancespot|binancefutures)" | grep -v grep

echo ""
echo "============================================"
echo "✅ 重启完成！"
echo "============================================"
echo ""
echo "数据目录: $DATA_PATH"
echo "日志文件: /tmp/binancespot_collector.log"
echo ""
echo "监控命令:"
echo "  查看日志: tail -f /tmp/binancespot_collector.log"
echo "  查看进程: ps aux | grep collector"
echo "  测试数据: python3 /home/hft/hftbacktest/test_binance_spot_depth.py"
echo ""

