# Margin Balance Optimizer

A Rust-based terminal UI application for monitoring Kraken cryptocurrency trading positions, balances, and real-time market data.

## Features

- **Real-time Position Tracking**: Monitor open positions with P&L updates every 6 seconds
- **Live Market Data**: Ticker price updates with intelligent batching to minimize API calls
- **Multi-Currency Support**: View balances in GBP, USD, or EUR (toggle with `C` key)
- **Terminal UI**: Clean, responsive interface built with Ratatui
- **Rate Limit Optimized**: Intelligent API call batching and caching to stay within Kraken's limits

## Prerequisites

- Rust 2021 edition or later
- Kraken API credentials (API key and private key)

## Installation

1. Clone the repository:
   ```bash
   git clone git@github.com:grimvoodoo/margin-balance-optimizer.git
   cd margin-balance-optimizer
   ```

2. Create a `.env` file with your Kraken API credentials:
   ```bash
   cp .env.example .env
   # Edit .env with your actual credentials
   ```

3. Your `.env` file should contain:
   ```
   KRAKEN_API_KEY=your_api_key_here
   KRAKEN_PRIVATE_KEY=your_private_key_here
   DISPLAY_CURRENCY=GBP  # Optional: GBP, USD, or EUR
   ```

## Building & Running

```bash
# Build the project
cargo build --release

# Run the application
cargo run --release
```

## Usage

### Keyboard Controls

- **↑/↓**: Navigate through positions
- **C**: Cycle display currency (GBP → USD → EUR → GBP)
- **H**: Show/hide help modal
- **Q**: Quit application
- **Esc**: Close help modal

## Development

### Project Structure

```
margin-balance-optimizer/
├── src/
│   ├── main.rs              # Main application
│   ├── models.rs            # Data structures
│   ├── kraken/              # API client modules
│   └── tui/                 # Terminal UI rendering
├── tests/
│   └── integration_tests.rs # API integration tests
├── docs/
│   ├── TESTING.md           # Testing documentation
│   └── WARP.md              # AI assistant context
├── Cargo.toml               # Project dependencies
├── .env.example             # Environment template
└── README.md                # This file
```

### Common Commands

```bash
# Build in debug mode
cargo build

# Run with optimizations
cargo run --release

# Check code without building
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Clean build artifacts
cargo clean
```

## Testing

The project includes comprehensive unit tests covering:
- Position P&L calculation logic
- Kraken API authentication and signing
- Asset name normalization and mapping
- Currency conversion utilities
- Allocation percentage calculations

Tests cover core business logic and API integration.

```bash
# Run all tests
cargo test

# Run specific test module
cargo test helpers::
cargo test models::
cargo test kraken::

# Run mock API integration tests (no real API calls)
cargo test mock_api_tests
```

## API Usage & Rate Limits

The application is optimized to stay within Kraken's API rate limits.

### Kraken Rate Limit Policy

Kraken enforces a **maximum of 1 request per second** across all API endpoints. Rate limits are tracked using two separate counters:

- **Matching Engine Counter**: Incremented by private order/trade operations
- **REST API Counter**: Incremented by all other REST API calls

Each successful request decrements the counter; exceeding the limit results in temporary rate limiting.

**Reference**: [Kraken API Rate Limits](https://support.kraken.com/articles/206548367-what-are-the-api-rate-limits-)

### Update Frequencies

- **Positions**: Every 6 seconds (fast tracking of P&L changes)
- **Balances**: Every 30 seconds
- **Ticker Prices**: Batched updates every 30 seconds
- **24h Price Changes**: Every 30 seconds
- **Asset Pair Discovery**: Cached for 1 hour

### Rate Limit Safety

The application maintains a conservative request rate:

- **Maximum**: ~0.5 requests/second average (well under 1 req/s limit)
- **Positions API**: 1 call every 6 seconds (0.17 req/s)
- **Balances API**: 1 call every 30 seconds (0.03 req/s)
- **Ticker calls**: Batched in groups of 10 to minimize API usage

## Architecture

The application uses Kraken's REST API with intelligent batching and caching:

- **Batch Processing**: Groups multiple ticker requests into single API calls
- **Smart Caching**: Asset pair discovery cached for 1 hour
- **Optimized Polling**: Different update intervals for different data types
- **Rate Limit Aware**: Built-in delays and request management to stay within limits

## Troubleshooting

### Rate Limit Errors

If you encounter rate limit errors (HTTP 429):

1. Ensure you're not running multiple instances of the application
2. Check that no other applications are using your Kraken API keys
3. Increase the position update interval in `src/main.rs` (default: 6 seconds)
4. The application is already configured to stay well under the 1 req/s limit

**Note**: Rate limits reset gradually, so wait 5-10 seconds before retrying.

### Asset Pair Mapping

The application automatically maps staked/wrapped tokens to their base pairs:
- `ETH.F` (staked ETH) → uses ETH price
- `SOL.F` / `SOL03.S` (staked Solana) → uses SOL price
- `ATOM21.S` (staked Cosmos) → uses ATOM price

## Testing Trading Features

### Important: No Demo API Available

⚠️ **Kraken does not provide a demo/sandbox API for spot trading.**

**Note**: Kraken offers a futures demo (demo-futures.kraken.com), but it uses a completely different API for derivatives trading and is **not compatible** with this spot trading application.

### Safe Testing Approaches

**1. Mock Testing (Recommended for Development)**

Test trading logic without any real API calls:

```bash
cargo test                # All unit tests
cargo test mock_api_tests # API response parsing tests
```

✅ **100% safe** - no network access, no credentials needed, no risk.

**2. Read-Only API Keys (For Integration Testing)**

Create API keys that can only view data, not trade:

1. Go to [Kraken API Settings](https://www.kraken.com/u/security/api)
2. Create new key with **ONLY** these permissions:
   - ✅ Query Funds
   - ✅ Query Open Orders & Trades
   - ✅ Query Closed Orders & Trades
3. **Disable all other permissions** (trading, withdrawals, etc.)
4. Save to `.env.test` (already gitignored)

✅ **Very safe** - physically impossible to trade or withdraw funds.

**3. Small Test Orders (Final Verification Only)**

When you must test actual trading:
- Maximum $1-2 per order
- Test on stablecoins (USDT/USDC) first
- Use limit orders only
- Implement confirmation prompts
- Keep detailed logs

⚠️ **Real money at risk** - only after thorough testing with options 1 & 2.

## Security

⚠️ **Important**: Never commit your `.env` file to version control. It contains sensitive API credentials.

The `.gitignore` file is configured to exclude:
- `.env` (your actual credentials)
- `.env.test` (test credentials)
- `/target` (build artifacts)
- `Cargo.lock`

## License

[Add your license here]

## Contributing

[Add contribution guidelines here]

## Documentation

- [Testing Guide](docs/TESTING.md) - Detailed testing documentation
- [Warp AI Context](docs/WARP.md) - AI assistant development context

## Support

For issues or questions about Kraken's API:
- [Kraken API Documentation](https://docs.kraken.com/rest/)
