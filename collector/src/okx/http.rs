use std::{
    io,
    io::ErrorKind,
    time::{Duration, Instant},
};

use anyhow::Error;
use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use tokio::{
    select,
    sync::mpsc::{UnboundedSender, unbounded_channel},
    time::interval,
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Bytes, Message, Utf8Bytes, client::IntoClientRequest},
};
use tracing::{error, warn};

/// Fetch list of trading instruments from OKX
pub async fn fetch_symbol_list(inst_type: &str) -> Result<Vec<String>, reqwest::Error> {
    let url = format!("https://www.okx.com/api/v5/public/instruments?instType={}", inst_type);
    
    Ok(reqwest::Client::new()
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?
        .get("data")
        .and_then(|data| data.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|inst| {
                    inst.get("instId")
                        .and_then(|id| id.as_str())
                        .map(|s| s.to_string())
                })
                .collect()
        })
        .unwrap_or_default())
}

/// Fetch orderbook snapshot from OKX REST API
pub async fn fetch_depth_snapshot(inst_id: &str) -> Result<String, reqwest::Error> {
    let url = format!(
        "https://www.okx.com/api/v5/market/books?instId={}&sz=400",
        inst_id
    );
    
    reqwest::Client::new()
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?
        .text()
        .await
}

pub async fn connect(
    url: &str,
    ws_tx: UnboundedSender<(DateTime<Utc>, Utf8Bytes)>,
) -> Result<(), anyhow::Error> {
    let request = url.into_client_request()?;
    let (ws_stream, _) = connect_async(request).await?;
    let (mut write, mut read) = ws_stream.split();
    let (tx, mut rx) = unbounded_channel::<Bytes>();

    tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            if write.send(Message::Pong(data)).await.is_err() {
                let _ = write.close().await;
                return;
            }
        }
    });

    let mut last_ping = Instant::now();
    let mut checker = interval(Duration::from_secs(5));

    loop {
        select! {
            msg = read.next() => match msg {
                Some(Ok(Message::Text(text))) => {
                    let recv_time = Utc::now();
                    if ws_tx.send((recv_time, text)).is_err() {
                        break;
                    }
                }
                Some(Ok(Message::Binary(_))) => {}
                Some(Ok(Message::Ping(data))) => {
                    if tx.send(data).is_err() {
                        return Err(Error::from(io::Error::new(
                            ErrorKind::ConnectionAborted,
                            "closed",
                        )));
                    }
                    last_ping = Instant::now();
                }
                Some(Ok(Message::Pong(_))) => {
                    last_ping = Instant::now();
                }
                Some(Ok(Message::Close(close_frame))) => {
                    warn!(?close_frame, "closed");
                    return Err(Error::from(io::Error::new(
                        ErrorKind::ConnectionAborted,
                        "closed",
                    )));
                }
                Some(Ok(Message::Frame(_))) => {}
                Some(Err(e)) => {
                    return Err(Error::from(e));
                }
                None => {
                    break;
                }
            },
            _ = checker.tick() => {
                if last_ping.elapsed() > Duration::from_secs(30) {
                    warn!("Ping timeout.");
                    return Err(Error::from(io::Error::new(
                        ErrorKind::TimedOut,
                        "Ping",
                    )));
                }
            }
        }
    }
    Ok(())
}

/// Maintain WebSocket connection to OKX, auto-reconnect on failure
pub async fn keep_connection(
    channels: Vec<String>,
    inst_ids: Vec<String>,
    ws_tx: UnboundedSender<(DateTime<Utc>, Utf8Bytes)>,
) {
    let mut error_count = 0;
    loop {
        let connect_time = Instant::now();
        
        // OKX uses public WebSocket endpoint
        let url = "wss://ws.okx.com:8443/ws/v5/public";
        
        if let Err(error) = connect_with_subscription(url, &channels, &inst_ids, ws_tx.clone()).await {
            error!(?error, "websocket error");
            error_count += 1;
            
            if connect_time.elapsed() > Duration::from_secs(30) {
                error_count = 0;
            }
            
            // Exponential backoff
            if error_count > 20 {
                tokio::time::sleep(Duration::from_secs(10)).await;
            } else if error_count > 10 {
                tokio::time::sleep(Duration::from_secs(5)).await;
            } else if error_count > 3 {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        } else {
            break;
        }
    }
}

async fn connect_with_subscription(
    url: &str,
    channels: &[String],
    inst_ids: &[String],
    ws_tx: UnboundedSender<(DateTime<Utc>, Utf8Bytes)>,
) -> Result<(), anyhow::Error> {
    let request = url.into_client_request()?;
    let (ws_stream, _) = connect_async(request).await?;
    let (mut write, mut read) = ws_stream.split();
    
    // Subscribe to channels after connection
    let args: Vec<serde_json::Value> = inst_ids
        .iter()
        .flat_map(|inst_id| {
            channels.iter().map(move |channel| {
                serde_json::json!({
                    "channel": channel,
                    "instId": inst_id
                })
            })
        })
        .collect();
    
    let subscribe_msg = serde_json::json!({
        "op": "subscribe",
        "args": args
    });
    
    write.send(Message::Text(subscribe_msg.to_string().into())).await?;
    
    let (tx, mut rx) = unbounded_channel::<Bytes>();
    
    tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            if write.send(Message::Pong(data)).await.is_err() {
                let _ = write.close().await;
                return;
            }
        }
    });
    
    let mut last_ping = Instant::now();
    let mut checker = interval(Duration::from_secs(5));
    
    loop {
        select! {
            msg = read.next() => match msg {
                Some(Ok(Message::Text(text))) => {
                    let recv_time = Utc::now();
                    if ws_tx.send((recv_time, text)).is_err() {
                        break;
                    }
                }
                Some(Ok(Message::Binary(_))) => {}
                Some(Ok(Message::Ping(data))) => {
                    if tx.send(data).is_err() {
                        return Err(Error::from(io::Error::new(
                            ErrorKind::ConnectionAborted,
                            "closed",
                        )));
                    }
                    last_ping = Instant::now();
                }
                Some(Ok(Message::Pong(_))) => {
                    last_ping = Instant::now();
                }
                Some(Ok(Message::Close(close_frame))) => {
                    warn!(?close_frame, "closed");
                    return Err(Error::from(io::Error::new(
                        ErrorKind::ConnectionAborted,
                        "closed",
                    )));
                }
                Some(Ok(Message::Frame(_))) => {}
                Some(Err(e)) => {
                    return Err(Error::from(e));
                }
                None => {
                    break;
                }
            },
            _ = checker.tick() => {
                if last_ping.elapsed() > Duration::from_secs(30) {
                    warn!("Ping timeout.");
                    return Err(Error::from(io::Error::new(
                        ErrorKind::TimedOut,
                        "Ping",
                    )));
                }
            }
        }
    }
    Ok(())
}
