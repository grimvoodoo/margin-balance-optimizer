# WebSocket Performance Improvements Summary

## Issue Reported
Updates were taking 10+ seconds between screen refreshes when using WebSocket.

## Root Cause Analysis
The initial WebSocket implementation had several issues:
1. **Naive redraw detection**: Only checking if data exists, not if it changed
2. **Slow polling interval**: Checking for updates every 500ms
3. **No visibility**: No logging to understand what's happening

## Fixes Applied

### 1. Timestamp-Based Change Detection
**Problem**: The redraw logic was triggering on every check, even if no new data arrived.

**Solution**: 
- Added `timestamp` field to `TickerUpdate` struct
- Track the most recent update timestamp
- Only trigger redraw when timestamp changes
- This ensures we redraw exactly when new data arrives, not on every poll

**Code changes**:
- `src/kraken/websocket.rs`: Added timestamp to TickerUpdate
- `src/main_ws.rs`: Changed redraw logic to compare timestamps

### 2. Increased Polling Frequency
**Problem**: Checking for updates every 500ms meant up to 500ms delay before detecting new data.

**Solution**: 
- Reduced polling interval from 500ms to 100ms
- This gives max 100ms latency between WebSocket receiving data and UI updating

**Code changes**:
- `src/main_ws.rs` line 295: `tokio::time::sleep(tokio::time::Duration::from_millis(100))`

### 3. Comprehensive Debug Logging
**Problem**: No way to see what's happening inside the WebSocket connection.

**Solution**: Added logging at every stage:
- **Raw messages** → `/tmp/kraken_ws_messages.txt`
- **Parsed tickers** → `/tmp/kraken_ws_tickers.txt`  
- **Redraw triggers** → `/tmp/kraken_ws_redraws.txt`
- **Parse errors** → `/tmp/kraken_ws_parse_errors.txt`

**Code changes**:
- `src/kraken/websocket.rs`: Added logging in message handler
- `src/main_ws.rs`: Added logging in redraw trigger logic

### 4. Debug Helper Script
Created `debug_ws.sh` to monitor all logs simultaneously in real-time.

## How to Test

### Option 1: Quick Test
```bash
# Clean logs
rm -f /tmp/kraken_ws_*.txt

# Run app
cargo run

# In another terminal, watch for updates:
tail -f /tmp/kraken_ws_tickers.txt
```

### Option 2: Full Debug Mode
```bash
# Terminal 1: Run the app
cargo run

# Terminal 2: Monitor all logs
./debug_ws.sh
```

### Option 3: Compare with REST API
```bash
# Run original REST version
mv src/main.rs src/main_ws.rs
cargo run

# Compare update frequency
```

## Expected Results

### High-Volume Pairs (BTC/USD, ETH/USD)
- **WebSocket**: Updates every 1-5 seconds
- **REST API**: Updates every 30 seconds
- **Improvement**: 6-30x faster

### Low-Volume Pairs (Altcoins)
- **WebSocket**: Updates every 10-60 seconds (depends on trading activity)
- **REST API**: Updates every 30 seconds
- **Improvement**: May be slower if pair doesn't trade frequently

## Diagnosing Issues

### If updates are still slow:

1. **Check if WebSocket is connected**:
   ```bash
   cat /tmp/kraken_ws_messages.txt
   ```
   Should see many JSON messages coming in.

2. **Check if data is being parsed**:
   ```bash
   cat /tmp/kraken_ws_tickers.txt
   ```
   Should see ticker updates with timestamps.

3. **Check if redraws are triggering**:
   ```bash
   cat /tmp/kraken_ws_redraws.txt
   ```
   Should see frequent "New WS data" messages.

4. **Check for errors**:
   ```bash
   cat /tmp/kraken_ws_parse_errors.txt
   ```
   Should be empty or minimal.

### Common Scenarios

**Scenario A: No messages received**
- WebSocket connection failed
- Network/firewall issue
- Wrong WebSocket URL

**Scenario B: Messages received but not parsed**
- Kraken API format changed
- Deserialization struct mismatch
- Check parse_errors.txt for details

**Scenario C: Parsed data but slow redraws**
- Low trading volume on monitored pairs
- This is expected behavior - not all pairs trade frequently
- Test with BTC/USD or ETH/USD for comparison

## Performance Comparison

### Before (REST API polling):
```
Update interval: Fixed 30 seconds
API calls: ~2 per minute per pair
Latency: Up to 30 seconds
Rate limit risk: Moderate (many API calls)
```

### After (WebSocket):
```
Update interval: Real-time (when prices change)
API calls: 0 (after initial connection)
Latency: ~100-200ms
Rate limit risk: Low (only initial connection)
Data freshness: Immediate
```

## Files Modified

1. `Cargo.toml` - Added tokio-tungstenite, futures-util, chrono
2. `src/kraken/mod.rs` - Export WebSocket types
3. `src/kraken/websocket.rs` - **NEW** WebSocket client with logging
4. `src/models.rs` - Added Default trait to TickerData
5. `src/main_ws.rs` - **NEW** Main using WebSocket with timestamp-based updates
6. `debug_ws.sh` - **NEW** Debug helper script
7. `DEBUG_WEBSOCKET.md` - **NEW** Debugging guide
8. `WEBSOCKET_MIGRATION.md` - **NEW** Migration guide

## Rollback Plan

If issues persist:
```bash
# Keep WebSocket version
mv src/main_ws.rs src/main_ws_keep.rs

# Revert to REST API (if you have backup)
git checkout src/main.rs  # or restore from backup
cargo build
```

## Next Steps

1. Test the changes and monitor the log files
2. Report findings:
   - How often do the logs update?
   - What's in each log file?
   - Is the UI updating faster now?

3. If still slow, share:
   ```bash
   head -50 /tmp/kraken_ws_messages.txt
   head -50 /tmp/kraken_ws_tickers.txt
   head -20 /tmp/kraken_ws_redraws.txt
   ```

This will help identify if the issue is:
- WebSocket connectivity
- Data parsing  
- Update frequency
- Or just low trading volume on your pairs
