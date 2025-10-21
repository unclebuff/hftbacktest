#!/usr/bin/env python3
"""
模拟collector生成的WebSocket URL
"""

# 配置（来自 main.rs）
streams = [
    "$symbol@trade",
    "$symbol@bookTicker",
    "$symbol@depth@100ms",
    "$symbol@depth20@100ms",
]

symbols = ["BTCUSDT", "ETHUSDT", "SOLUSDT"]

# 模拟collector的URL生成逻辑
all_streams = []
for symbol in symbols:
    s = symbol.lower()
    for stream in streams:
        stream_name = stream.replace("$symbol", s)
        all_streams.append(stream_name)

streams_str = "/".join(all_streams)
url = f"wss://stream.binance.com:9443/stream?streams={streams_str}"

print("Collector生成的WebSocket URL:")
print("="*80)
print(f"URL长度: {len(url)}")
print(f"\n完整URL:\n{url}")
print("\n"+"="*80)
print("订阅的数据流列表:")
for i, stream in enumerate(all_streams, 1):
    print(f"  {i:2d}. {stream}")

print("\n"+"="*80)
print(f"总计: {len(all_streams)} 个数据流")

