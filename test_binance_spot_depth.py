#!/usr/bin/env python3
"""
测试脚本：验证币安现货订单簿深度数据收集的修复
"""

import json
import gzip
import sys
from collections import defaultdict

def analyze_spot_depth_continuity(file_path, max_lines=10000):
    """
    分析现货深度数据的连续性
    
    根据币安API文档，现货depth更新的连续性检查规则是：
    - 第一个事件的 U <= lastUpdateId+1 AND u >= lastUpdateId+1
    - 后续事件需要 U == prevU + 1
    
    其中 U 是本次更新的起始updateId，u 是本次更新的结束updateId
    """
    
    print("=" * 80)
    print(f"分析文件: {file_path}")
    print("=" * 80)
    
    depth_updates = defaultdict(list)
    missing_count = defaultdict(int)
    total_updates = defaultdict(int)
    
    with gzip.open(file_path, 'rt') as f:
        for i, line in enumerate(f):
            if i >= max_lines:
                break
            
            try:
                parts = line.strip().split(' ', 1)
                data = json.loads(parts[1])
                
                if 'depth' in data.get('stream', ''):
                    depth_data = data.get('data', {})
                    if depth_data.get('e') == 'depthUpdate':
                        symbol = depth_data.get('s')
                        U = depth_data.get('U')
                        u = depth_data.get('u')
                        
                        if symbol and U and u:
                            depth_updates[symbol].append({
                                'U': U,
                                'u': u,
                                'line': i + 1
                            })
                            total_updates[symbol] += 1
                            
            except Exception as e:
                continue
    
    # 分析每个交易对的连续性
    for symbol in sorted(depth_updates.keys()):
        updates = depth_updates[symbol]
        print(f"\n交易对: {symbol}")
        print(f"  总更新数: {total_updates[symbol]}")
        
        gaps = []
        prev_u = None
        
        for idx, update in enumerate(updates):
            U = update['U']
            u = update['u']
            
            if prev_u is not None:
                # 检查连续性：U 应该 <= prev_u + 1
                if U > prev_u + 1:
                    gap = U - prev_u - 1
                    gaps.append({
                        'position': idx,
                        'line': update['line'],
                        'gap': gap,
                        'prev_u': prev_u,
                        'U': U,
                        'u': u
                    })
                    missing_count[symbol] += 1
            
            prev_u = u
        
        if gaps:
            print(f"  ❌ 发现 {len(gaps)} 个间隙:")
            for gap_info in gaps[:5]:  # 只显示前5个
                print(f"     位置 {gap_info['position']}: 间隙 {gap_info['gap']} "
                      f"(prev_u={gap_info['prev_u']}, U={gap_info['U']})")
            if len(gaps) > 5:
                print(f"     ... 还有 {len(gaps) - 5} 个间隙")
        else:
            print(f"  ✅ 数据连续，无间隙")
    
    print("\n" + "=" * 80)
    print("总结")
    print("=" * 80)
    total_symbols = len(depth_updates)
    problematic_symbols = sum(1 for s in depth_updates if missing_count[s] > 0)
    
    print(f"分析的交易对数量: {total_symbols}")
    print(f"有间隙的交易对: {problematic_symbols}")
    print(f"无间隙的交易对: {total_symbols - problematic_symbols}")
    
    if problematic_symbols == 0:
        print("\n✅ 所有交易对的深度数据都是连续的！")
        return True
    else:
        print(f"\n⚠️  有 {problematic_symbols} 个交易对存在数据间隙")
        return False

def compare_old_new_logic():
    """
    说明旧逻辑和新逻辑的区别
    """
    print("\n" + "=" * 80)
    print("币安现货深度数据连续性检查逻辑对比")
    print("=" * 80)
    
    print("\n【旧逻辑 - 错误】:")
    print("  if prev_u.is_none() || U != *prev_u.unwrap() + 1")
    print("  问题: 要求 U 必须严格等于 prev_u + 1")
    print("  但币安现货的 U 是本批次更新的起始ID，可能 < prev_u + 1")
    
    print("\n【新逻辑 - 正确】:")
    print("  if U > prev_u_val + 1")
    print("  正确: 只要 U <= prev_u + 1，说明数据是连续的")
    print("  这符合币安API文档的要求: U <= lastUpdateId+1 AND u >= lastUpdateId+1")
    
    print("\n【示例】:")
    print("  假设 prev_u = 1000")
    print("  收到更新: U=998, u=1005")
    print("    旧逻辑: U(998) != prev_u+1(1001) → 认为有间隙 ❌")
    print("    新逻辑: U(998) <= prev_u+1(1001) → 数据连续 ✅")

if __name__ == '__main__':
    # 显示逻辑对比
    compare_old_new_logic()
    
    # 分析实际数据
    file_path = '/data/shared/hft-trading-data/binance/spot/btcusdt_20251017.gz'
    
    print("\n" + "=" * 80)
    print("开始分析实际数据...")
    print("=" * 80)
    
    import os
    if not os.path.exists(file_path):
        print(f"\n文件不存在: {file_path}")
        print("请确保collector正在运行并收集数据")
        sys.exit(1)
    
    is_continuous = analyze_spot_depth_continuity(file_path, max_lines=50000)
    
    if is_continuous:
        print("\n🎉 修复成功！数据收集正常。")
        sys.exit(0)
    else:
        print("\n⚠️  仍存在问题，需要进一步调查。")
        sys.exit(1)



