use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct KrakenResponse<T> {
    pub error: Vec<String>,
    pub result: Option<T>,
}

#[derive(Debug, Deserialize)]
pub struct OpenPositions {
    #[serde(flatten)]
    pub positions: HashMap<String, Position>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Position {
    pub ordertxid: String,
    pub posstatus: String,
    pub pair: String,
    pub time: f64,
    #[serde(rename = "type")]
    pub position_type: String,
    pub ordertype: String,
    pub cost: String,
    pub fee: String,
    pub vol: String,
    pub vol_closed: String,
    pub margin: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub net: Option<String>,
    #[serde(default)]
    pub terms: Option<String>,
    #[serde(default)]
    pub rollovertm: Option<String>,
    #[serde(default)]
    pub misc: Option<String>,
    #[serde(default)]
    pub oflags: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TickerInfo {
    #[serde(flatten)]
    pub pairs: HashMap<String, TickerData>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[allow(dead_code)]
pub struct TickerData {
    #[serde(default)]
    pub a: Vec<String>, // ask [price, whole lot volume, lot volume]
    #[serde(default)]
    pub b: Vec<String>, // bid [price, whole lot volume, lot volume]
    #[serde(default)]
    pub c: Vec<String>, // last trade closed [price, lot volume]
    #[serde(default)]
    pub v: Vec<String>, // volume [today, last 24 hours]
    #[serde(default)]
    pub p: Vec<String>, // volume weighted average price [today, last 24 hours]
    #[serde(default)]
    pub t: Vec<u32>, // number of trades [today, last 24 hours]
    #[serde(default)]
    pub l: Vec<String>, // low [today, last 24 hours]
    #[serde(default)]
    pub h: Vec<String>, // high [today, last 24 hours]
    #[serde(default)]
    pub o: String, // today's opening price
}

#[derive(Debug, Deserialize)]
pub struct AccountBalance {
    #[serde(flatten)]
    pub balances: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct AssetPairInfo {
    #[serde(flatten)]
    pub pairs: HashMap<String, AssetPair>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct AssetPair {
    pub altname: String,
    pub wsname: String,
    pub base: String,
    pub quote: String,
}

// OHLC response - using Value for flexible parsing
pub type OHLCResponse = HashMap<String, serde_json::Value>;

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct BalanceEntry {
    pub asset: String,
    pub balance: f64,
    pub value_gbp: f64,
    pub current_price: f64,
    pub allocation_percent: f64,
}

impl Position {
    pub fn calculate_unrealized_pnl(&self) -> f64 {
        // Kraken provides 'net' field which is the unrealized P&L
        if let Some(net) = &self.net {
            net.parse().unwrap_or(0.0)
        } else {
            // Fallback calculation if net is not available
            let cost: f64 = self.cost.parse().unwrap_or(0.0);
            let value: f64 = self
                .value
                .as_ref()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0);
            let fee: f64 = self.fee.parse().unwrap_or(0.0);

            value - cost - fee
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_position(
        net: Option<String>,
        value: Option<String>,
        cost: &str,
        fee: &str,
    ) -> Position {
        Position {
            ordertxid: "TEST123".to_string(),
            posstatus: "open".to_string(),
            pair: "XBTUSD".to_string(),
            time: 1234567890.0,
            position_type: "buy".to_string(),
            ordertype: "market".to_string(),
            cost: cost.to_string(),
            fee: fee.to_string(),
            vol: "1.0".to_string(),
            vol_closed: "0.0".to_string(),
            margin: "100.0".to_string(),
            value,
            net,
            terms: None,
            rollovertm: None,
            misc: None,
            oflags: None,
        }
    }

    #[test]
    fn test_calculate_unrealized_pnl_with_net() {
        // When net field is provided, it should use that value
        let position = create_test_position(
            Some("150.50".to_string()),
            Some("1050.50".to_string()),
            "900.0",
            "10.0",
        );

        assert_eq!(position.calculate_unrealized_pnl(), 150.50);
    }

    #[test]
    fn test_calculate_unrealized_pnl_without_net() {
        // When net field is missing, calculate from value - cost - fee
        let position = create_test_position(None, Some("1050.50".to_string()), "900.0", "10.0");

        // 1050.50 - 900.0 - 10.0 = 140.50
        assert_eq!(position.calculate_unrealized_pnl(), 140.50);
    }

    #[test]
    fn test_calculate_unrealized_pnl_negative() {
        // Test negative P&L
        let position = create_test_position(
            Some("-50.25".to_string()),
            Some("850.0".to_string()),
            "900.0",
            "10.0",
        );

        assert_eq!(position.calculate_unrealized_pnl(), -50.25);
    }

    #[test]
    fn test_calculate_unrealized_pnl_missing_value() {
        // When value is missing, should default to 0
        let position = create_test_position(None, None, "900.0", "10.0");

        // 0 - 900.0 - 10.0 = -910.0
        assert_eq!(position.calculate_unrealized_pnl(), -910.0);
    }

    #[test]
    fn test_calculate_unrealized_pnl_invalid_strings() {
        // Test with invalid numeric strings
        let position = create_test_position(
            Some("invalid".to_string()),
            Some("1000.0".to_string()),
            "invalid",
            "invalid",
        );

        // Should default to 0 when net is invalid
        assert_eq!(position.calculate_unrealized_pnl(), 0.0);
    }

    #[test]
    fn test_calculate_unrealized_pnl_zero_values() {
        // Test with zero values
        let position = create_test_position(
            Some("0.0".to_string()),
            Some("0.0".to_string()),
            "0.0",
            "0.0",
        );

        assert_eq!(position.calculate_unrealized_pnl(), 0.0);
    }
}
