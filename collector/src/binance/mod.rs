mod http;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
pub use http::{fetch_depth_snapshot, keep_connection};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use tokio_tungstenite::tungstenite::Utf8Bytes;
use tracing::{error, warn};

use crate::{error::ConnectorError, throttler::Throttler};

fn handle(
    prev_u_map: &mut HashMap<String, i64>,
    writer_tx: &UnboundedSender<(DateTime<Utc>, String, String)>,
    recv_time: DateTime<Utc>,
    data: Utf8Bytes,
    throttler: &Throttler,
) -> Result<(), ConnectorError> {
    let j: serde_json::Value = serde_json::from_str(data.as_str())?;
    if let Some(j_data) = j.get("data") {
        let j_data_obj = j_data.as_object().ok_or(ConnectorError::FormatError)?;
        
        // Try to get symbol from data.s field, or extract from stream name
        let symbol: String = if let Some(j_symbol) = j_data_obj.get("s") {
            j_symbol.as_str().ok_or(ConnectorError::FormatError)?.to_string()
        } else {
            // For streams without 's' field (e.g., depth20@100ms), extract symbol from stream name
            if let Some(stream) = j.get("stream") {
                let stream_str = stream.as_str().ok_or(ConnectorError::FormatError)?;
                // Extract symbol from "btcusdt@depth20@100ms" -> "BTCUSDT"
                stream_str
                    .split('@')
                    .next()
                    .ok_or(ConnectorError::FormatError)?
                    .to_uppercase()
            } else {
                return Err(ConnectorError::FormatError);
            }
        };
        
        if let Some(e) = j_data.get("e") {
            let ev = e.as_str().ok_or(ConnectorError::FormatError)?;
            if ev == "depthUpdate" {
                let u = j_data
                    .get("u")
                    .ok_or(ConnectorError::FormatError)?
                    .as_i64()
                    .ok_or(ConnectorError::FormatError)?;
                #[allow(non_snake_case)]
                let U = j_data
                    .get("U")
                    .ok_or(ConnectorError::FormatError)?
                    .as_i64()
                    .ok_or(ConnectorError::FormatError)?;
                let prev_u = prev_u_map.get(&symbol);
                // For Binance Spot, check if U <= prev_u + 1 <= u to ensure continuity
                // This is different from futures which has a 'pu' field
                if let Some(&prev_u_val) = prev_u {
                    if U > prev_u_val + 1 {
                        warn!(%symbol, prev_u=prev_u_val, U, u, "missing depth feed has been detected.");
                        let symbol_ = symbol.clone();
                        let writer_tx_ = writer_tx.clone();
                        let mut throttler_ = throttler.clone();
                        tokio::spawn(async move {
                            match throttler_.execute(fetch_depth_snapshot(&symbol_)).await {
                                Some(Ok(data)) => {
                                    let recv_time = Utc::now();
                                    let _ = writer_tx_.send((recv_time, symbol_, data));
                                }
                                Some(Err(error)) => {
                                    error!(
                                        symbol = symbol_,
                                        ?error,
                                        "couldn't fetch the depth snapshot."
                                    );
                                }
                                None => {
                                    warn!(
                                        symbol = symbol_,
                                        "Fetching the depth snapshot is rate-limited."
                                    )
                                }
                            }
                        });
                    }
                }
                *prev_u_map.entry(symbol.clone()).or_insert(0) = u;
            }
        }
        let _ = writer_tx.send((recv_time, symbol, data.to_string()));
    }
    Ok(())
}

pub async fn run_collection(
    streams: Vec<String>,
    symbols: Vec<String>,
    writer_tx: UnboundedSender<(DateTime<Utc>, String, String)>,
) -> Result<(), anyhow::Error> {
    let mut prev_u_map = HashMap::new();
    let (ws_tx, mut ws_rx) = unbounded_channel();
    let h = tokio::spawn(keep_connection(streams, symbols, ws_tx.clone()));
    // todo: check the Spot API rate limits.
    // https://www.binance.com/en/support/faq/rate-limits-on-binance-futures-281596e222414cdd9051664ea621cdc3
    // The default rate limit per IP is 2,400/min and the weight is 20 at a depth of 1000.
    // The maximum request rate for fetching snapshots is 120 per minute.
    // Sets the rate limit with a margin to account for connection requests.
    let throttler = Throttler::new(100);
    while let Some((recv_time, data)) = ws_rx.recv().await {
        if let Err(error) = handle(&mut prev_u_map, &writer_tx, recv_time, data, &throttler) {
            error!(?error, "couldn't handle the received data.");
        }
    }
    let _ = h.await;
    Ok(())
}
