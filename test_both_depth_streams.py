#!/usr/bin/env python3
"""
测试同时订阅 depth@100ms 和 depth20@100ms
"""

import asyncio
import websockets
import json

async def test_both_streams():
    # 测试1：只订阅depth20@100ms
    print("测试1：只订阅 depth20@100ms")
    print("="*80)
    url1 = "wss://stream.binance.com:9443/stream?streams=btcusdt@depth20@100ms"
    
    try:
        async with websockets.connect(url1) as ws:
            print("✓ 连接成功")
            count = 0
            for _ in range(5):
                data = await asyncio.wait_for(ws.recv(), timeout=2)
                parsed = json.loads(data)
                if 'stream' in parsed:
                    count += 1
            print(f"✓ 收到 {count} 条depth20@100ms消息\n")
    except Exception as e:
        print(f"✗ 错误: {e}\n")
    
    # 测试2：同时订阅depth@100ms和depth20@100ms
    print("测试2：同时订阅 depth@100ms 和 depth20@100ms")
    print("="*80)
    url2 = "wss://stream.binance.com:9443/stream?streams=btcusdt@depth@100ms/btcusdt@depth20@100ms"
    
    try:
        async with websockets.connect(url2) as ws:
            print("✓ 连接成功")
            streams_seen = set()
            for _ in range(20):
                try:
                    data = await asyncio.wait_for(ws.recv(), timeout=2)
                    parsed = json.loads(data)
                    if 'stream' in parsed:
                        streams_seen.add(parsed['stream'])
                except asyncio.TimeoutError:
                    break
            
            print(f"收到的数据流:")
            for s in sorted(streams_seen):
                print(f"  ✓ {s}")
            
            if 'btcusdt@depth20@100ms' in streams_seen:
                print("\n✅ depth20@100ms 工作正常！")
            else:
                print("\n❌ depth20@100ms 没有数据！")
                
    except Exception as e:
        print(f"✗ 错误: {e}\n")

if __name__ == "__main__":
    asyncio.run(test_both_streams())

