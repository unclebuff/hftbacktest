#!/usr/bin/env python3
"""
使用collector的确切URL进行测试
"""

import asyncio
import websockets
import json

async def test_exact_url():
    url = "wss://stream.binance.com:9443/stream?streams=btcusdt@trade/btcusdt@bookTicker/btcusdt@depth@100ms/btcusdt@depth20@100ms/ethusdt@trade/ethusdt@bookTicker/ethusdt@depth@100ms/ethusdt@depth20@100ms/solusdt@trade/solusdt@bookTicker/solusdt@depth@100ms/solusdt@depth20@100ms"
    
    print("测试collector的确切URL")
    print("="*80)
    print(f"URL: {url[:100]}...")
    print("\n连接WebSocket...")
    
    stream_counts = {}
    
    try:
        async with websockets.connect(url) as websocket:
            print("✓ 连接成功！\n")
            print("收集30秒数据...")
            
            start_time = asyncio.get_event_loop().time()
            total_messages = 0
            
            while asyncio.get_event_loop().time() - start_time < 30:
                try:
                    data = await asyncio.wait_for(websocket.recv(), timeout=2.0)
                    total_messages += 1
                    
                    parsed = json.loads(data)
                    if 'stream' in parsed:
                        stream = parsed['stream']
                        stream_counts[stream] = stream_counts.get(stream, 0) + 1
                        
                except asyncio.TimeoutError:
                    continue
            
            print(f"\n✓ 收集完成！总消息数: {total_messages}")
            print("\n各数据流统计:")
            print("="*80)
            
            # 按交易对分组
            for symbol in ['btcusdt', 'ethusdt', 'solusdt']:
                print(f"\n{symbol.upper()}:")
                
                types = {
                    'trade': f"{symbol}@trade",
                    'bookTicker': f"{symbol}@bookTicker",
                    'depth@100ms': f"{symbol}@depth@100ms",
                    'depth20@100ms': f"{symbol}@depth20@100ms",
                }
                
                for type_name, stream_name in types.items():
                    count = stream_counts.get(stream_name, 0)
                    if count > 0:
                        freq = count / 30
                        status = "✓"
                    else:
                        freq = 0
                        status = "✗"
                    print(f"  {status} {type_name:15s}: {count:4d} 消息 ({freq:.2f}/秒)")
            
            print("\n"+"="*80)
            
            # 重点检查depth20
            depth20_streams = [s for s in stream_counts if 'depth20' in s]
            if depth20_streams:
                print("✅ depth20@100ms 数据流正常工作！")
                for s in depth20_streams:
                    print(f"   {s}: {stream_counts[s]} 条消息")
            else:
                print("❌ depth20@100ms 数据流没有数据！")
                
    except Exception as e:
        print(f"✗ 错误: {e}")

if __name__ == "__main__":
    asyncio.run(test_exact_url())

