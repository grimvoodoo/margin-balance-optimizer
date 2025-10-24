use ratatui::{
    layout::Constraint,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};
use std::collections::HashMap;

use crate::models::{BalanceEntry, TickerData};

pub fn render_balance(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    balances: &HashMap<String, String>,
    ticker_data: &HashMap<String, TickerData>,
    asset_pair_map: &HashMap<String, String>,
    price_changes_24h: &HashMap<String, f64>,
    display_currency: &str,
) {
    // Get conversion rate from USD to display currency
    let usd_to_display_rate = match display_currency {
        "GBP" => {
            ticker_data
                .get("USDGBP")
                .and_then(|t| t.c.first())
                .and_then(|p| p.parse::<f64>().ok())
                .unwrap_or(0.8) // Fallback approximate rate
        }
        "EUR" => {
            ticker_data
                .get("USDEUR")
                .and_then(|t| t.c.first())
                .and_then(|p| p.parse::<f64>().ok())
                .unwrap_or(0.92) // Fallback approximate rate
        }
        _ => 1.0, // USD or unknown
    };

    let currency_symbol = match display_currency {
        "GBP" => "£",
        "EUR" => "€",
        _ => "$",
    };
    // Convert balances to BalanceEntry and calculate values
    let mut balance_entries: Vec<BalanceEntry> = balances
        .iter()
        .filter_map(|(asset, balance_str)| {
            let balance: f64 = balance_str.parse().ok()?;

            // Skip zero or very small balances
            if balance < 0.000001 {
                return None;
            }

            // Skip fiat currencies for allocation calculation
            let is_fiat = asset.ends_with("GBP")
                || asset == "ZGBP"
                || asset == "GBP"
                || asset.ends_with("EUR")
                || asset == "ZEUR"
                || asset == "EUR"
                || asset.ends_with("USD")
                || asset == "ZUSD"
                || asset == "USD";

            // Try multiple pair name formats that Kraken uses
            let mut current_price = 0.0;

            if is_fiat {
                // Fiat currencies - convert to display currency
                if asset == "ZUSD" || asset == "USD" {
                    current_price = usd_to_display_rate;
                } else if asset == "ZGBP" || asset == "GBP" {
                    // GBP to display currency
                    if display_currency == "GBP" {
                        current_price = 1.0;
                    } else if display_currency == "EUR" {
                        current_price = ticker_data
                            .get("GBPUSD")
                            .and_then(|t| t.c.first())
                            .and_then(|p| p.parse::<f64>().ok())
                            .map(|gbp_usd| gbp_usd * usd_to_display_rate)
                            .unwrap_or(1.0);
                    } else {
                        // USD
                        current_price = ticker_data
                            .get("GBPUSD")
                            .and_then(|t| t.c.first())
                            .and_then(|p| p.parse::<f64>().ok())
                            .unwrap_or(1.0);
                    }
                } else if asset == "ZEUR" || asset == "EUR" {
                    // EUR to display currency
                    if display_currency == "EUR" {
                        current_price = 1.0;
                    } else {
                        // EUR -> USD -> display
                        current_price = ticker_data
                            .get("EURUSD")
                            .and_then(|t| t.c.first())
                            .and_then(|p| p.parse::<f64>().ok())
                            .map(|eur_usd| eur_usd * usd_to_display_rate)
                            .unwrap_or(1.0);
                    }
                } else {
                    current_price = 1.0;
                }
            } else {
                // Crypto assets - all priced in USD, convert to display currency
                if let Some(pair_name) = asset_pair_map.get(asset) {
                    if let Some(ticker) = ticker_data.get(pair_name) {
                        if let Some(price_str) = ticker.c.first() {
                            if let Ok(price_usd) = price_str.parse::<f64>() {
                                // Convert USD price to display currency
                                current_price = price_usd * usd_to_display_rate;
                            }
                        }
                    }
                }
            }

            let value_display = balance * current_price;

            Some(BalanceEntry {
                asset: asset.clone(),
                balance,
                value_gbp: value_display, // Rename would be value_display but keeping field name for compatibility
                current_price,
                allocation_percent: 0.0, // Will calculate after
            })
        })
        .collect();

    // Calculate total value for allocation percentages (excluding fiat)
    let total_value: f64 = balance_entries
        .iter()
        .filter(|e| {
            let asset = &e.asset;
            !(asset.ends_with("GBP")
                || asset == "ZGBP"
                || asset == "GBP"
                || asset.ends_with("EUR")
                || asset == "ZEUR"
                || asset == "EUR"
                || asset.ends_with("USD")
                || asset == "ZUSD"
                || asset == "USD")
        })
        .map(|e| e.value_gbp)
        .sum();

    // Update allocation percentages (only for non-fiat)
    for entry in &mut balance_entries {
        let is_fiat = entry.asset.ends_with("GBP")
            || entry.asset == "ZGBP"
            || entry.asset == "GBP"
            || entry.asset.ends_with("EUR")
            || entry.asset == "ZEUR"
            || entry.asset == "EUR"
            || entry.asset.ends_with("USD")
            || entry.asset == "ZUSD"
            || entry.asset == "USD";

        if is_fiat {
            entry.allocation_percent = 0.0;
        } else {
            entry.allocation_percent = if total_value > 0.0 {
                (entry.value_gbp / total_value) * 100.0
            } else {
                0.0
            };
        }
    }

    // Sort by current value (highest first) - more stable than allocation
    balance_entries.sort_by(|a, b| {
        b.value_gbp
            .partial_cmp(&a.value_gbp)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Take top 10
    balance_entries.truncate(10);

    let rows: Vec<Row> = balance_entries
        .iter()
        .map(|entry| {
            // Clean asset name (remove X/Z prefixes)
            let asset_display = entry
                .asset
                .trim_start_matches('X')
                .trim_start_matches('Z')
                .to_string();

            // Check if this is a fiat currency
            let is_fiat = entry.asset.ends_with("GBP")
                || entry.asset == "ZGBP"
                || entry.asset == "GBP"
                || entry.asset.ends_with("EUR")
                || entry.asset == "ZEUR"
                || entry.asset == "EUR"
                || entry.asset.ends_with("USD")
                || entry.asset == "ZUSD"
                || entry.asset == "USD";

            // Get 24h change - all are USD-based now for consistency
            let change_24h = if is_fiat {
                // For fiat currencies, show their change vs USD
                if entry.asset == "ZEUR" || entry.asset == "EUR" {
                    // EUR/USD change
                    price_changes_24h.get("EURUSD").copied().unwrap_or(0.0)
                } else if entry.asset == "ZGBP"
                    || entry.asset == "GBP"
                    || entry.asset.ends_with("GBP")
                {
                    // GBP/USD change
                    price_changes_24h.get("GBPUSD").copied().unwrap_or(0.0)
                } else if entry.asset == "ZUSD" || entry.asset == "USD" {
                    // USD shows no change vs itself
                    0.0
                } else {
                    0.0
                }
            } else {
                // For crypto assets, look up their pair's change
                asset_pair_map
                    .get(&entry.asset)
                    .and_then(|pair_name| price_changes_24h.get(pair_name))
                    .copied()
                    .unwrap_or(0.0)
            };

            // Format values properly
            let allocation_str = if entry.allocation_percent > 0.01 {
                format!("{:.2}%", entry.allocation_percent)
            } else {
                "0.00%".to_string()
            };

            let value_str = if entry.value_gbp > 0.01 {
                format!("{}{:.2}", currency_symbol, entry.value_gbp)
            } else {
                format!("{}0.00", currency_symbol)
            };

            let price_str = if entry.current_price > 0.0 {
                if entry.current_price < 1.0 {
                    format!("{}{:.4}", currency_symbol, entry.current_price)
                } else {
                    format!("{}{:.2}", currency_symbol, entry.current_price)
                }
            } else {
                format!("{}0.00", currency_symbol)
            };

            Row::new(vec![
                Cell::from(asset_display).style(Style::default().fg(Color::White)),
                Cell::from(allocation_str).style(Style::default().fg(Color::Cyan)),
                Cell::from(format!("{:.8}", entry.balance)),
                Cell::from(value_str),
                Cell::from(price_str),
                Cell::from(format!("{:+.2}%", change_24h)).style(if change_24h > 0.0 {
                    Style::default().fg(Color::Green)
                } else if change_24h < 0.0 {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default()
                }),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(12), // Asset
        Constraint::Length(12), // Allocation
        Constraint::Length(16), // Balance
        Constraint::Length(14), // Current Value
        Constraint::Length(14), // Avg Price
        Constraint::Length(10), // 24h Change
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec![
                "ASSET",
                "ALLOCATION",
                "BALANCE",
                "CURRENT VALUE",
                "AVG. PRICE",
                "24H CHG.",
            ])
            .style(Style::default().add_modifier(Modifier::BOLD))
            .bottom_margin(1),
        )
        .block(
            Block::default()
                .title("PORTFOLIO BALANCE")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

    f.render_widget(table, area);
}
