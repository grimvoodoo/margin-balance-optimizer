# Testing Documentation

## Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific module
cargo test helpers::
cargo test models::
cargo test kraken::

# Run mock API tests (no real API calls)
cargo test mock_api_tests
```

## Test Coverage

**Total: 25 tests** (23 unit + 2 integration)
- ✅ All passing
- 🔍 Core business logic covered

## Test Breakdown by Module

### 1. Models Module (`src/models.rs`)

Tests for the `Position::calculate_unrealized_pnl()` function:

- **`test_calculate_unrealized_pnl_with_net`**: Verifies correct P&L calculation when `net` field is provided by Kraken
- **`test_calculate_unrealized_pnl_without_net`**: Tests fallback calculation using value - cost - fee
- **`test_calculate_unrealized_pnl_negative`**: Ensures negative P&L is handled correctly
- **`test_calculate_unrealized_pnl_missing_value`**: Handles missing value field gracefully
- **`test_calculate_unrealized_pnl_invalid_strings`**: Validates error handling for invalid numeric strings
- **`test_calculate_unrealized_pnl_zero_values`**: Edge case testing with zero values

**Coverage**: 6 tests covering all code paths in P&L calculation

### 2. Kraken Client Module (`src/kraken/client.rs`)

Tests for API client construction and cryptographic signing:

- **`test_new_kraken_client_valid_key`**: Validates client creation with valid base64 private key
- **`test_new_kraken_client_invalid_base64`**: Ensures invalid keys are rejected
- **`test_get_nonce`**: Verifies nonce generation is strictly increasing
- **`test_sign_request`**: Tests request signing produces consistent, non-empty signatures
- **`test_sign_request_different_endpoints`**: Confirms different endpoints produce different signatures

**Coverage**: 5 tests covering authentication and security functions

### 3. Helpers Module (`src/helpers.rs`)

Tests for utility functions used throughout the application:

#### Asset Mapping (4 tests)
- **`test_map_asset_to_gbp_pair_major_cryptos`**: BTC, ETH, XRP mapping
- **`test_map_asset_to_gbp_pair_staked_tokens`**: Staked/wrapped token mapping
- **`test_map_asset_to_gbp_pair_stablecoins`**: USDT and stablecoin mapping
- **`test_map_asset_to_gbp_pair_default_format`**: Unknown assets get default format

#### Asset Name Normalization (3 tests)
- **`test_normalize_asset_name_special_cases`**: Special asset name conversions
- **`test_normalize_asset_name_staked_suffixes`**: Removing .F, .B, .S suffixes
- **`test_normalize_asset_name_prefixes`**: Removing X/Z prefixes

#### Currency Functions (1 test)
- **`test_is_fiat_currency`**: Identifies GBP, EUR, USD and their variants
- **`test_get_currency_symbol`**: Returns correct symbols (£, €, $)

#### Calculations (3 tests)
- **`test_calculate_allocation_percent`**: Basic percentage calculation
- **`test_calculate_allocation_percent_zero_total`**: Division by zero safety
- **`test_calculate_allocation_percent_edge_cases`**: Edge cases and small values

**Coverage**: 12 tests covering all utility functions

### 4. Integration Tests (`tests/integration_tests.rs`)

Mock-based tests for API response parsing:

- **`test_parse_ticker_response`**: Validates ticker JSON parsing
- **`test_parse_position_response`**: Validates position JSON parsing

**Coverage**: 2 tests for API response handling (no real API calls)

## What's NOT Tested (and Why)

### Async API Functions
The following are not unit tested because they require network access:
- `get_open_positions()`
- `get_ticker()`
- `get_balance()`
- `get_asset_pairs()`
- `find_pairs_for_assets()`
- `get_ohlc()`

**Note**: Kraken does not provide a sandbox for spot trading. Use mock testing or read-only API keys for safe integration testing.

### UI Rendering Functions
Functions in `src/tui/` are not unit tested because they:
- Require terminal mock infrastructure
- Are primarily visual/presentation layer
- Are better tested manually or with snapshot testing

### Main Application Logic
The `src/main.rs` orchestration code is not unit tested because it:
- Consists mainly of async task spawning and state management
- Is better suited for integration/end-to-end testing
- Would require extensive mocking of dependencies

## Test Quality Standards

All tests follow these principles:

1. **Descriptive Names**: Test names clearly describe what they test
2. **Arrange-Act-Assert**: Tests follow AAA pattern
3. **Edge Cases**: Cover zero values, negatives, invalid inputs
4. **Error Handling**: Verify graceful handling of bad data
5. **Isolation**: Each test is independent and can run in any order

## Adding New Tests

When adding new functionality:

1. **Write tests first** (TDD approach recommended)
2. **Test happy path** - normal, expected inputs
3. **Test edge cases** - boundary values, empty inputs
4. **Test error cases** - invalid data, missing fields
5. **Keep tests fast** - avoid slow operations in unit tests

### Example Test Template

```rust
#[test]
fn test_function_name_scenario() {
    // Arrange - set up test data
    let input = create_test_data();
    
    // Act - call the function
    let result = function_to_test(input);
    
    // Assert - verify the output
    assert_eq!(result, expected_value);
}
```

## Continuous Integration

To ensure tests run automatically:

```bash
# Add to CI pipeline
cargo test --all-features
cargo clippy -- -D warnings
cargo fmt -- --check
```

## Test Maintenance

- Run tests before every commit: `cargo test`
- Update tests when changing business logic
- Remove tests for deprecated functionality
- Keep test code as clean as production code

## Future Test Improvements

1. **Integration Tests**: Add tests in `tests/` directory for end-to-end scenarios
2. **Mock API**: Create mock Kraken API server for integration testing
3. **Property-Based Testing**: Use `proptest` for fuzzing edge cases
4. **Benchmark Tests**: Add performance benchmarks with `criterion`
5. **Coverage Reports**: Generate coverage with `tarpaulin` or `grcov`
