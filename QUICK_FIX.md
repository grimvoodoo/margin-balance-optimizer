# Quick Rate Limit Fix

## The Problem
You're currently making **~52 API calls every 30 seconds** which exceeds Kraken's rate limit of 15 calls burst.

## The Solution  
Three simple changes:

### 1. Batch Ticker Calls (CRITICAL)
**Current**: 25 individual ticker calls
**Fixed**: 3 batched calls

Change line 313 from:
```rust
match client_clone.get_ticker(vec![pair.clone()]).await {
```

To call ticker API with multiple pairs at once:
```rust
// Batch in groups of 10
let batch_size = 10;
for chunk in pairs_vec.chunks(batch_size) {
    match client_clone.get_ticker(chunk.to_vec()).await {
        Ok(tickers) => all_tickers.extend(tickers),
        Err(e) => { /* log error */ }
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
}
```

**Savings**: 25 calls → 3 calls

### 2. Cache OHLC Data (CRITICAL)
**Current**: Fetching 25 OHLC calls every 30 seconds  
**Fixed**: Fetch only every 5 minutes

Add before line 358:
```rust
// Only fetch OHLC every 5 minutes
let should_fetch_ohlc = {
    let mut last = last_ohlc_fetch_clone.lock().unwrap();
    if last.elapsed() > std::time::Duration::from_secs(300) {
        *last = std::time::Instant::now();
        true
    } else {
        false
    }
};

if should_fetch_ohlc {
    // ... existing OHLC fetch code ...
}
```

**Savings**: 25 calls every 30s → 25 calls every 5 minutes

### 3. Cache Asset Pair Discovery (MEDIUM)
**Current**: 1 call every 30 seconds
**Fixed**: 1 call per hour

Wrap line 236-283 with:
```rust
let should_fetch_pairs = {
    let mut last = last_asset_pairs_fetch_clone.lock().unwrap();
    let should_fetch = last.elapsed() > std::time::Duration::from_secs(3600);
    if should_fetch {
        *last = std::time::Instant::now();
    }
    should_fetch || asset_pair_map_clone.lock().unwrap().is_empty()
};

if should_fetch_pairs {
    // ... existing pair discovery code ...
}
```

**Savings**: 1 call every 30s → 1 call per hour

## Result

**Before**: ~52 calls every 30 seconds = **~104 calls/minute** ❌ RATE LIMITED

**After**:  
- Positions: 1 call
- Balance: 1 call  
- Ticker (batched): 3 calls
- Asset pairs: 0 calls (cached)
- OHLC: 0 calls (cached, fetched every 5 min)

**Total**: ~5 calls every 30 seconds = **~10 calls/minute** ✅ SAFE

## Testing

After applying changes:
1. Run the app
2. Check `/tmp/kraken_debug.txt` - should show FAR fewer API calls
3. Monitor for any "Rate limit" errors - should be none

The app will work exactly the same, just with much better rate limit safety!
