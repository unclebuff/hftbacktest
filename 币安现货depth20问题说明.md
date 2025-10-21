# 币安现货没有 depth20@100ms 数据流的原因说明

## 问题描述

在检查数据收集时发现：
- ✅ **期货路径**有 `btcusdt@depth20@100ms` 数据流
- ❌ **现货路径**没有 `btcusdt@depth20@100ms` 数据流

## 根本原因

**这不是bug，而是币安API的设计差异！**

### 币安期货 API 支持

期货市场支持灵活的部分深度数据流：
- `<symbol>@depth5@100ms` - 5档，100ms更新
- `<symbol>@depth10@100ms` - 10档，100ms更新
- `<symbol>@depth20@100ms` - 20档，100ms更新 ✅
- `<symbol>@depth@0ms` - 全量，实时更新

### 币安现货 API 限制

现货市场的部分深度数据流**不支持自定义更新频率**：
- `<symbol>@depth5` - 5档，**固定250ms**
- `<symbol>@depth10` - 10档，**固定250ms**
- `<symbol>@depth20` - 20档，**固定250ms**
- `<symbol>@depth@100ms` - 全量，100ms更新 ✅

**关键差异**：
- 期货：`depth20@100ms` ✅ 支持
- 现货：`depth20@100ms` ❌ 不支持，只有 `depth20`（250ms）

## 实际数据验证

### 现货实际收集的数据流
```bash
$ zcat btcusdt_20251017.gz | grep -o '"stream":"[^"]*"' | sort | uniq
"stream":"btcusdt@bookTicker"
"stream":"btcusdt@depth@100ms"     ← 全量深度，100ms
"stream":"btcusdt@trade"
```

### 期货实际收集的数据流
```bash
$ zcat btcusdt_20251017.gz | grep -o '"stream":"[^"]*"' | sort | uniq
"stream":"btcusdt@bookTicker"
"stream":"btcusdt@depth@0ms"       ← 全量深度，实时
"stream":"btcusdt@depth20@100ms"   ← 20档深度，100ms ✅
"stream":"btcusdt@trade"
```

## 代码配置分析

### 当前代码配置（main.rs）

```rust
// 现货配置
"binance" | "binancespot" => {
    let streams = [
        "$symbol@trade", 
        "$symbol@bookTicker", 
        "$symbol@depth@100ms",
        "$symbol@depth20@100ms",  // ← 这个流不存在！
    ]
}

// 期货配置
"binancefutures" | "binancefuturesum" => {
    let streams = [
        "$symbol@trade",
        "$symbol@bookTicker",
        "$symbol@depth@0ms",
        "$symbol@depth20@100ms",  // ← 这个流存在 ✅
    ]
}
```

### WebSocket行为

当订阅一个不存在的数据流时，币安WebSocket不会返回错误，而是：
1. 成功建立连接
2. 只推送**存在的数据流**
3. 不存在的数据流被**静默忽略**

这就是为什么：
- 代码中配置了4个流
- 实际只收到3个流的数据

## 解决方案

### 方案1：移除不支持的数据流（推荐）

修改 `main.rs` 中的现货配置：

```rust
"binance" | "binancespot" => {
    let streams = [
        "$symbol@trade", 
        "$symbol@bookTicker", 
        "$symbol@depth@100ms",
        // 移除：现货不支持 depth20@100ms
        // "$symbol@depth20@100ms",
    ]
        .iter()
        .map(|stream| stream.to_string())
        .collect();

    tokio::spawn(binance::run_collection(streams, args.symbols, writer_tx))
}
```

### 方案2：使用现货支持的部分深度（250ms）

如果确实需要固定档位的快照，可以使用：

```rust
"binance" | "binancespot" => {
    let streams = [
        "$symbol@trade", 
        "$symbol@bookTicker", 
        "$symbol@depth@100ms",      // 全量深度，100ms
        "$symbol@depth20",           // 20档快照，250ms ✅
    ]
}
```

但注意：
- `depth20` 更新频率是 250ms（比 `depth@100ms` 慢）
- 通常 `depth@100ms` 已经足够

### 方案3：保持现状（不推荐但也可行）

由于不存在的流会被静默忽略，保持现状不会导致错误，只是：
- 配置和实际不一致
- 可能让人误以为数据流存在但没收到

## 数据对比

| 数据流 | 期货 | 现货 | 档位 | 更新频率 | 说明 |
|--------|------|------|------|----------|------|
| `depth@0ms` | ✅ | ❌ | 全量 | 实时 | 期货特有 |
| `depth@100ms` | ✅ | ✅ | 全量 | 100ms | 两者都有 |
| `depth20@100ms` | ✅ | ❌ | 20档 | 100ms | **期货特有** |
| `depth20` | ✅ | ✅ | 20档 | 250ms | 两者都有 |
| `depth10` | ✅ | ✅ | 10档 | 250ms | 两者都有 |
| `depth5` | ✅ | ✅ | 5档 | 250ms | 两者都有 |

## 推荐配置

### 期货（保持不变）
```rust
let streams = [
    "$symbol@trade",
    "$symbol@bookTicker",
    "$symbol@depth@0ms",       // 全量实时
    "$symbol@depth20@100ms",   // 20档100ms快照
];
```

### 现货（修改）
```rust
let streams = [
    "$symbol@trade",
    "$symbol@bookTicker",
    "$symbol@depth@100ms",     // 全量100ms（已足够）
    // 移除不存在的 depth20@100ms
];
```

## 为什么 depth@100ms 已经足够？

1. **包含所有档位**：`depth@100ms` 包含完整的订单簿更新
2. **更新频率合适**：100ms对大多数策略已经足够快
3. **数据完整性**：通过维护本地订单簿可以得到任意档位
4. **简化处理**：不需要同时处理全量和部分深度

## 是否需要修复？

### 是否需要立即修复
**否**，因为：
- ✅ 不影响数据收集（不存在的流被忽略）
- ✅ 不影响系统运行
- ✅ 实际收集的3个流已经足够

### 建议修复时机
- 下次代码维护时
- 或者现在修复以保持配置清晰

## 总结

### 关键发现
1. **不是bug**：币安现货API设计就是不支持 `depth20@100ms`
2. **期货独有**：只有期货支持灵活的部分深度更新频率
3. **静默忽略**：WebSocket不会报错，只是不推送数据

### 建议
- 从现货配置中移除 `$symbol@depth20@100ms`
- 或者保持现状（不影响功能，只是配置不清晰）
- 专注使用 `depth@100ms`（已足够满足需求）

### 影响评估
- **数据质量**：✅ 无影响（实际收集的数据正常）
- **系统性能**：✅ 无影响（少订阅不存在的流反而更好）
- **策略开发**：✅ 无影响（`depth@100ms` 包含所有需要的信息）

---

**结论**：这是币安API的设计差异，不是代码bug。可以选择修改配置以保持清晰，但不修改也不影响功能。

