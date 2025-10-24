mod kraken;
mod models;
mod tui;

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::collections::HashMap;
use std::env;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use chrono;

use kraken::{KrakenClient, WebSocketManager};
use tui::render_positions;


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
    let ws_manager = Arc::new(WebSocketManager::new());

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let selected_index = Arc::new(Mutex::new(0usize));
    let should_quit = Arc::new(Mutex::new(false));
    let should_redraw = Arc::new(Mutex::new(true));
    let show_help = Arc::new(Mutex::new(false));
    
    // Shared state for positions, balances, asset-to-pair mapping
    let positions_data = Arc::new(Mutex::new(HashMap::new()));
    let balance_data = Arc::new(Mutex::new(HashMap::new()));
    let asset_pair_map = Arc::new(Mutex::new(HashMap::new()));
    
    // Store display currency for rendering
    let display_currency_shared = Arc::new(Mutex::new(display_currency.clone()));
    
    // Track loading state
    let is_loading = Arc::new(Mutex::new(true));
    let loading_stage = Arc::new(Mutex::new(String::from("Initializing...")));

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
                                let mut help = show_help_clone.lock().unwrap();
                                *help = !*help;
                                *should_redraw_clone.lock().unwrap() = true;
                            }
                            KeyCode::Char('c') | KeyCode::Char('C') => {
                                let mut currency = display_currency_kb.lock().unwrap();
                                *currency = match currency.as_str() {
                                    "GBP" => "USD".to_string(),
                                    "USD" => "EUR".to_string(),
                                    _ => "GBP".to_string(),
                                };
                                *should_redraw_clone.lock().unwrap() = true;
                            }
                            KeyCode::Esc => {
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

    // Spawn animation task for loading spinner
    let should_redraw_anim = Arc::clone(&should_redraw);
    let is_loading_anim = Arc::clone(&is_loading);
    let should_quit_anim = Arc::clone(&should_quit);
    
    tokio::spawn(async move {
        loop {
            if *should_quit_anim.lock().unwrap() {
                break;
            }
            
            if *is_loading_anim.lock().unwrap() {
                *should_redraw_anim.lock().unwrap() = true;
            }
            
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });
    
    // Spawn REST API task for positions and balances (updates less frequently)
    let positions_data_clone = Arc::clone(&positions_data);
    let balance_data_clone = Arc::clone(&balance_data);
    let asset_pair_map_clone = Arc::clone(&asset_pair_map);
    let is_loading_clone = Arc::clone(&is_loading);
    let loading_stage_clone = Arc::clone(&loading_stage);
    let should_redraw_clone = Arc::clone(&should_redraw);
    let should_quit_clone = Arc::clone(&should_quit);
    let client_clone = client.clone();
    let ws_manager_clone = Arc::clone(&ws_manager);
    
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        loop {
            if *should_quit_clone.lock().unwrap() {
                break;
            }

            // Fetch positions
            *loading_stage_clone.lock().unwrap() = "Fetching positions...".to_string();
            *should_redraw_clone.lock().unwrap() = true;
            match client_clone.get_open_positions().await {
                Ok(positions) => {
                    *positions_data_clone.lock().unwrap() = positions;
                }
                Err(_) => {}
            }

            // Fetch balances
            *loading_stage_clone.lock().unwrap() = "Fetching balances...".to_string();
            *should_redraw_clone.lock().unwrap() = true;
            match client_clone.get_balance().await {
                Ok(balances) => {
                    *balance_data_clone.lock().unwrap() = balances;
                }
                Err(_) => {}
            }

            // Collect all pairs needed for WebSocket subscription
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
                    !(asset.ends_with("GBP") || asset.as_str() == "ZGBP" || asset.as_str() == "GBP" 
                        || asset.ends_with("EUR") || asset.as_str() == "ZEUR" || asset.as_str() == "EUR"
                        || asset.ends_with("USD") || asset.as_str() == "ZUSD" || asset.as_str() == "USD")
                })
                .cloned()
                .collect();

            // Discover pairs for balance assets
            *loading_stage_clone.lock().unwrap() = "Discovering ticker pairs...".to_string();
            *should_redraw_clone.lock().unwrap() = true;
            if !balance_assets.is_empty() {
                match client_clone.find_pairs_for_assets(&balance_assets, "USD").await {
                    Ok(asset_pairs) => {
                        *asset_pair_map_clone.lock().unwrap() = asset_pairs.clone();
                        
                        for pair_name in asset_pairs.values() {
                            pairs.insert(pair_name.clone());
                        }
                        
                        // Add currency conversion pairs
                        pairs.insert("GBP/USD".to_string());
                        pairs.insert("EUR/USD".to_string());
                        pairs.insert("USD/GBP".to_string());
                        pairs.insert("USD/EUR".to_string());
                    }
                    Err(_) => {}
                }
            }
            
                    Ok(_) => {
                        *loading_stage_clone.lock().unwrap() = "WebSocket connected - receiving real-time updates".to_string();
                    }
                    Err(e) => {
                        *loading_stage_clone.lock().unwrap() = format!("WebSocket error: {:?}", e);
                    }
                }
            }
            
            // Mark as loaded after first successful fetch
            *is_loading_clone.lock().unwrap() = false;
            *should_redraw_clone.lock().unwrap() = true;

            // Update positions and balances every 60 seconds (less frequent than old 30s polling)
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    });

    // Spawn WebSocket update task (triggers redraws continuously)
    let should_redraw_ws = Arc::clone(&should_redraw);
    let should_quit_ws = Arc::clone(&should_quit);
    let ws_manager_ws = Arc::clone(&ws_manager);
    
    tokio::spawn(async move {
        let mut last_update_time: Option<u64> = None;
        loop {
            if *should_quit_ws.lock().unwrap() {
                break;
            }
            
            // Check if WebSocket has new data based on timestamp
            let current_update_time = ws_manager_ws.get_last_update_time().await;
            
            // Trigger redraw if we have new data or first time
            if let Some(current_time) = current_update_time {
                let should_redraw = last_update_time.map_or(true, |last| current_time > last);
                
                if should_redraw {
                    *should_redraw_ws.lock().unwrap() = true;
                    
                    // Debug: Log when data actually updates
                    use std::io::Write;
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/kraken_ws_redraws.txt") {
                        let _ = writeln!(file, "[{}] New WS data - timestamp: {} (prev: {:?})",
                            chrono::Utc::now().format("%H:%M:%S%.3f"),
                            current_time, last_update_time);
                    }
                    
                    last_update_time = Some(current_time);
                }
            }
            
            // Check frequently for new updates (100ms)
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
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
                let balances = balance_data.lock().unwrap().clone();
                let pair_map = asset_pair_map.lock().unwrap().clone();
                let current_index = *selected_index.lock().unwrap();
                let display_curr = display_currency_shared.lock().unwrap().clone();
                let help_visible = *show_help.lock().unwrap();
                let loading = *is_loading.lock().unwrap();
                let load_msg = loading_stage.lock().unwrap().clone();
                
                // Get real-time ticker data from WebSocket
                let ws_ticker_data = ws_manager.get_ticker_data().await;
                
                // Convert WebSocket ticker format to the format expected by render_positions
                let mut tickers = HashMap::new();
                let mut changes_24h = HashMap::new();
                
                for (pair, update) in ws_ticker_data {
                    // Create ticker data in old format
                    let ticker = models::TickerData {
                        c: vec![update.price.to_string()],
                        v: vec![update.volume.to_string()],
                        ..Default::default()
                    };
                    tickers.insert(pair.clone(), ticker);
                    changes_24h.insert(pair, update.change_24h);
                }
                
                render_positions(&mut terminal, &positions, &tickers, &balances, &pair_map, &changes_24h, &display_curr, help_visible, loading, &load_msg, current_index)?;
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
