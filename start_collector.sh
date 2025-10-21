#!/bin/bash
# HFT数据收集程序 - 快速启动脚本

echo "=== HFT数据收集程序启动助手 ==="
echo

# 检查当前目录
if [[ ! -f "./target/release/collector" ]]; then
    echo "❌ 错误：请在 /home/hft/hftbacktest 目录下运行此脚本"
    exit 1
fi

# 检查是否有程序在运行
if pgrep -f "collector.*binancefutures" > /dev/null; then
    echo "⚠️  警告：检测到collector程序正在运行"
    echo "当前运行的进程："
    ps aux | grep collector | grep -v grep
    echo
    read -p "是否要停止现有程序？(y/N): " stop_existing
    if [[ $stop_existing =~ ^[Yy]$ ]]; then
        pkill -f "collector.*binancefutures"
        echo "✅ 已停止现有程序"
        sleep 2
    else
        echo "❌ 取消启动"
        exit 1
    fi
fi

# 检查磁盘空间
echo "🔍 检查磁盘空间..."
available_space=$(df /data/shared/ | tail -1 | awk '{print $4}')
if [[ $available_space -lt 10485760 ]]; then  # 10GB in KB
    echo "⚠️  警告：可用磁盘空间不足10GB"
    df -h /data/shared/
    read -p "是否继续？(y/N): " continue_anyway
    if [[ ! $continue_anyway =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# 设置默认参数
DATA_PATH="/data/shared/hft-trading-data"
EXCHANGE="binancefutures"
DEFAULT_SYMBOLS="BTCUSDT ETHUSDT SOLUSDT"

echo "📝 配置参数："
echo "数据路径: $DATA_PATH"
echo "交易所: $EXCHANGE"
echo "默认交易对: $DEFAULT_SYMBOLS"
echo

# 询问用户是否自定义交易对
read -p "是否使用默认交易对？(Y/n): " use_default
if [[ $use_default =~ ^[Nn]$ ]]; then
    echo "请输入交易对（空格分隔，如：BTCUSDT ETHUSDT ADAUSDT）："
    read -r SYMBOLS
    if [[ -z "$SYMBOLS" ]]; then
        SYMBOLS=$DEFAULT_SYMBOLS
        echo "使用默认交易对: $SYMBOLS"
    fi
else
    SYMBOLS=$DEFAULT_SYMBOLS
fi

# 构建启动命令
CMD="./target/release/collector $DATA_PATH $EXCHANGE $SYMBOLS"
LOG_FILE="collector_$(date +%Y%m%d_%H%M%S).log"

echo
echo "🚀 准备启动数据收集程序..."
echo "命令: $CMD"
echo "日志文件: $LOG_FILE"
echo

read -p "确认启动？(Y/n): " confirm_start
if [[ ! $confirm_start =~ ^[Nn]$ ]]; then
    # 启动程序
    echo "✅ 启动中..."
    nohup $CMD > "$LOG_FILE" 2>&1 &
    PID=$!
    
    # 等待程序启动
    sleep 3
    
    # 检查进程是否存在
    if kill -0 $PID 2>/dev/null; then
        echo "🎉 程序启动成功！"
        echo "进程ID: $PID"
        echo "日志文件: $LOG_FILE"
        echo
        echo "📊 监控命令："
        echo "查看进程: ps aux | grep collector"
        echo "查看日志: tail -f $LOG_FILE"
        echo "查看数据: ls -lah $DATA_PATH/"
        echo "停止程序: kill $PID"
        echo
        
        # 显示前几行日志
        echo "📋 最新日志："
        tail -10 "$LOG_FILE"
        
    else
        echo "❌ 程序启动失败，请检查日志："
        cat "$LOG_FILE"
    fi
else
    echo "❌ 取消启动"
fi