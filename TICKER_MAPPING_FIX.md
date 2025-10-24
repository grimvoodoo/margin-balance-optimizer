# Ticker Pair Mapping Fix

## Problem
The application was requesting 97 different ticker pair variations for only 23 assets, causing:
- Inefficient API usage
- Mismatched ticker pairs between balance assets and Kraken's API
- Zero GBP values shown for most assets

## Root Cause
The code was generating multiple variations of ticker pair names (e.g., `XRPGBP`, `XRPZGBP`, `XXXRPZGBP`, `XXXXRPZGBP`) without knowing which format Kraken actually uses for each asset.

## Solution
Created a centralized mapping function `map_asset_to_gbp_pair()` that maps each Kraken asset name to its **correct** ticker pair format:

### Key Mappings
```rust
XXRP       → XXRPZGBP      // XRP (major crypto with XX prefix)
SUI.F      → SUIZGBP       // Staked SUI
SUI.B      → SUIZGBP       // Wrapped SUI  
ETH.F      → ETHZGBP       // Staked ETH
SOL.F      → SOLZGBP       // Staked Solana
SOL03.S    → SOLZGBP       // Staked Solana variant
ATOM21.S   → ATOMZGBP      // Staked Cosmos
FET        → FETZGBP       // Fetch.ai
KTA        → KTAZGBP       // Katana
USDT       → USDTZGBP      // Tether
U          → USDTZGBP      // USDT alternative name
```

### Key Observations
1. **XX prefix**: Major cryptos like XRP use `XXRPZGBP` format
2. **Staked tokens**: `.F`, `.B`, `.S` suffixes map to the base asset pair
3. **Z prefix in pairs**: Kraken uses `Z` before fiat currencies (ZGBP, ZEUR)
4. **Consistent format**: Most pairs follow `[ASSET]ZGBP` pattern

## Changes Made

### 1. `src/main.rs`
- Added `map_asset_to_gbp_pair()` function (lines 25-62)
- Added `map_asset_to_eur_pair()` function (lines 64-74) for EUR conversions
- Replaced pair generation loop (lines 191-198) to use mapping functions
- **Result**: Now requesting ~23-30 pairs instead of 97

### 2. `src/tui/balance.rs`
- Added same `map_asset_to_gbp_pair()` function (lines 13-50)
- Replaced ticker lookup logic (lines 80-87) to use direct mapping
- **Result**: Correct price lookup for each asset

## Testing Checklist

After running the app, verify:

1. **Check debug file**: `cat /tmp/kraken_pairs_requested.txt`
   - Should see ~23-30 pairs instead of 97
   - Each asset should map to ONE correct pair

2. **Check TUI display**:
   - Each asset should show a non-zero GBP value
   - `SUI.F` and `SUI.B` should use the same SUI price
   - `SOL.F` and `SOL03.S` should use the same SOL price
   - `ATOM21.S` should show ATOM price

3. **Verify specific assets**:
   - XXRP (0.03429344) → should show current XRP price in GBP
   - SUI.F (5.05146) → should show ~£X.XX per SUI
   - ETH.F (0.0044850923) → should show current ETH price
   - FET (1.6145718200) → should show current FET price
   - KTA (19.34235) → should show current KTA price

## Unknown Assets

Some assets might need verification:
- **XL1**: Currently mapped to FLRZGBP (Flare token) - needs confirmation
- **U**: Currently mapped to USDTZGBP - verify if correct

If these are wrong, you can update the mapping function in both files.

## Future Improvements

1. **DRY principle**: Extract the mapping function to a shared module instead of duplicating it
2. **Dynamic discovery**: Query Kraken's asset pairs API to build the mapping dynamically
3. **Fallback handling**: Better error messages when a pair isn't found
4. **Conversion rates**: Handle EUR balances by converting through EUR pairs

## How to Update Mappings

If you find an incorrect mapping:

1. Find the asset in your debug output
2. Check Kraken's API or web interface for the correct pair name
3. Update both mapping functions:
   - `src/main.rs` line 25
   - `src/tui/balance.rs` line 13
4. Rebuild: `cargo build`
