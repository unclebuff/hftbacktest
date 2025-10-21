#!/usr/bin/env python3
"""
检查depth20@100ms的实际数据格式
"""

import asyncio
import websockets
import json

async def check_format():
    url = "wss://stream.binance.com:9443/ws/btcusdt@depth20@100ms"
    
    print("连接并获取depth20@100ms数据...")
    print("="*80)
    
    async with websockets.connect(url) as ws:
        for i in range(3):
            data = await ws.recv()
            parsed = json.loads(data)
            
            print(f"\n消息 #{i+1}:")
            print(json.dumps(parsed, indent=2)[:500])
            print("...")
            
            print("\n关键字段检查:")
            print(f"  有'stream'字段: {'stream' in parsed}")
            print(f"  有'data'字段: {'data' in parsed}")
            
            if 'data' in parsed:
                print(f"  data中有's'字段: {'s' in parsed['data']}")
                print(f"  data中有'e'字段: {'e' in parsed['data']}")
            else:
                # 可能是单流订阅，数据直接在顶层
                print(f"  顶层有's'字段: {'s' in parsed}")
                print(f"  顶层有'bids'字段: {'bids' in parsed}")
                print(f"  顶层有'asks'字段: {'asks' in parsed}")

if __name__ == "__main__":
    asyncio.run(check_format())

