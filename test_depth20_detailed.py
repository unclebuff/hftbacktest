#!/usr/bin/env python3
"""
详细测试币安现货depth20数据流
"""

import asyncio
import websockets
import json

async def test_stream_detailed(stream, duration=10):
    """详细测试一个数据流"""
    url = f"wss://stream.binance.com:9443/ws/{stream}"
    
    print(f"\n测试: {stream}")
    print(f"URL: {url}")
    print("="*80)
    
    message_count = 0
    
    try:
        async with websockets.connect(url) as websocket:
            print(f"✓ 连接成功")
            
            start_time = asyncio.get_event_loop().time()
            
            while asyncio.get_event_loop().time() - start_time < duration:
                try:
                    data = await asyncio.wait_for(websocket.recv(), timeout=2.0)
                    message_count += 1
                    
                    if message_count <= 3:  # 只显示前3条消息
                        parsed = json.loads(data)
                        print(f"\n消息 #{message_count}:")
                        print(f"  原始数据长度: {len(data)} 字节")
                        
                        # 尝试解析数据
                        if isinstance(parsed, dict):
                            if 'stream' in parsed:
                                print(f"  Stream: {parsed.get('stream')}")
                                if 'data' in parsed:
                                    depth = parsed['data']
                                    print(f"  Event: {depth.get('e')}")
                                    print(f"  Symbol: {depth.get('s')}")
                                    print(f"  买盘档位数: {len(depth.get('bids', []))}")
                                    print(f"  卖盘档位数: {len(depth.get('asks', []))}")
                            else:
                                # 单流订阅格式
                                print(f"  Event: {parsed.get('e')}")
                                print(f"  Symbol: {parsed.get('s')}")
                                bids = parsed.get('bids', [])
                                asks = parsed.get('asks', [])
                                print(f"  买盘档位数: {len(bids)}")
                                print(f"  卖盘档位数: {len(asks)}")
                                if bids:
                                    print(f"  最佳买价: {bids[0][0]}")
                                if asks:
                                    print(f"  最佳卖价: {asks[0][0]}")
                        
                except asyncio.TimeoutError:
                    continue
            
            print(f"\n统计信息:")
            print(f"  总消息数: {message_count}")
            print(f"  测试时长: {duration}秒")
            if message_count > 0:
                print(f"  消息频率: {message_count/duration:.2f} 消息/秒")
                print(f"  ✅ 数据流正常工作！")
            else:
                print(f"  ❌ 没有收到任何数据")
                
    except Exception as e:
        print(f"✗ 错误: {e}")

async def main():
    # 测试两个数据流
    await test_stream_detailed("btcusdt@depth20", duration=10)
    await test_stream_detailed("btcusdt@depth20@100ms", duration=10)

if __name__ == "__main__":
    print("币安现货 depth20 详细测试")
    print("="*80)
    asyncio.run(main())
    print("\n"+"="*80)
    print("测试完成")

