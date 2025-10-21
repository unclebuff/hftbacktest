#!/usr/bin/env python3
"""
对比所有数据流的格式
"""

import asyncio
import websockets
import json

async def check_stream(stream_name):
    url = f"wss://stream.binance.com:9443/stream?streams={stream_name}"
    
    print(f"\n{stream_name}")
    print("="*80)
    
    try:
        async with websockets.connect(url) as ws:
            data = await asyncio.wait_for(ws.recv(), timeout=3)
            parsed = json.loads(data)
            
            has_stream = 'stream' in parsed
            has_data = 'data' in parsed
            
            print(f"有'stream'字段: {has_stream}")
            print(f"有'data'字段: {has_data}")
            
            if has_data:
                data_obj = parsed['data']
                has_s = 's' in data_obj
                has_e = 'e' in data_obj
                
                print(f"data中有's'字段: {has_s}")
                print(f"data中有'e'字段: {has_e}")
                
                if has_e:
                    print(f"event type: {data_obj['e']}")
                
                # 显示所有顶层字段
                print(f"data的所有字段: {list(data_obj.keys())[:10]}")
                
            print(f"\n示例数据 (前300字符):")
            print(json.dumps(parsed, indent=2)[:300])
            print("...\n")
            
    except asyncio.TimeoutError:
        print("超时\n")
    except Exception as e:
        print(f"错误: {e}\n")

async def main():
    streams = [
        "btcusdt@trade",
        "btcusdt@bookTicker",
        "btcusdt@depth@100ms",
        "btcusdt@depth20@100ms",
    ]
    
    for stream in streams:
        await check_stream(stream)

if __name__ == "__main__":
    asyncio.run(main())

