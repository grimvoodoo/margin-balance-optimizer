use anyhow::Result;
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
    Terminal,
};
use std::collections::HashMap;

use crate::constants::POSITION_UPDATE_INTERVAL_SECS;
use crate::models::{Position, TickerData};
use crate::tui::render_balance;

/// Configuration for rendering the positions UI
pub struct RenderConfig<'a> {
    pub positions: &'a HashMap<String, Position>,
    pub ticker_data: &'a HashMap<String, TickerData>,
    pub balances: &'a HashMap<String, String>,
    pub asset_pair_map: &'a HashMap<String, String>,
    pub price_changes_24h: &'a HashMap<String, f64>,
    pub display_currency: &'a str,
    pub show_help: bool,
    pub is_loading: bool,
    pub loading_message: &'a str,
    pub selected_index: usize,
    pub seconds_since_update: f64,
}

pub fn render_positions<B: Backend>(
    terminal: &mut Terminal<B>,
    config: &RenderConfig,
) -> Result<()> {
    terminal.draw(|f| {
        let size = f.area();

        // Split the screen into three sections: positions, balance, status bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(58), // Positions table
                Constraint::Percentage(39), // Balance table
                Constraint::Length(3),      // Status bar
            ])
            .split(size);

        // Sort positions by UP&L (highest to lowest)
        let mut sorted_positions: Vec<_> = config.positions.iter().collect();
        sorted_positions.sort_by(|a, b| {
            let upnl_a = a.1.calculate_unrealized_pnl();
            let upnl_b = b.1.calculate_unrealized_pnl();
            upnl_b
                .partial_cmp(&upnl_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let rows: Vec<Row> = sorted_positions
            .iter()
            .enumerate()
            .map(|(index, (_pos_id, position))| {
                let pair = &position.pair;
                let side = &position.position_type;
                let volume: f64 = position.vol.parse().unwrap_or(0.0);
                let cost: f64 = position.cost.parse().unwrap_or(0.0);
                let margin: f64 = position.margin.parse().unwrap_or(0.0);
                let open_price = if volume != 0.0 { cost / volume } else { 0.0 };

                let current_price = config
                    .ticker_data
                    .get(pair)
                    .and_then(|t| t.c.first())
                    .and_then(|p| p.parse::<f64>().ok())
                    .unwrap_or(0.0);

                let upnl = position.calculate_unrealized_pnl();

                let side_text = if side.to_lowercase() == "buy" {
                    "Long"
                } else {
                    "Short"
                };
                let side_color = if side.to_lowercase() == "buy" {
                    Color::Green
                } else {
                    Color::Red
                };
                let upnl_color = if upnl > 0.0 {
                    Color::Green
                } else if upnl < 0.0 {
                    Color::Red
                } else {
                    Color::White
                };

                let is_selected = index == config.selected_index;

                if is_selected {
                    // Selected row: white background, black text
                    Row::new(vec![
                        Cell::from(pair.to_string()),
                        Cell::from(side_text.to_string()),
                        Cell::from(format!("{:.8}", volume)),
                        Cell::from(format!("{:.2}", open_price)),
                        Cell::from(format!("{:.2}", current_price)),
                        Cell::from(format!("{:.2}", margin)),
                        Cell::from(format!("{:+.2}", upnl)),
                    ])
                    .style(
                        Style::default()
                            .bg(Color::White)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    // Non-selected row: colored cells
                    Row::new(vec![
                        Cell::from(pair.to_string()).style(Style::default().fg(Color::Cyan)),
                        Cell::from(side_text.to_string()).style(Style::default().fg(side_color)),
                        Cell::from(format!("{:.8}", volume)),
                        Cell::from(format!("{:.2}", open_price)),
                        Cell::from(format!("{:.2}", current_price)),
                        Cell::from(format!("{:.2}", margin)),
                        Cell::from(format!("{:+.2}", upnl))
                            .style(Style::default().fg(upnl_color).add_modifier(Modifier::BOLD)),
                    ])
                }
            })
            .collect();

        let widths = [
            Constraint::Length(15),
            Constraint::Length(6),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(16),
        ];

        let table = Table::new(rows, widths)
            .header(
                Row::new(vec![
                    "PAIR",
                    "SIDE",
                    "OPEN QTY",
                    "OPEN PRICE",
                    "CURRENT",
                    "MARGIN",
                    "UP&L",
                ])
                .style(Style::default().add_modifier(Modifier::BOLD))
                .bottom_margin(1),
            )
            .block(
                Block::default()
                    .title("KRAKEN POSITIONS MONITOR")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::White)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_widget(table, chunks[0]);

        // Render balance table
        render_balance(
            f,
            chunks[1],
            config.balances,
            config.ticker_data,
            config.asset_pair_map,
            config.price_changes_24h,
            config.display_currency,
        );

        // Render status bar with subtle progress indicator
        let progress = (config.seconds_since_update / POSITION_UPDATE_INTERVAL_SECS).min(1.0);

        // Create two chunks: one for progress bar, one for status text
        let status_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Progress bar
                Constraint::Length(2), // Status text
            ])
            .split(chunks[2]);

        // Render subtle progress bar with dotted style
        let progress_char = "▪";
        let empty_char = "·";
        let total_chars = size.width as usize;
        let filled_chars = (total_chars as f64 * progress) as usize;

        let mut progress_text = String::new();
        for i in 0..total_chars {
            if i < filled_chars {
                progress_text.push_str(progress_char);
            } else {
                progress_text.push_str(empty_char);
            }
        }

        // Keep color calm and reassuring - subtle cyan that matches the theme
        let progress_bar = Paragraph::new(progress_text)
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::NONE));
        f.render_widget(progress_bar, status_chunks[0]);

        // Render status text
        let status_text = format!(
            " [H]elp  [C] Balance Currency: {}  [Q]uit ",
            config.display_currency
        );
        let status_bar = Paragraph::new(status_text)
            .style(Style::default().bg(Color::DarkGray).fg(Color::White))
            .block(Block::default().borders(Borders::NONE));
        f.render_widget(status_bar, status_chunks[1]);

        // Render loading overlay if still loading
        if config.is_loading {
            let loading_block = Block::default()
                .title(" Loading ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));

            // Simple spinner animation based on time
            let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let frame_idx = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
                / 100) as usize
                % spinner_frames.len();
            let spinner = spinner_frames[frame_idx];

            let loading_text = format!(
                "\n\n    {}  {}\n\n    Please wait...",
                spinner, config.loading_message
            );

            let loading_paragraph = Paragraph::new(loading_text)
                .block(loading_block)
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Left);

            let area = centered_rect(50, 30, size);
            f.render_widget(Clear, area);
            f.render_widget(loading_paragraph, area);
        }
        // Render help modal if requested
        else if config.show_help {
            let help_block = Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));

            let help_text = vec![
                "Keyboard Shortcuts:",
                "",
                "  ↑/↓      Navigate positions",
                "  H        Toggle this help",
                "  C        Cycle balance display currency (GBP → USD → EUR)",
                "  Q        Quit",
                "  ESC      Close this help",
                "",
                "Currency Display:",
                "• Balance values shown in selected currency",
                "• Positions show native trading pair prices",
                "• All 24H changes calculated vs USD",
                "",
                "Press ESC or H to close",
            ];

            let help_content = help_text.join("\n");
            let help_paragraph = Paragraph::new(help_content)
                .block(help_block)
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Left);

            // Center the help modal
            let area = centered_rect(60, 50, size);
            f.render_widget(Clear, area); // Clear background
            f.render_widget(help_paragraph, area);
        }
    })?;

    Ok(())
}

/// Helper function to create a centered rectangle
fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
