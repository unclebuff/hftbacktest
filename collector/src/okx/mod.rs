use std::collections::HashMap;

use anyhow::Result;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, warn};

use crate::throttler::Throttler;

mod http;

/// Main entry point for OKX market data collection
/// Subscribes to WebSocket channels and transforms to Binance-compatible format
pub async fn run_collection(
    inst_ids: Vec<String>,
    channels: Vec<String>,
    writer_tx: UnboundedSender<(chrono::DateTime<chrono::Utc>, String, String)>,
) -> Result<()> {
    let (ws_tx, mut ws_rx) = tokio::sync::mpsc::unbounded_channel();

    // Create throttler to limit REST API calls (20/2s = 600/min for OKX public endpoints)
    let mut throttler = Throttler::new(20);

    // Spawn connection handler
    tokio::spawn(async move {
        http::keep_connection(channels, inst_ids, ws_tx).await;
    });

    // Track last update IDs for depth snapshot validation
    let mut last_update_id_map: HashMap<String, i64> = HashMap::new();

    // Process incoming WebSocket messages
    while let Some((recv_time, text)) = ws_rx.recv().await {
        match serde_json::from_str::<Value>(&text) {
            Ok(json) => {
                // Handle subscription responses
                if let Some(event) = json.get("event").and_then(|v| v.as_str()) {
                    if event == "subscribe" {
                        continue;
                    } else if event == "error" {
                        error!(?json, "OKX subscription error");
                        continue;
                    }
                }

                // Process data messages
                if let Some(arg) = json.get("arg") {
                    if let Some(data_array) = json.get("data").and_then(|v| v.as_array()) {
                        let channel = arg.get("channel").and_then(|v| v.as_str()).unwrap_or("");
                        let inst_id = arg.get("instId").and_then(|v| v.as_str()).unwrap_or("");
                        
                        let symbol = inst_id.replace("-", "").to_lowercase();

                        for data in data_array {
                                // Transform OKX data to Binance-compatible format
                            match channel {
                                "trades" => {
                                    if let Some(binance_trade) = transform_trade(data, &symbol, recv_time) {
                                        if writer_tx.send((recv_time, symbol.clone(), binance_trade)).is_err() {
                                            break;
                                        }
                                    }
                                }
                                "bbo-tbt" => {
                                    if let Some(binance_ticker) = transform_bbo(data, &symbol, recv_time) {
                                        if writer_tx.send((recv_time, symbol.clone(), binance_ticker)).is_err() {
                                            break;
                                        }
                                    }
                                }
                                "books" | "books5" | "books-l2-tbt" => {
                                    // Handle depth updates
                                    let action = json.get("action").and_then(|v| v.as_str());
                                    
                                    if action == Some("snapshot") {
                                        // Fetch full snapshot via REST API with throttling
                                        if let Some(snapshot_result) = throttler.execute(http::fetch_depth_snapshot(inst_id)).await {
                                        match snapshot_result {
                                            Ok(snapshot_text) => {
                                                if let Ok(snapshot_json) = serde_json::from_str::<Value>(&snapshot_text) {
                                                    if let Some(snapshot_data) = snapshot_json
                                                        .get("data")
                                                        .and_then(|v| v.as_array())
                                                        .and_then(|arr| arr.first())
                                                    {
                                                        if let Some(binance_depth) = transform_depth_snapshot(
                                                            snapshot_data,
                                                            &symbol,
                                                            recv_time,
                                                        ) {
                                                            // Update tracking map
                                                            if let Some(u) = snapshot_data.get("ts")
                                                                .and_then(|v| v.as_str())
                                                                .and_then(|s| s.parse::<i64>().ok())
                                                            {
                                                                last_update_id_map.insert(symbol.clone(), u);
                                                            }
                                                            
                                                            if writer_tx.send((recv_time, symbol.clone(), binance_depth)).is_err() {
                                                                break;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                error!(?e, inst_id, "Failed to fetch OKX depth snapshot");
                                            }
                                        }
                                        }
                                    } else if action == Some("update") {
                                        // Incremental depth update
                                        if let Some(binance_depth) = transform_depth_update(
                                            data,
                                            &symbol,
                                            recv_time,
                                            &last_update_id_map,
                                        ) {
                                            // Update last update ID
                                            if let Some(u) = data.get("ts")
                                                .and_then(|v| v.as_str())
                                                .and_then(|s| s.parse::<i64>().ok())
                                            {
                                                last_update_id_map.insert(symbol.clone(), u);
                                            }
                                            
                                            if writer_tx.send((recv_time, symbol.clone(), binance_depth)).is_err() {
                                                break;
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    warn!(channel, "Unknown OKX channel");
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let text_str = text.to_string();
                error!(?e, text = %text_str, "Failed to parse OKX WebSocket message");
            }
        }
    }

    Ok(())
}

/// Transform OKX trade to Binance trade format
/// OKX: {"instId":"BTC-USDT","tradeId":"...","px":"...","sz":"...","side":"buy","ts":"..."}
/// Binance: {"stream":"btcusdt@trade","data":{"e":"trade","E":...,"s":"BTCUSDT","t":...,"p":"...","q":"...","T":...}}
fn transform_trade(data: &Value, symbol: &str, recv_time: chrono::DateTime<chrono::Utc>) -> Option<String> {
    let trade_id = data.get("tradeId").and_then(|v| v.as_str())?;
    let price = data.get("px").and_then(|v| v.as_str())?;
    let qty = data.get("sz").and_then(|v| v.as_str())?;
    let timestamp = data.get("ts").and_then(|v| v.as_str())?.parse::<i64>().ok()?;
    
    let binance_format = serde_json::json!({
        "stream": format!("{}@trade", symbol),
        "data": {
            "e": "trade",
            "E": recv_time.timestamp_millis(),
            "s": symbol.to_uppercase(),
            "t": trade_id.parse::<i64>().unwrap_or(0),
            "p": price,
            "q": qty,
            "T": timestamp
        }
    });
    
    serde_json::to_string(&binance_format).ok()
}

/// Transform OKX BBO (best bid/offer) to Binance bookTicker format
/// OKX: {"asks":[["price","size","...",""]],"bids":[["price","size","...",""]],"ts":"..."}
/// Binance: {"stream":"btcusdt@bookTicker","data":{"u":...,"s":"BTCUSDT","b":"...","B":"...","a":"...","A":"..."}}
fn transform_bbo(data: &Value, symbol: &str, _recv_time: chrono::DateTime<chrono::Utc>) -> Option<String> {
    let asks = data.get("asks").and_then(|v| v.as_array())?;
    let bids = data.get("bids").and_then(|v| v.as_array())?;
    let timestamp = data.get("ts").and_then(|v| v.as_str())?.parse::<i64>().ok()?;
    
    let best_ask = asks.first().and_then(|a| a.as_array())?;
    let best_bid = bids.first().and_then(|b| b.as_array())?;
    
    let ask_price = best_ask.get(0).and_then(|v| v.as_str())?;
    let ask_qty = best_ask.get(1).and_then(|v| v.as_str())?;
    let bid_price = best_bid.get(0).and_then(|v| v.as_str())?;
    let bid_qty = best_bid.get(1).and_then(|v| v.as_str())?;
    
    let binance_format = serde_json::json!({
        "stream": format!("{}@bookTicker", symbol),
        "data": {
            "u": timestamp,
            "s": symbol.to_uppercase(),
            "b": bid_price,
            "B": bid_qty,
            "a": ask_price,
            "A": ask_qty
        }
    });
    
    serde_json::to_string(&binance_format).ok()
}

/// Transform OKX depth snapshot to Binance depth format
/// OKX: {"asks":[["price","size","...",""]],"bids":[["price","size","...",""]],"ts":"..."}
/// Binance: {"stream":"btcusdt@depth","data":{"e":"depthUpdate","E":...,"s":"BTCUSDT","U":...,"u":...,"b":[["price","qty"]],"a":[["price","qty"]]}}
fn transform_depth_snapshot(
    data: &Value,
    symbol: &str,
    recv_time: chrono::DateTime<chrono::Utc>,
) -> Option<String> {
    let asks = data.get("asks").and_then(|v| v.as_array())?;
    let bids = data.get("bids").and_then(|v| v.as_array())?;
    let timestamp = data.get("ts").and_then(|v| v.as_str())?.parse::<i64>().ok()?;
    
    // Convert OKX format [price, size, liquidated_orders, num_orders] to Binance [price, qty]
    let binance_asks: Vec<Vec<String>> = asks
        .iter()
        .filter_map(|a| {
            let arr = a.as_array()?;
            Some(vec![
                arr.get(0)?.as_str()?.to_string(),
                arr.get(1)?.as_str()?.to_string(),
            ])
        })
        .collect();
    
    let binance_bids: Vec<Vec<String>> = bids
        .iter()
        .filter_map(|b| {
            let arr = b.as_array()?;
            Some(vec![
                arr.get(0)?.as_str()?.to_string(),
                arr.get(1)?.as_str()?.to_string(),
            ])
        })
        .collect();
    
    let binance_format = serde_json::json!({
        "stream": format!("{}@depth", symbol),
        "data": {
            "e": "depthUpdate",
            "E": recv_time.timestamp_millis(),
            "s": symbol.to_uppercase(),
            "U": timestamp,
            "u": timestamp,
            "b": binance_bids,
            "a": binance_asks
        }
    });
    
    serde_json::to_string(&binance_format).ok()
}

/// Transform OKX incremental depth update to Binance format
fn transform_depth_update(
    data: &Value,
    symbol: &str,
    recv_time: chrono::DateTime<chrono::Utc>,
    last_update_map: &HashMap<String, i64>,
) -> Option<String> {
    let timestamp = data.get("ts").and_then(|v| v.as_str())?.parse::<i64>().ok()?;
    let prev_update_id = last_update_map.get(symbol).copied().unwrap_or(timestamp - 1);
    
    // Get asks/bids arrays (may be empty for no changes)
    let empty_vec = Vec::new();
    let asks = data.get("asks").and_then(|v| v.as_array()).unwrap_or(&empty_vec);
    let bids = data.get("bids").and_then(|v| v.as_array()).unwrap_or(&empty_vec);
    
    // Convert to Binance format
    let binance_asks: Vec<Vec<String>> = asks
        .iter()
        .filter_map(|a| {
            let arr = a.as_array()?;
            Some(vec![
                arr.get(0)?.as_str()?.to_string(),
                arr.get(1)?.as_str()?.to_string(),
            ])
        })
        .collect();
    
    let binance_bids: Vec<Vec<String>> = bids
        .iter()
        .filter_map(|b| {
            let arr = b.as_array()?;
            Some(vec![
                arr.get(0)?.as_str()?.to_string(),
                arr.get(1)?.as_str()?.to_string(),
            ])
        })
        .collect();
    
    let binance_format = serde_json::json!({
        "stream": format!("{}@depth", symbol),
        "data": {
            "e": "depthUpdate",
            "E": recv_time.timestamp_millis(),
            "s": symbol.to_uppercase(),
            "U": prev_update_id,
            "u": timestamp,
            "b": binance_bids,
            "a": binance_asks
        }
    });
    
    serde_json::to_string(&binance_format).ok()
}
