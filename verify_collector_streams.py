#!/usr/bin/env python3
"""
验证collector订阅的所有数据流
"""

import asyncio
import websockets
import json

async def test_collector_subscription():
    """测试collector实际订阅的数据流"""
    
    # 模拟collector的订阅
    symbols = ["BTCUSDT", "ETHUSDT", "SOLUSDT"]
    streams = []
    
    for symbol in symbols:
        s = symbol.lower()
        streams.append(f"{s}@trade")
        streams.append(f"{s}@bookTicker")
        streams.append(f"{s}@depth@100ms")
        streams.append(f"{s}@depth20@100ms")
    
    stream_str = "/".join(streams)
    url = f"wss://stream.binance.com:9443/stream?streams={stream_str}"
    
    print("测试collector订阅配置")
    print("="*80)
    print(f"订阅的数据流数量: {len(streams)}")
    print(f"URL长度: {len(url)}")
    print("\n数据流列表:")
    for i, s in enumerate(streams, 1):
        print(f"  {i}. {s}")
    
    print("\n"+"="*80)
    print("连接WebSocket...")
    
    stream_counts = {}
    
    try:
        async with websockets.connect(url) as websocket:
            print("✓ 连接成功！")
            print("\n收集30秒数据...")
            
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
            
            print(f"\n✓ 收集完成！")
            print(f"总消息数: {total_messages}")
            print("\n各数据流统计:")
            print("="*80)
            
            # 按符号分组
            for symbol in symbols:
                s = symbol.lower()
                print(f"\n{symbol}:")
                
                types = {
                    'trade': f"{s}@trade",
                    'bookTicker': f"{s}@bookTicker",
                    'depth@100ms': f"{s}@depth@100ms",
                    'depth20@100ms': f"{s}@depth20@100ms",
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
            print("总结:")
            active_streams = len([s for s in stream_counts if stream_counts[s] > 0])
            print(f"  配置的数据流: {len(streams)}")
            print(f"  活跃的数据流: {active_streams}")
            print(f"  未响应的数据流: {len(streams) - active_streams}")
            
            if active_streams == len(streams):
                print("\n✅ 所有数据流都正常工作！")
            else:
                print(f"\n⚠️  有 {len(streams) - active_streams} 个数据流没有数据")
                
    except Exception as e:
        print(f"✗ 错误: {e}")

if __name__ == "__main__":
    asyncio.run(test_collector_subscription())

