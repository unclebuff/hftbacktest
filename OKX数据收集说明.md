# OKX交易所行情收集说明

## 概述

OKX收集器已经实现，可以收集OKX交易所的实时市场数据并转换为与Binance兼容的格式，使得数据可以与现有的Binance数据处理流程无缝集成。

## 支持的市场类型

### 1. OKX现货市场 (okxspot)
```bash
./target/release/collector /data/shared/hft-trading-data/okx/spot okxspot BTCUSDT ETHUSDT SOLUSDT
```

### 2. OKX永续合约 (okxswap)
```bash
./target/release/collector /data/shared/hft-trading-data/okx/swap okxswap BTCUSDT ETHUSDT SOLUSDT
```

## 数据格式转换

OKX收集器自动将OKX的数据格式转换为Binance兼容格式：

### 1. 交易数据 (Trades)
- **OKX格式**: `{"instId":"BTC-USDT","tradeId":"...","px":"...","sz":"...","side":"buy","ts":"..."}`
- **转换后**: `{"stream":"btcusdt@trade","data":{"e":"trade","E":...,"s":"BTCUSDT","t":...,"p":"...","q":"...","T":...}}`

### 2. 最优买卖价 (Best Bid/Offer)
- **OKX格式**: `{"asks":[["price","size",...]],"bids":[["price","size",...]],"ts":"..."}`
- **转换后**: `{"stream":"btcusdt@bookTicker","data":{"u":...,"s":"BTCUSDT","b":"...","B":"...","a":"...","A":"..."}}`

### 3. 深度数据 (Order Book Depth)
- **OKX格式**: `{"asks":[["price","size",...]],"bids":[["price","size",...]],"ts":"..."}`
- **转换后**: `{"stream":"btcusdt@depth","data":{"e":"depthUpdate","E":...,"s":"BTCUSDT","U":...,"u":...,"b":[["price","qty"]],"a":[["price","qty"]]}}`

## 订阅的频道

默认订阅以下OKX WebSocket频道：
- `trades`: 实时交易数据
- `bbo-tbt`: 最优买卖价（tick-by-tick，10ms更新）
- `books`: 深度数据（400档，100ms更新）

## 符号格式转换

### 现货市场
- 输入: `BTCUSDT`, `ETHUSDT`, `SOLUSDT`
- OKX API格式: `BTC-USDT`, `ETH-USDT`, `SOL-USDT`
- 存储文件名: `btcusdt_20251014.gz`

### 永续合约
- 输入: `BTCUSDT`, `ETHUSDT`, `SOLUSDT`
- OKX API格式: `BTC-USDT-SWAP`, `ETH-USDT-SWAP`, `SOL-USDT-SWAP`
- 存储文件名: `btcusdt_20251014.gz`

## 数据存储目录结构

```
/data/shared/hft-trading-data/
├── binance/
│   ├── futures/
│   │   ├── btcusdt_20251014.gz
│   │   ├── ethusdt_20251014.gz
│   │   └── solusdt_20251014.gz
│   └── spot/
│       ├── btcusdt_20251014.gz
│       ├── ethusdt_20251014.gz
│       └── solusdt_20251014.gz
└── okx/
    ├── spot/
    │   ├── btcusdt_20251014.gz
    │   ├── ethusdt_20251014.gz
    │   └── solusdt_20251014.gz
    └── swap/
        ├── btcusdt_20251014.gz
        ├── ethusdt_20251014.gz
        └── solusdt_20251014.gz
```

## 完整启动示例

### 1. OKX现货市场 - 后台运行
```bash
cd /home/hft/hftbacktest

# 创建数据目录
mkdir -p /data/shared/hft-trading-data/okx/spot

# 启动收集器（后台运行）
nohup ./target/release/collector \
    /data/shared/hft-trading-data/okx/spot \
    okxspot \
    BTCUSDT ETHUSDT SOLUSDT \
    > /data/shared/hft-trading-data/okx/spot/collector.log 2>&1 &

# 记录进程ID
echo $! > /data/shared/hft-trading-data/okx/spot/collector.pid
```

### 2. OKX永续合约 - 后台运行
```bash
cd /home/hft/hftbacktest

# 创建数据目录
mkdir -p /data/shared/hft-trading-data/okx/swap

# 启动收集器（后台运行）
nohup ./target/release/collector \
    /data/shared/hft-trading-data/okx/swap \
    okxswap \
    BTCUSDT ETHUSDT SOLUSDT \
    > /data/shared/hft-trading-data/okx/swap/collector.log 2>&1 &

# 记录进程ID
echo $! > /data/shared/hft-trading-data/okx/swap/collector.pid
```

## 监控和管理

### 查看运行状态
```bash
# 查看OKX现货收集器日志
tail -f /data/shared/hft-trading-data/okx/spot/collector.log

# 查看OKX永续合约收集器日志
tail -f /data/shared/hft-trading-data/okx/swap/collector.log

# 检查进程是否运行
ps aux | grep collector | grep okx
```

### 查看数据收集情况
```bash
# 查看OKX现货数据文件大小
ls -lh /data/shared/hft-trading-data/okx/spot/

# 查看OKX永续合约数据文件大小
ls -lh /data/shared/hft-trading-data/okx/swap/

# 查看实时数据（解压查看前几行）
zcat /data/shared/hft-trading-data/okx/spot/btcusdt_$(date +%Y%m%d).gz | head -5
```

### 停止收集器
```bash
# 停止OKX现货收集器
kill $(cat /data/shared/hft-trading-data/okx/spot/collector.pid)

# 停止OKX永续合约收集器
kill $(cat /data/shared/hft-trading-data/okx/swap/collector.pid)
```

## API限制

OKX公共WebSocket和REST API限制：
- WebSocket: 无特定连接数限制，但建议合理使用
- REST API (深度快照): 20次/2秒

收集器已经实现了自动限流机制，确保不会超过API限制。

## 数据兼容性

由于OKX数据已转换为Binance兼容格式，您可以使用相同的数据处理流程：

1. **数据加载**: 使用hftbacktest的数据加载器直接读取OKX数据
2. **回测**: OKX数据可以像Binance数据一样用于回测
3. **分析**: 所有现有的数据分析工具都可以直接使用

## 技术实现

### WebSocket连接
- 端点: `wss://ws.okx.com:8443/ws/v5/public`
- 自动重连机制（指数退避）
- 连接异常处理

### 数据转换
- 实时转换OKX消息格式到Binance格式
- 保持时间戳精度（毫秒）
- 维护深度更新的连续性（lastUpdateId追踪）

### 性能优化
- 异步IO处理
- 高效的JSON序列化/反序列化
- 压缩存储（gzip）

## 故障排查

### 问题1: 连接失败
```bash
# 检查网络连接
ping www.okx.com

# 检查日志中的错误信息
tail -100 /data/shared/hft-trading-data/okx/spot/collector.log | grep -i error
```

### 问题2: 数据文件未生成
```bash
# 检查目录权限
ls -ld /data/shared/hft-trading-data/okx/spot/

# 检查磁盘空间
df -h /data/shared/
```

### 问题3: 进程意外退出
```bash
# 查看系统日志
journalctl -u collector --since "1 hour ago"

# 检查OOM killer
dmesg | grep -i "out of memory"
```

## 与Binance收集器的对比

| 特性 | Binance | OKX |
|------|---------|-----|
| 数据格式 | Binance原生 | 转换为Binance兼容 |
| WebSocket端点 | stream.binance.com | ws.okx.com |
| 深度更新频率 | 0ms/100ms | 10ms/100ms |
| API限制 | 较宽松 | 20次/2秒(REST) |
| 符号格式 | BTCUSDT | BTC-USDT → btcusdt |

## 扩展支持的交易对

要添加更多交易对，只需在启动命令中追加：

```bash
# 现货市场示例
./target/release/collector \
    /data/shared/hft-trading-data/okx/spot \
    okxspot \
    BTCUSDT ETHUSDT SOLUSDT BNBUSDT ADAUSDT DOGEUSDT

# 永续合约示例
./target/release/collector \
    /data/shared/hft-trading-data/okx/swap \
    okxswap \
    BTCUSDT ETHUSDT SOLUSDT BNBUSDT ADAUSDT DOGEUSDT
```

## 注意事项

1. **数据一致性**: OKX和Binance的价格可能存在差异，这是正常的跨交易所现象
2. **时间同步**: 确保服务器时间准确（使用NTP）
3. **存储空间**: OKX数据量与Binance类似，请预留足够空间
4. **网络稳定性**: 建议使用稳定的网络连接，避免频繁重连

## 支持和反馈

如遇到问题或需要支持，请：
1. 检查日志文件中的详细错误信息
2. 确认网络连接和API访问正常
3. 验证数据目录权限和磁盘空间
