#!/usr/bin/env python3
"""
检查多流WebSocket的depth20@100ms格式
"""

import asyncio
import websockets
import json

async def check_format():
    url = "wss://stream.binance.com:9443/stream?streams=btcusdt@depth20@100ms"
    
    print("连接多流WebSocket并获取depth20@100ms数据...")
    print("="*80)
    
    async with websockets.connect(url) as ws:
        for i in range(3):
            data = await ws.recv()
            parsed = json.loads(data)
            
            print(f"\n消息 #{i+1}:")
            print(json.dumps(parsed, indent=2)[:600])
            print("...")
            
            print("\n关键字段检查:")
            print(f"  有'stream'字段: {'stream' in parsed}")
            print(f"  有'data'字段: {'data' in parsed}")
            
            if 'data' in parsed:
                data_obj = parsed['data']
                print(f"  data中有's'字段: {'s' in data_obj}")
                print(f"  data中有'e'字段: {'e' in data_obj}")
                print(f"  data中有'bids'字段: {'bids' in data_obj}")
                print(f"  data中有'lastUpdateId'字段: {'lastUpdateId' in data_obj}")
                if 'e' in data_obj:
                    print(f"  event type: {data_obj['e']}")

if __name__ == "__main__":
    asyncio.run(check_format())

