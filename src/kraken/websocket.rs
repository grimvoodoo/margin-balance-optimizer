use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const KRAKEN_WS_URL: &str = "wss://ws.kraken.com/v2";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickerUpdate {
    pub pair: String,
    pub price: f64,
    pub volume: f64,
    pub change_24h: f64,
    pub timestamp: u64, // Unix timestamp in milliseconds
}

#[derive(Debug, Serialize)]
struct SubscribeMessage {
    method: String,
    params: SubscribeParams,
}

#[derive(Debug, Serialize)]
struct SubscribeParams {
    channel: String,
    symbol: Vec<String>,
    snapshot: bool,
}

#[derive(Debug, Deserialize)]
struct WsResponse {
    #[serde(default)]
    channel: String,
    #[serde(rename = "type", default)]
    msg_type: String,
    #[serde(default)]
    data: Vec<TickerData>,
}

#[derive(Debug, Deserialize, Clone)]
struct TickerData {
    symbol: String,
    last: f64,
    volume: f64,
    #[serde(rename = "change", default)]
    change_pct: f64,
    #[serde(default)]
    vwap: f64,
    #[serde(default)]
    high: f64,
    #[serde(default)]
    low: f64,
}

pub struct KrakenWebSocket {
    ticker_data: Arc<Mutex<HashMap<String, TickerUpdate>>>,
}

impl KrakenWebSocket {
    pub fn new() -> Self {
        Self {
            ticker_data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn connect_and_subscribe(&self, pairs: Vec<String>) -> Result<()> {
        // Debug: Log connection attempt
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/kraken_ws_connection.txt")
        {
            let _ = writeln!(
                file,
                "\n[{}] Attempting to connect to {} with {} pairs",
                chrono::Utc::now().format("%H:%M:%S%.3f"),
                KRAKEN_WS_URL,
                pairs.len()
            );
            let _ = writeln!(file, "Pairs: {:?}", pairs);
        }

        let (ws_stream, _) = connect_async(KRAKEN_WS_URL)
            .await
            .context("Failed to connect to Kraken WebSocket")?;

        // Debug: Log successful connection
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/kraken_ws_connection.txt")
        {
            let _ = writeln!(
                file,
                "[{}] ✓ Connected successfully!",
                chrono::Utc::now().format("%H:%M:%S%.3f")
            );
        }

        let (mut write, mut read) = ws_stream.split();

        // Subscribe to ticker channel for specified pairs
        let subscribe_msg = SubscribeMessage {
            method: "subscribe".to_string(),
            params: SubscribeParams {
                channel: "ticker".to_string(),
                symbol: pairs.clone(),
                snapshot: true,
            },
        };

        let msg = serde_json::to_string(&subscribe_msg)?;
        write.send(Message::Text(msg)).await?;

        // Spawn task to handle incoming messages
        let ticker_data = Arc::clone(&self.ticker_data);

        tokio::spawn(async move {
            while let Some(msg_result) = read.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        // Debug: Log all incoming messages
                        use std::io::Write;
                        if let Ok(mut file) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("/tmp/kraken_ws_messages.txt")
                        {
                            let _ = writeln!(
                                file,
                                "[{}] Received: {}",
                                chrono::Utc::now().format("%H:%M:%S%.3f"),
                                &text[..text.len().min(200)]
                            );
                        }

                        if let Ok(response) = serde_json::from_str::<WsResponse>(&text) {
                            if response.channel == "ticker" && !response.data.is_empty() {
                                let mut data = ticker_data.lock().await;

                                for ticker in response.data {
                                    let timestamp = SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap()
                                        .as_millis()
                                        as u64;

                                    let update = TickerUpdate {
                                        pair: ticker.symbol.clone(),
                                        price: ticker.last,
                                        volume: ticker.volume,
                                        change_24h: ticker.change_pct,
                                        timestamp,
                                    };
                                    data.insert(ticker.symbol.clone(), update.clone());

                                    // Debug: Log ticker updates
                                    if let Ok(mut file) = std::fs::OpenOptions::new()
                                        .create(true)
                                        .append(true)
                                        .open("/tmp/kraken_ws_tickers.txt")
                                    {
                                        let _ = writeln!(
                                            file,
                                            "[{}] {} = {} (change: {:.2}%)",
                                            chrono::Utc::now().format("%H:%M:%S%.3f"),
                                            ticker.symbol,
                                            ticker.last,
                                            ticker.change_pct
                                        );
                                    }
                                }
                            }
                        } else if let Ok(mut file) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("/tmp/kraken_ws_parse_errors.txt")
                        {
                            let _ = writeln!(
                                file,
                                "[{}] Failed to parse: {}",
                                chrono::Utc::now().format("%H:%M:%S%.3f"),
                                &text[..text.len().min(500)]
                            );
                        }
                    }
                    Ok(Message::Ping(payload)) => {
                        // Respond to pings to keep connection alive
                        let _ = write.send(Message::Pong(payload)).await;
                    }
                    Ok(Message::Close(_)) => {
                        break;
                    }
                    Err(e) => {
                        eprintln!("WebSocket error: {:?}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    pub async fn get_ticker_data(&self) -> HashMap<String, TickerUpdate> {
        self.ticker_data.lock().await.clone()
    }

    pub async fn get_last_update_time(&self) -> Option<u64> {
        self.ticker_data
            .lock()
            .await
            .values()
            .map(|t| t.timestamp)
            .max()
    }

    /// Subscribe to additional pairs dynamically
    pub async fn subscribe_to_pairs(&self, pairs: Vec<String>) -> Result<()> {
        // This would require maintaining the write half of the websocket
        // For now, reconnection is needed to change subscriptions
        Ok(())
    }
}

/// Manager to handle WebSocket reconnections and subscription updates
pub struct WebSocketManager {
    ws: Arc<Mutex<Option<KrakenWebSocket>>>,
    current_pairs: Arc<Mutex<Vec<String>>>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        Self {
            ws: Arc::new(Mutex::new(None)),
            current_pairs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn connect_with_pairs(&self, pairs: Vec<String>) -> Result<()> {
        let ws = KrakenWebSocket::new();
        ws.connect_and_subscribe(pairs.clone()).await?;

        *self.ws.lock().await = Some(ws);
        *self.current_pairs.lock().await = pairs;

        Ok(())
    }

    pub async fn update_subscriptions(&self, new_pairs: Vec<String>) -> Result<()> {
        let current = self.current_pairs.lock().await.clone();

        // Only reconnect if pairs have changed
        if current != new_pairs {
            let ws = KrakenWebSocket::new();
            ws.connect_and_subscribe(new_pairs.clone()).await?;

            *self.ws.lock().await = Some(ws);
            *self.current_pairs.lock().await = new_pairs;
        }

        Ok(())
    }

    pub async fn get_ticker_data(&self) -> HashMap<String, TickerUpdate> {
        if let Some(ws) = self.ws.lock().await.as_ref() {
            ws.get_ticker_data().await
        } else {
            HashMap::new()
        }
    }

    pub async fn get_last_update_time(&self) -> Option<u64> {
        if let Some(ws) = self.ws.lock().await.as_ref() {
            ws.get_last_update_time().await
        } else {
            None
        }
    }
}
