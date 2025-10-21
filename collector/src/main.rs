use anyhow::anyhow;
use clap::Parser;
use tokio::{self, select, signal, sync::mpsc::unbounded_channel};
use tracing::{error, info};

use crate::file::Writer;

mod binance;
mod binancefuturescm;
mod binancefuturesum;
mod bybit;
mod error;
mod file;
mod hyperliquid;
mod okx;
mod throttler;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path for the files where collected data will be written.
    path: String,

    /// Name of the exchange
    exchange: String,

    /// Symbols for which data will be collected.
    symbols: Vec<String>,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    tracing_subscriber::fmt::init();

    let (writer_tx, mut writer_rx) = unbounded_channel();

    let handle = match args.exchange.as_str() {
        "binancefutures" | "binancefuturesum" => {
            let streams = [
                "$symbol@trade",
                "$symbol@bookTicker",
                "$symbol@depth@0ms",
                "$symbol@depth20@100ms",
                // "$symbol@@markPrice@1s"
            ]
            .iter()
            .map(|stream| stream.to_string())
            .collect();

            tokio::spawn(binancefuturesum::run_collection(
                streams,
                args.symbols,
                writer_tx,
            ))
        }
        "binancefuturescm" => {
            let streams = [
                "$symbol@trade",
                "$symbol@bookTicker",
                "$symbol@depth@0ms",
                "$symbol@depth20@100ms",
                // "$symbol@@markPrice@1s"
            ]
            .iter()
            .map(|stream| stream.to_string())
            .collect();

            tokio::spawn(binancefuturescm::run_collection(
                streams,
                args.symbols,
                writer_tx,
            ))
        }
        "binance" | "binancespot" => {
            let streams = [
                "$symbol@trade", 
                "$symbol@bookTicker", 
                "$symbol@depth@100ms",      // Full depth, 100ms
                "$symbol@depth20@100ms",    // 20-level snapshot, 100ms (Verified working!)
            ]
                .iter()
                .map(|stream| stream.to_string())
                .collect();

            tokio::spawn(binance::run_collection(streams, args.symbols, writer_tx))
        }
        "bybit" => {
            let topics = [
                "orderbook.50.$symbol",    // 50档深度快照（包含足够的市场深度信息）
                "publicTrade.$symbol",     // 逐笔成交
            ]
            .iter()
            .map(|topic| topic.to_string())
            .collect();

            tokio::spawn(bybit::run_collection(
                topics,
                args.symbols,
                writer_tx,
                "linear",  // 永续合约
            ))
        }
        "bybitspot" => {
            let topics = [
                "orderbook.50.$symbol",    // 50档深度快照
                "publicTrade.$symbol",     // 逐笔成交
            ]
            .iter()
            .map(|topic| topic.to_string())
            .collect();

            tokio::spawn(bybit::run_collection(
                topics,
                args.symbols,
                writer_tx,
                "spot",  // 现货
            ))
        }
        "hyperliquid" => {
            let subscriptions = ["trades", "l2Book", "bbo"]
                .iter()
                .map(|sub| sub.to_string())
                .collect();

            tokio::spawn(hyperliquid::run_collection(
                subscriptions,
                args.symbols,
                writer_tx,
            ))
        }
        "okx" | "okxspot" => {
            // OKX WebSocket public channels
            let channels = [
                "trades",              // 交易数据
                "bbo-tbt",             // 最优买卖价（tick-by-tick）
                "books",               // 增量深度（400档，每100ms推送变化的档位）
                "books5",              // 5档深度快照（每100ms推送完整的5档数据）
            ]
                .iter()
                .map(|ch| ch.to_string())
                .collect();

            // Transform symbols to OKX format (e.g., BTCUSDT -> BTC-USDT)
            let okx_symbols: Vec<String> = args.symbols
                .iter()
                .map(|s| {
                    // Assume format like BTCUSDT, convert to BTC-USDT
                    let upper = s.to_uppercase();
                    if upper.ends_with("USDT") {
                        let base = &upper[..upper.len() - 4];
                        format!("{}-USDT", base)
                    } else if upper.ends_with("USDC") {
                        let base = &upper[..upper.len() - 4];
                        format!("{}-USDC", base)
                    } else {
                        upper
                    }
                })
                .collect();

            tokio::spawn(okx::run_collection(
                okx_symbols,
                channels,
                writer_tx,
            ))
        }
        "okxswap" | "okxfutures" => {
            // OKX perpetual swap or futures
            let channels = [
                "trades",              // 交易数据
                "bbo-tbt",             // 最优买卖价（tick-by-tick）
                "books",               // 增量深度（400档，每100ms推送变化的档位）
                "books5",              // 5档深度快照（每100ms推送完整的5档数据）
            ]
                .iter()
                .map(|ch| ch.to_string())
                .collect();

            // Transform symbols to OKX SWAP format (e.g., BTCUSDT -> BTC-USDT-SWAP)
            let okx_symbols: Vec<String> = args.symbols
                .iter()
                .map(|s| {
                    let upper = s.to_uppercase();
                    if upper.ends_with("USDT") {
                        let base = &upper[..upper.len() - 4];
                        format!("{}-USDT-SWAP", base)
                    } else if upper.ends_with("USDC") {
                        let base = &upper[..upper.len() - 4];
                        format!("{}-USDC-SWAP", base)
                    } else {
                        upper
                    }
                })
                .collect();

            tokio::spawn(okx::run_collection(
                okx_symbols,
                channels,
                writer_tx,
            ))
        }
        exchange => {
            return Err(anyhow!("{exchange} is not supported."));
        }
    };

    let mut writer = Writer::new(&args.path);
    loop {
        select! {
            _ = signal::ctrl_c() => {
                info!("ctrl-c received");
                break;
            }
            r = writer_rx.recv() => match r {
                Some((recv_time, symbol, data)) => {
                    if let Err(error) = writer.write(recv_time, symbol, data) {
                        error!(?error, "write error");
                        break;
                    }
                }
                None => {
                    break;
                }
            }
        }
    }
    // let _ = handle.await;
    Ok(())
}
