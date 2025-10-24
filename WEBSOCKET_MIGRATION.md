# WebSocket Migration Guide

This document describes the refactoring to use Kraken's WebSocket API for real-time data.

## Overview

The application has been refactored to use WebSocket connections for real-time ticker data instead of polling the REST API every 30 seconds. This provides:

- **Lower latency**: Ticker prices update in real-time as they change
- **Reduced API calls**: No need to repeatedly poll for ticker data
- **Better performance**: Less bandwidth and fewer rate limit concerns
- **Instant updates**: Price changes appear immediately in the UI

## Architecture Changes

### Hybrid Approach

The new implementation uses a hybrid approach:

1. **WebSocket** for real-time ticker prices (continuous updates)
2. **REST API** for positions and balances (updated every 60 seconds)

This is optimal because:
- Ticker prices change frequently and benefit from real-time updates
- Positions and balances change infrequently and don't need WebSocket overhead

### New Components

#### `src/kraken/websocket.rs`

Contains three main structures:

- **`KrakenWebSocket`**: Core WebSocket client that connects to Kraken and handles incoming messages
- **`TickerUpdate`**: Simplified data structure for real-time ticker updates
- **`WebSocketManager`**: Manages WebSocket lifecycle and subscription updates

#### `src/main_ws.rs`

New main entry point using WebSocket for ticker data. Key features:

- Connects to Kraken WebSocket API on startup
- Automatically subscribes to relevant trading pairs
- Updates subscriptions when new pairs are discovered
- Maintains backward compatibility with existing TUI rendering

## Files Changed

- `Cargo.toml`: Added `tokio-tungstenite` and `futures-util` dependencies
- `src/models.rs`: Added `Default` trait to `TickerData` for easier construction
- `src/kraken/mod.rs`: Exports new WebSocket types
- `src/kraken/websocket.rs`: **NEW** - WebSocket client implementation
- `src/main_ws.rs`: **NEW** - Refactored main using WebSocket

## Migration Path

### Testing the WebSocket Version

1. Build and run with the new main:
   ```bash
   cargo build
   cargo run --bin margin-balance-optimizer-ws
   ```

2. Observe real-time price updates in the TUI

### Switching Completely

To make WebSocket the default:

1. Backup the original:
   ```bash
   mv src/main.rs src/main_old.rs
   ```

2. Replace with WebSocket version:
   ```bash
   mv src/main_ws.rs src/main.rs
   ```

3. Update `Cargo.toml` if you created a separate binary:
   ```toml
   [[bin]]
   name = "margin-balance-optimizer"
   path = "src/main.rs"
   ```

## WebSocket API Details

### Kraken WebSocket v2

- **Endpoint**: `wss://ws.kraken.com/v2`
- **Channel**: `ticker` for price updates
- **Subscription format**:
  ```json
  {
    "method": "subscribe",
    "params": {
      "channel": "ticker",
      "symbol": ["BTC/USD", "ETH/USD"],
      "snapshot": true
    }
  }
  ```

### Data Format

Ticker updates include:
- `symbol`: Trading pair (e.g., "BTC/USD")
- `last`: Last trade price
- `volume`: 24h volume
- `change`: 24h percentage change
- `vwap`, `high`, `low`: Additional metrics

## Benefits

1. **Real-time Updates**: Prices update instantly as trades occur
2. **Reduced Polling**: From 30s REST API polls to continuous WebSocket
3. **Lower Latency**: ~100-500ms vs 30+ seconds for updates
4. **Better UX**: Users see live market movements
5. **Scalability**: Less load on Kraken's API servers

## Considerations

- **Connection Management**: WebSocket reconnects automatically if disconnected
- **Subscription Updates**: When new pairs are discovered, WebSocket reconnects with updated subscriptions
- **Fallback**: Can always revert to `src/main_old.rs` for REST-only approach
- **Rate Limits**: WebSocket has different limits than REST API (generally more permissive)

## Testing

1. Monitor connection status in the loading stage message
2. Verify real-time price updates (should be instant)
3. Check for WebSocket errors in `/tmp/kraken_*.txt` debug files
4. Test currency switching (C key) to ensure conversion rates work

## Future Enhancements

Possible improvements:

1. **Private WebSocket**: Use authenticated WebSocket for position updates
2. **Depth Data**: Subscribe to order book depth for better market insight
3. **Trade History**: Real-time trade stream for your own trades
4. **Reconnection Logic**: More sophisticated exponential backoff
5. **Multiple Connections**: Separate connections for different data types

## Rollback

If issues occur, simply revert to the original:

```bash
mv src/main.rs src/main_ws.rs
mv src/main_old.rs src/main.rs
cargo build
```

The old polling approach in `src/main_old.rs` remains fully functional.
