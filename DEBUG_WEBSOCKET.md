# WebSocket Debugging Guide

## The Problem

You reported that updates are taking 10+ seconds between screen refreshes, which shouldn't happen with WebSocket.

## What I've Fixed

### 1. **Added Timestamp Tracking**
   - Every WebSocket message now includes a timestamp
   - Redraw logic now checks if data has actually changed (based on timestamp)
   - This ensures we only redraw when new data arrives

### 2. **Improved Redraw Frequency**
   - Changed from checking every 500ms to every 100ms
   - More responsive to incoming WebSocket messages

### 3. **Added Comprehensive Debug Logging**
   - `/tmp/kraken_ws_messages.txt` - Raw WebSocket messages received
   - `/tmp/kraken_ws_tickers.txt` - Parsed ticker updates with timestamps
   - `/tmp/kraken_ws_redraws.txt` - When redraws are triggered
   - `/tmp/kraken_ws_parse_errors.txt` - Any parsing errors

## How to Debug

### Step 1: Clean Logs and Run
```bash
# Clean old logs
rm -f /tmp/kraken_ws_*.txt

# Run the app
cargo run --bin margin-balance-optimizer
```

### Step 2: Monitor Logs (in another terminal)
```bash
# Watch all logs at once
./debug_ws.sh

# Or watch individual logs:
tail -f /tmp/kraken_ws_messages.txt    # Raw messages
tail -f /tmp/kraken_ws_tickers.txt     # Ticker updates
tail -f /tmp/kraken_ws_redraws.txt     # Redraw triggers
```

### Step 3: Analyze What's Happening

#### Check if WebSocket is Receiving Data
```bash
cat /tmp/kraken_ws_messages.txt
```

If this file is **empty** or has very few entries:
- WebSocket connection may not be established
- Subscription may have failed
- Wrong pair format being used

#### Check if Data is Being Parsed
```bash
cat /tmp/kraken_ws_tickers.txt
```

If messages are received but this file is **empty**:
- Check `/tmp/kraken_ws_parse_errors.txt` for parsing issues
- The WebSocket API response format may be different than expected

#### Check if Redraws are Triggered
```bash
cat /tmp/kraken_ws_redraws.txt
```

If tickers are updating but redraws are **infrequent**:
- The timestamp logic may not be working correctly
- Multiple updates might have the same timestamp

## Common Issues & Solutions

### Issue 1: No WebSocket Messages Received

**Symptoms:**
- `/tmp/kraken_ws_messages.txt` is empty
- Loading message stuck on "Connecting to WebSocket..."

**Solutions:**
1. Check network connectivity
2. Verify firewall isn't blocking WebSocket connections
3. Try testing WebSocket manually:
   ```bash
   wscat -c wss://ws.kraken.com/v2
   ```

### Issue 2: Messages Received but Not Parsed

**Symptoms:**
- Messages in `/tmp/kraken_ws_messages.txt`
- Errors in `/tmp/kraken_ws_parse_errors.txt`
- No updates in `/tmp/kraken_ws_tickers.txt`

**Solutions:**
1. Check the actual JSON format in the messages file
2. The Kraken v2 API format may differ from expected
3. May need to update the deserialization structures

### Issue 3: Updates Too Slow

**Symptoms:**
- Tickers updating but 10+ seconds between screen updates
- Few entries in `/tmp/kraken_ws_redraws.txt`

**Possible causes:**
1. **Timestamp collision**: Multiple updates have same millisecond timestamp
   - Solution: Use a counter instead of just timestamp
2. **Lock contention**: Mutex locks blocking the update check
   - Solution: Reduce lock duration or use different synchronization
3. **Kraken sends updates slowly**: Some pairs don't trade frequently
   - Solution: This is expected for low-volume pairs

## Testing with High-Frequency Pairs

To verify WebSocket is working correctly, test with very active pairs:

```bash
# Edit the pair subscription in src/main_ws.rs to include:
pairs.insert("BTC/USD".to_string());
pairs.insert("ETH/USD".to_string());
```

These pairs should update multiple times per second.

## Expected Behavior

With WebSocket working correctly:
- BTC/USD, ETH/USD: Updates every few seconds (high activity)
- Low-volume pairs: Updates may be 10-30+ seconds apart
- UI should update immediately when any pair changes

## Next Steps

1. Run the app and immediately check the log files
2. Share the contents of the log files if issues persist:
   ```bash
   head -20 /tmp/kraken_ws_messages.txt
   head -20 /tmp/kraken_ws_tickers.txt
   head -20 /tmp/kraken_ws_redraws.txt
   ```

3. Note the time between entries - this will show if:
   - WebSocket is receiving updates
   - UI is redrawing appropriately
   - The issue is with Kraken's update frequency vs our code

## Reverting to REST API

If WebSocket continues to have issues, you can revert:
```bash
mv src/main.rs src/main_ws_broken.rs
mv src/main_old.rs src/main.rs  # If you made a backup
cargo build
```

Or just use the original REST polling which works but is slower.
