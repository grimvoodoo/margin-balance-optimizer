mod kraken;
mod models;
mod tui;

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::HashMap;
use std::env;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kraken::KrakenClient;
use tui::{render_positions, RenderConfig};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let api_key = env::var("KRAKEN_API_KEY").context("KRAKEN_API_KEY not found in environment")?;
    let private_key =
        env::var("KRAKEN_PRIVATE_KEY").context("KRAKEN_PRIVATE_KEY not found in environment")?;

    // Get display currency from environment (default to GBP)
    let display_currency = env::var("DISPLAY_CURRENCY").unwrap_or_else(|_| "GBP".to_string());
    let display_currency = display_currency.to_uppercase();

    // Validate currency
    if !["GBP", "USD", "EUR"].contains(&display_currency.as_str()) {
        anyhow::bail!("DISPLAY_CURRENCY must be one of: GBP, USD, EUR");
    }

    let client = KrakenClient::new(api_key, private_key)?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let selected_index = Arc::new(Mutex::new(0usize));
    let should_quit = Arc::new(Mutex::new(false));
    let should_redraw = Arc::new(Mutex::new(true)); // Start with true to draw initially
    let show_help = Arc::new(Mutex::new(false));

    // Shared state for positions, ticker data, balances, asset-to-pair mapping, and 24h price changes
    let positions_data = Arc::new(Mutex::new(HashMap::new()));
    let ticker_data = Arc::new(Mutex::new(HashMap::new()));
    let balance_data = Arc::new(Mutex::new(HashMap::new()));
    let asset_pair_map = Arc::new(Mutex::new(HashMap::new()));
    let price_changes_24h = Arc::new(Mutex::new(HashMap::new()));

    // Caching: Track last time we fetched asset pairs
    let last_asset_pairs_fetch = Arc::new(Mutex::new(
        std::time::Instant::now() - std::time::Duration::from_secs(3600),
    ));

    // Store display currency for rendering (mutable for runtime switching)
    let display_currency_shared = Arc::new(Mutex::new(display_currency.clone()));

    // Track loading state
    let is_loading = Arc::new(Mutex::new(true));
    let loading_stage = Arc::new(Mutex::new(String::from("Initializing...")));

    // Track last update time for progress bar
    let last_update_time = Arc::new(Mutex::new(std::time::Instant::now()));

    // Spawn keyboard handler task
    let selected_index_clone = Arc::clone(&selected_index);
    let should_quit_clone = Arc::clone(&should_quit);
    let should_redraw_clone = Arc::clone(&should_redraw);
    let show_help_clone = Arc::clone(&show_help);
    let display_currency_kb = Arc::clone(&display_currency_shared);
    let positions_data_clone = Arc::clone(&positions_data);

    tokio::spawn(async move {
        loop {
            if event::poll(Duration::from_millis(100)).unwrap_or(false) {
                if let Ok(Event::Key(key_event)) = event::read() {
                    if key_event.kind == KeyEventKind::Press {
                        match key_event.code {
                            KeyCode::Up => {
                                let mut index = selected_index_clone.lock().unwrap();
                                if *index > 0 {
                                    *index -= 1;
                                    *should_redraw_clone.lock().unwrap() = true;
                                }
                            }
                            KeyCode::Down => {
                                let mut index = selected_index_clone.lock().unwrap();
                                let max = positions_data_clone.lock().unwrap().len();
                                if max > 0 && *index < max - 1 {
                                    *index += 1;
                                    *should_redraw_clone.lock().unwrap() = true;
                                }
                            }
                            KeyCode::Char('q') | KeyCode::Char('Q') => {
                                *should_quit_clone.lock().unwrap() = true;
                                break;
                            }
                            KeyCode::Char('h') | KeyCode::Char('H') => {
                                // Toggle help modal
                                let mut help = show_help_clone.lock().unwrap();
                                *help = !*help;
                                *should_redraw_clone.lock().unwrap() = true;
                            }
                            KeyCode::Char('c') | KeyCode::Char('C') => {
                                // Cycle through currencies: GBP -> USD -> EUR -> GBP
                                let mut currency = display_currency_kb.lock().unwrap();
                                *currency = match currency.as_str() {
                                    "GBP" => "USD".to_string(),
                                    "USD" => "EUR".to_string(),
                                    _ => "GBP".to_string(),
                                };
                                *should_redraw_clone.lock().unwrap() = true;
                            }
                            KeyCode::Esc => {
                                // Close help modal
                                if *show_help_clone.lock().unwrap() {
                                    *show_help_clone.lock().unwrap() = false;
                                    *should_redraw_clone.lock().unwrap() = true;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    });

    // Spawn animation task for loading spinner and progress bar
    let should_redraw_anim = Arc::clone(&should_redraw);
    let is_loading_anim = Arc::clone(&is_loading);
    let should_quit_anim = Arc::clone(&should_quit);

    tokio::spawn(async move {
        loop {
            if *should_quit_anim.lock().unwrap() {
                break;
            }

            // Trigger redraws during loading for spinner animation
            if *is_loading_anim.lock().unwrap() {
                *should_redraw_anim.lock().unwrap() = true;
            } else {
                // Always trigger redraw for progress bar animation
                *should_redraw_anim.lock().unwrap() = true;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });

    // Spawn API polling task
    let positions_data_clone = Arc::clone(&positions_data);
    let ticker_data_clone = Arc::clone(&ticker_data);
    let balance_data_clone = Arc::clone(&balance_data);
    let asset_pair_map_clone = Arc::clone(&asset_pair_map);
    let price_changes_24h_clone = Arc::clone(&price_changes_24h);
    let display_currency_clone = Arc::clone(&display_currency_shared);
    let is_loading_clone = Arc::clone(&is_loading);
    let loading_stage_clone = Arc::clone(&loading_stage);
    let should_redraw_clone = Arc::clone(&should_redraw);
    let should_quit_clone = Arc::clone(&should_quit);
    let client_clone = client.clone();
    let last_asset_pairs_fetch_clone = Arc::clone(&last_asset_pairs_fetch);

    // Spawn fast position update loop (every 6 seconds)
    let positions_fast_clone = Arc::clone(&positions_data);
    let should_quit_fast = Arc::clone(&should_quit);
    let should_redraw_fast = Arc::clone(&should_redraw);
    let loading_stage_fast = Arc::clone(&loading_stage);
    let is_loading_fast = Arc::clone(&is_loading);
    let client_fast = client.clone();
    let last_update_fast = Arc::clone(&last_update_time);

    tokio::spawn(async move {
        // Start positions first (100ms delay)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let mut first_run = true;

        loop {
            if *should_quit_fast.lock().unwrap() {
                break;
            }

            // Show loading message only on first run
            if first_run && *is_loading_fast.lock().unwrap() {
                *loading_stage_fast.lock().unwrap() = "Fetching positions...".to_string();
                *should_redraw_fast.lock().unwrap() = true;
            }

            // Reset update time at start of cycle
            *last_update_fast.lock().unwrap() = std::time::Instant::now();

            // Debug: Log position fetch
            use std::io::Write;
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/kraken_position_fetch.txt")
            {
                let _ = writeln!(
                    file,
                    "[{}] Fetching positions...",
                    chrono::Utc::now().format("%H:%M:%S%.3f")
                );
            }

            match client_fast.get_open_positions().await {
                Ok(positions) => {
                    // Debug: Log position received
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/kraken_position_fetch.txt")
                    {
                        let _ = writeln!(
                            file,
                            "[{}] Received {} positions",
                            chrono::Utc::now().format("%H:%M:%S%.3f"),
                            positions.len()
                        );
                    }
                    *positions_fast_clone.lock().unwrap() = positions;
                    first_run = false;
                }
                Err(e) => {
                    first_run = false;
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/kraken_position_fetch.txt")
                    {
                        let _ = writeln!(
                            file,
                            "[{}] ERROR: {:?}",
                            chrono::Utc::now().format("%H:%M:%S%.3f"),
                            e
                        );
                    }
                }
            }

            // Update every 6 seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;
        }
    });

    tokio::spawn(async move {
        // Wait for positions to complete first (600ms = 100ms + 500ms for position API call)
        tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;

        loop {
            if *should_quit_clone.lock().unwrap() {
                break;
            }

            let is_loading = *is_loading_clone.lock().unwrap();

            // Fetch balances
            if is_loading {
                *loading_stage_clone.lock().unwrap() = "Fetching balances...".to_string();
                *should_redraw_clone.lock().unwrap() = true;
            }

            // Debug: Log balance fetch
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/kraken_balance_fetch.txt")
            {
                let _ = writeln!(
                    file,
                    "[{}] Fetching balances...",
                    chrono::Utc::now().format("%H:%M:%S%.3f")
                );
            }

            match client_clone.get_balance().await {
                Ok(balances) => {
                    // Debug: Log balance received
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/kraken_balance_fetch.txt")
                    {
                        let _ = writeln!(
                            file,
                            "[{}] Received {} balances",
                            chrono::Utc::now().format("%H:%M:%S%.3f"),
                            balances.len()
                        );
                    }
                    *balance_data_clone.lock().unwrap() = balances;
                }
                Err(e) => {
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/kraken_balance_fetch.txt")
                    {
                        let _ = writeln!(
                            file,
                            "[{}] ERROR: {:?}",
                            chrono::Utc::now().format("%H:%M:%S%.3f"),
                            e
                        );
                    }
                }
            }

            // Collect all pairs needed for ticker data (from positions and balances)
            let mut pairs: std::collections::HashSet<String> = positions_data_clone
                .lock()
                .unwrap()
                .values()
                .map(|p| p.pair.clone())
                .collect();

            // Get all non-fiat assets from balances
            let balance_assets: Vec<String> = balance_data_clone
                .lock()
                .unwrap()
                .keys()
                .filter(|asset| {
                    // Skip fiat currencies
                    !(asset.ends_with("GBP")
                        || asset.as_str() == "ZGBP"
                        || asset.as_str() == "GBP"
                        || asset.ends_with("EUR")
                        || asset.as_str() == "ZEUR"
                        || asset.as_str() == "EUR"
                        || asset.ends_with("USD")
                        || asset.as_str() == "ZUSD"
                        || asset.as_str() == "USD")
                })
                .cloned()
                .collect();

            // Debug logging - import Write once for this block
            use std::io::Write;
            if let Ok(mut file) = std::fs::File::create("/tmp/kraken_discovery_start.txt") {
                let _ = writeln!(
                    file,
                    "Starting pair discovery for {} assets",
                    balance_assets.len()
                );
                for asset in &balance_assets {
                    let _ = writeln!(file, "  - {}", asset);
                }
            }

            // Dynamically find valid pairs for our assets (cached - only fetch every hour)
            let should_fetch_pairs = {
                let last_fetch = last_asset_pairs_fetch_clone.lock().unwrap();
                last_fetch.elapsed() > std::time::Duration::from_secs(3600) // 1 hour
            };

            if !balance_assets.is_empty()
                && (should_fetch_pairs || asset_pair_map_clone.lock().unwrap().is_empty())
            {
                if is_loading {
                    *loading_stage_clone.lock().unwrap() =
                        "Discovering ticker pairs...".to_string();
                    *should_redraw_clone.lock().unwrap() = true;
                }

                // Update last fetch time
                *last_asset_pairs_fetch_clone.lock().unwrap() = std::time::Instant::now();

                match client_clone
                    .find_pairs_for_assets(&balance_assets, "USD")
                    .await
                {
                    Ok(asset_pairs) => {
                        // Store the mapping for use in rendering
                        *asset_pair_map_clone.lock().unwrap() = asset_pairs.clone();

                        // Debug: Log the asset-to-pair mappings
                        if let Ok(mut file) =
                            std::fs::File::create("/tmp/kraken_asset_mappings.txt")
                        {
                            let _ = writeln!(file, "=== Asset to Pair Mappings ===");
                            let _ = writeln!(file, "Total mappings: {}\n", asset_pairs.len());
                            let mut sorted: Vec<_> = asset_pairs.iter().collect();
                            sorted.sort_by_key(|(k, _)| k.as_str());
                            for (asset, pair) in sorted {
                                let _ = writeln!(file, "  {} -> {}", asset, pair);
                            }
                            let _ = writeln!(file, "\n=== Unmapped Assets ===");
                            for asset in &balance_assets {
                                if !asset_pairs.contains_key(asset) {
                                    let _ = writeln!(file, "  {}", asset);
                                }
                            }
                        }

                        for pair_name in asset_pairs.values() {
                            pairs.insert(pair_name.clone());
                        }

                        // Fetch conversion rates between major currencies
                        // Always fetch USD pairs since that's our calculation base
                        pairs.insert("GBPUSD".to_string());
                        pairs.insert("EURUSD".to_string());

                        // Also fetch cross rates for display currency conversion
                        let curr = display_currency_clone.lock().unwrap();
                        if curr.as_str() == "GBP" {
                            pairs.insert("USDGBP".to_string());
                        } else if curr.as_str() == "EUR" {
                            pairs.insert("USDEUR".to_string());
                        }
                        drop(curr); // Release lock
                    }
                    Err(e) => {
                        if let Ok(mut file) =
                            std::fs::File::create("/tmp/kraken_pair_discovery_error.txt")
                        {
                            let _ = writeln!(file, "Error discovering pairs: {:?}", e);
                        }
                    }
                }
            } else if !balance_assets.is_empty() {
                // Use cached asset pair mappings
                let cached_pairs = asset_pair_map_clone.lock().unwrap();
                for pair_name in cached_pairs.values() {
                    pairs.insert(pair_name.clone());
                }
            }

            // Always add currency conversion pairs
            pairs.insert("GBPUSD".to_string());
            pairs.insert("EURUSD".to_string());
            {
                let curr = display_currency_clone.lock().unwrap().clone();
                if curr.as_str() == "GBP" {
                    pairs.insert("USDGBP".to_string());
                } else if curr.as_str() == "EUR" {
                    pairs.insert("USDEUR".to_string());
                }
            }

            let pairs_vec: Vec<String> = pairs.into_iter().collect();

            // Debug: Write requested pairs BEFORE API call
            if let Ok(mut file) = std::fs::File::create("/tmp/kraken_pairs_requested.txt") {
                let _ = writeln!(
                    file,
                    "=== Pairs we're requesting ({} total) ===",
                    pairs_vec.len()
                );
                for (i, pair) in pairs_vec.iter().enumerate() {
                    let _ = writeln!(file, "  {}: {}", i, pair);
                }
                let _ = writeln!(file, "\n=== Balance Assets ===");
                let balances = balance_data_clone.lock().unwrap();
                for (asset, bal) in balances.iter() {
                    let _ = writeln!(file, "  {} = {}", asset, bal);
                }
            }

            // Fetch tickers in batches to reduce API calls (25 calls -> 3 calls)
            if is_loading {
                *loading_stage_clone.lock().unwrap() =
                    format!("Fetching {} ticker prices...", pairs_vec.len());
                *should_redraw_clone.lock().unwrap() = true;
            }
            if !pairs_vec.is_empty() {
                let mut all_tickers = HashMap::new();
                let mut successful_pairs = Vec::new();
                let mut failed_pairs = Vec::new();

                // Batch in groups of 10 pairs per API call
                let batch_size = 10;
                for (batch_idx, chunk) in pairs_vec.chunks(batch_size).enumerate() {
                    // Delay between batches to avoid rate limiting
                    if batch_idx > 0 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    }

                    match client_clone.get_ticker(chunk.to_vec()).await {
                        Ok(tickers) => {
                            for pair in chunk {
                                successful_pairs.push(pair.clone());
                            }
                            all_tickers.extend(tickers);
                        }
                        Err(e) => {
                            for pair in chunk {
                                failed_pairs.push((pair.clone(), format!("{:?}", e)));
                            }
                        }
                    }
                }

                // Debug: Write results
                if let Ok(mut file) = std::fs::File::create("/tmp/kraken_debug.txt") {
                    let _ = writeln!(file, "Pairs requested: {}", pairs_vec.len());
                    let _ = writeln!(file, "Successful: {}", successful_pairs.len());
                    let _ = writeln!(file, "Failed: {}\n", failed_pairs.len());

                    if !successful_pairs.is_empty() {
                        let _ = writeln!(file, "=== Successful Pairs ===");
                        for (i, pair) in successful_pairs.iter().enumerate() {
                            let _ = writeln!(file, "  {}: {}", i, pair);
                        }
                    }

                    if !failed_pairs.is_empty() {
                        let _ = writeln!(file, "\n=== Failed Pairs ===");
                        for (pair, error) in &failed_pairs {
                            let _ = writeln!(file, "  {}: {}", pair, error);
                        }
                    }

                    let _ = writeln!(file, "\n=== Received Ticker Data ===");
                    let mut received: Vec<_> = all_tickers.keys().collect();
                    received.sort();
                    for (i, pair) in received.iter().enumerate() {
                        if let Some(ticker) = all_tickers.get(*pair) {
                            let price = ticker.c.first().map(|s| s.as_str()).unwrap_or("0");
                            let _ = writeln!(file, "  {}: {} = £{}", i, pair, price);
                        }
                    }
                }

                *ticker_data_clone.lock().unwrap() = all_tickers;
            }

            // Fetch 24h price changes for all pairs
            if is_loading {
                *loading_stage_clone.lock().unwrap() = "Calculating 24H changes...".to_string();
                *should_redraw_clone.lock().unwrap() = true;
            }
            let mut changes_24h = HashMap::new();
            let mut ohlc_debug = Vec::new();

            for (idx, pair) in pairs_vec.iter().enumerate() {
                // Add delay between requests to avoid rate limiting
                // Kraken public API limit is ~1 call per second burst, so space them out
                if idx > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                }

                match client_clone.get_ohlc(pair, Some(1440)).await {
                    Ok(json) => {
                        // Parse the JSON response: {"error":[], "result":{"PAIRNAME":[[...],[...]], "last":timestamp}}
                        if let Some(result) = json.get("result") {
                            // Find the pair data (it's the key that's not "last")
                            if let Some(obj) = result.as_object() {
                                for (key, value) in obj {
                                    if key != "last" {
                                        if let Some(candles) = value.as_array() {
                                            ohlc_debug.push(format!(
                                                "  {}: {} candles",
                                                pair,
                                                candles.len()
                                            ));

                                            if candles.len() >= 2 {
                                                // Each candle is [time, open, high, low, close, vwap, volume, count]
                                                let yesterday = &candles[candles.len() - 2];
                                                let today = &candles[candles.len() - 1];

                                                if let (Some(prev_arr), Some(curr_arr)) =
                                                    (yesterday.as_array(), today.as_array())
                                                {
                                                    // Index 4 is the close price
                                                    if let (
                                                        Some(prev_close_str),
                                                        Some(curr_close_str),
                                                    ) = (prev_arr.get(4), curr_arr.get(4))
                                                    {
                                                        if let (Some(prev_str), Some(curr_str)) = (
                                                            prev_close_str.as_str(),
                                                            curr_close_str.as_str(),
                                                        ) {
                                                            if let (
                                                                Ok(prev_close),
                                                                Ok(curr_close),
                                                            ) = (
                                                                prev_str.parse::<f64>(),
                                                                curr_str.parse::<f64>(),
                                                            ) {
                                                                if prev_close > 0.0 {
                                                                    let change_pct = ((curr_close
                                                                        - prev_close)
                                                                        / prev_close)
                                                                        * 100.0;
                                                                    changes_24h.insert(
                                                                        pair.clone(),
                                                                        change_pct,
                                                                    );
                                                                    ohlc_debug.push(format!("    Change: {:.2}% (prev: {:.2}, curr: {:.2})", change_pct, prev_close, curr_close));
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        break; // Only process the first non-"last" key
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        ohlc_debug.push(format!("  {}: ERROR - {:?}", pair, e));
                    }
                }
            }

            // Debug: Write OHLC fetch results
            if let Ok(mut file) = std::fs::File::create("/tmp/kraken_ohlc_debug.txt") {
                let _ = writeln!(file, "=== OHLC Data Fetch Results ===");
                let _ = writeln!(file, "Total pairs attempted: {}", pairs_vec.len());
                let _ = writeln!(
                    file,
                    "Successful changes calculated: {}\n",
                    changes_24h.len()
                );
                for line in &ohlc_debug {
                    let _ = writeln!(file, "{}", line);
                }
                let _ = writeln!(file, "\n=== Calculated Changes ===");
                let mut sorted_changes: Vec<_> = changes_24h.iter().collect();
                sorted_changes.sort_by_key(|(k, _)| k.as_str());
                for (pair, change) in sorted_changes {
                    let _ = writeln!(file, "  {}: {:+.2}%", pair, change);
                }
            }

            *price_changes_24h_clone.lock().unwrap() = changes_24h;

            // Mark as loaded after first successful fetch
            *is_loading_clone.lock().unwrap() = false;

            // Always trigger redraw after API fetch cycle
            *should_redraw_clone.lock().unwrap() = true;

            // Poll every 30 seconds (Kraken data doesn't change that fast)
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    });

    // Main display loop
    let result = async {
        loop {
            if *should_quit.lock().unwrap() {
                break;
            }

            if *should_redraw.lock().unwrap() {
                let positions = positions_data.lock().unwrap().clone();
                let tickers = ticker_data.lock().unwrap().clone();
                let balances = balance_data.lock().unwrap().clone();
                let pair_map = asset_pair_map.lock().unwrap().clone();
                let changes_24h = price_changes_24h.lock().unwrap().clone();
                let current_index = *selected_index.lock().unwrap();
                let display_curr = display_currency_shared.lock().unwrap().clone();
                let help_visible = *show_help.lock().unwrap();
                let loading = *is_loading.lock().unwrap();
                let load_msg = loading_stage.lock().unwrap().clone();
                let last_update = last_update_time.lock().unwrap().elapsed().as_secs_f64();

                let config = RenderConfig {
                    positions: &positions,
                    ticker_data: &tickers,
                    balances: &balances,
                    asset_pair_map: &pair_map,
                    price_changes_24h: &changes_24h,
                    display_currency: &display_curr,
                    show_help: help_visible,
                    is_loading: loading,
                    loading_message: &load_msg,
                    selected_index: current_index,
                    seconds_since_update: last_update,
                };

                render_positions(&mut terminal, &config)?;
                *should_redraw.lock().unwrap() = false;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(16)).await; // ~60fps
        }
        Ok::<(), anyhow::Error>(())
    }
    .await;

    // Cleanup
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
