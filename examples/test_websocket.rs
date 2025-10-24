use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;

#[tokio::main]
async fn main() {
    println!("Testing Kraken WebSocket connection...");
    
    let url = "wss://ws.kraken.com/v2";
    println!("Connecting to: {}", url);
    
    match connect_async(url).await {
        Ok((ws_stream, response)) => {
            println!("✓ Connected successfully!");
            println!("Response: {:?}", response);
            
            let (mut write, mut read) = ws_stream.split();
            
            // Subscribe to BTC/USD ticker
            let subscribe = json!({
                "method": "subscribe",
                "params": {
                    "channel": "ticker",
                    "symbol": ["BTC/USD"],
                    "snapshot": true
                }
            });
            
            println!("\nSending subscription: {}", subscribe);
            write.send(Message::Text(subscribe.to_string())).await.unwrap();
            
            println!("\nWaiting for messages (will show first 5)...\n");
            
            let mut count = 0;
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        count += 1;
                        println!("Message {}: {}", count, &text[..text.len().min(200)]);
                        
                        if count >= 5 {
                            println!("\n✓ WebSocket is working! Received {} messages", count);
                            break;
                        }
                    }
                    Ok(Message::Ping(_)) => {
                        println!("Received ping");
                    }
                    Ok(Message::Close(_)) => {
                        println!("Connection closed");
                        break;
                    }
                    Err(e) => {
                        println!("Error: {:?}", e);
                        break;
                    }
                    _ => {}
                }
            }
            
            if count == 0 {
                println!("\n✗ No messages received - check subscription format or pair name");
            }
        }
        Err(e) => {
            println!("✗ Connection failed: {:?}", e);
            println!("\nPossible issues:");
            println!("  - Network/firewall blocking WebSocket");
            println!("  - Invalid WebSocket URL");
            println!("  - TLS/SSL certificate issues");
        }
    }
}
