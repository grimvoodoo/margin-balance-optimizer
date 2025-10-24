# Kraken API Rate Limit Optimization

## Current API Usage

### Per Update Cycle (every 30 seconds):

1. **OpenPositions** (Private): 1 call
2. **Balance** (Private): 1 call  
3. **AssetPairs** (Public): 1 call (for pair discovery)
4. **Ticker** (Public): ~25 calls (one per pair, with 100ms delay every 10 pairs)
5. **OHLC** (Public): ~25 calls (one per pair, with 200ms delay between each)

**Total per cycle**: ~52 API calls
**Per minute**: ~104 API calls

## Kraken Rate Limits

### Public API (Tier 1 - Starter):
- **Burst**: 15 calls
- **Decay rate**: +1 call every 3 seconds
- **Maximum**: 20 calls in counter

### Private API (Tier 1 - Starter):
- **Burst**: 15 calls
- **Decay rate**: +1 call every 3 seconds  
- **Maximum**: 20 calls in counter

## Current Status: ⚠️ EXCEEDING LIMITS

With ~25 pairs, we're making:
- **Ticker calls**: 25 calls in ~2.5 seconds (10 calls, wait 100ms, 10 calls, wait 100ms, 5 calls)
- **OHLC calls**: 25 calls in ~5 seconds (200ms between each)

This exceeds the burst limit of 15 calls.

## Optimization Strategy

### 1. Batch Ticker Requests ✅
Instead of 25 individual calls, batch them into groups:
```rust
// Current: 25 calls
for pair in pairs { get_ticker(vec![pair]) }

// Optimized: 3-4 calls
get_ticker(pairs[0..10])   // 1 call
get_ticker(pairs[10..20])  // 1 call  
get_ticker(pairs[20..])    // 1 call
```

**Savings**: 25 calls → 3 calls

### 2. Cache OHLC Data ✅
24h changes don't need to be fetched every 30 seconds:
- Fetch OHLC only every 5-10 minutes
- Cache the values and reuse them

**Savings**: 25 calls per 30s → 25 calls per 5 minutes

### 3. Skip AssetPairs After First Fetch ✅
Asset pairs don't change frequently:
- Fetch once on startup
- Cache the mapping
- Only re-fetch if new assets appear

**Savings**: 1 call per 30s → 1 call per hour

### 4. Increase Update Interval (Optional)
Consider updating every 45-60 seconds instead of 30:
- Most crypto prices don't change significantly in 30s
- Positions/balances change even less frequently

**Savings**: 50% reduction in all calls

## Optimized API Usage

### After Optimization:

**Every 30 seconds**:
- OpenPositions: 1 call
- Balance: 1 call
- Ticker (batched): 3 calls
- **Total: 5 calls every 30 seconds = 10 calls/minute**

**Every 5 minutes**:
- OHLC: 25 calls (spread over 5 seconds)

**Once per hour**:
- AssetPairs: 1 call

### Rate Limit Analysis:

**Peak usage**: 5 calls in ~0.5 seconds (well under 15 burst limit)
**Average**: 10-15 calls per minute (well under decay rate)

## Implementation Priority

### High Priority (Immediate):
1. ✅ Batch ticker requests
2. ✅ Cache OHLC data (fetch every 5 min instead of 30s)

### Medium Priority:
3. ✅ Cache asset pairs mapping
4. Add exponential backoff on errors

### Low Priority (Optional):
5. Increase update interval to 45-60s
6. Add rate limit counter tracking

## Implementation Code

See the updated `src/main.rs` with:
- Batched ticker API calls
- Cached OHLC with 5-minute refresh
- Cached asset pair discovery

## Monitoring

Check `/tmp/kraken_debug.txt` for:
- Number of API calls per cycle
- Failed requests (may indicate rate limiting)
- Timing of requests

If you see errors like "Rate limit exceeded", increase delays or update intervals.
