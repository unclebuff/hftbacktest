#!/usr/bin/env python3
"""
测试币安现货是否支持 depth20 数据流
"""

import asyncio
import websockets
import json

async def test_binance_spot_depth20():
    """测试币安现货depth20数据流"""
    
    # 测试不同的数据流格式
    test_streams = [
        "btcusdt@depth20",           # 方式1：直接depth20
        "btcusdt@depth20@250ms",     # 方式2：depth20@250ms
        "btcusdt@depth20@100ms",     # 方式3：depth20@100ms（应该不支持）
    ]
    
    for stream in test_streams:
        print(f"\n{'='*80}")
        print(f"测试数据流: {stream}")
        print(f"{'='*80}")
        
        url = f"wss://stream.binance.com:9443/ws/{stream}"
        
        try:
            async with websockets.connect(url) as websocket:
                print(f"✓ WebSocket连接成功")
                
                # 等待接收数据
                print(f"等待接收数据（5秒）...")
                
                try:
                    data = await asyncio.wait_for(websocket.recv(), timeout=5.0)
                    parsed = json.loads(data)
                    print(f"✓ 收到数据！")
                    print(f"  Stream: {parsed.get('stream', 'N/A')}")
                    print(f"  Event: {parsed.get('e', 'N/A')}")
                    
                    if 'data' in parsed:
                        depth_data = parsed['data']
                        print(f"  Symbol: {depth_data.get('s', 'N/A')}")
                        bids_len = len(depth_data.get('bids', []))
                        asks_len = len(depth_data.get('asks', []))
                        print(f"  买盘档位: {bids_len}")
                        print(f"  卖盘档位: {asks_len}")
                        
                        if bids_len > 0:
                            print(f"  最佳买价: {depth_data['bids'][0][0]}")
                        if asks_len > 0:
                            print(f"  最佳卖价: {depth_data['asks'][0][0]}")
                    
                    print(f"\n✅ 数据流 {stream} 存在且工作正常！")
                    
                except asyncio.TimeoutError:
                    print(f"✗ 5秒内没有收到数据")
                    print(f"❌ 数据流 {stream} 可能不存在或不支持")
                    
        except Exception as e:
            print(f"✗ 连接失败: {e}")
            print(f"❌ 数据流 {stream} 不支持")
        
        await asyncio.sleep(1)

if __name__ == "__main__":
    print("币安现货 depth20 数据流测试")
    print(f"{'='*80}\n")
    asyncio.run(test_binance_spot_depth20())
    print(f"\n{'='*80}")
    print("测试完成")

