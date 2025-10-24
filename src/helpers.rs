use std::collections::HashMap;

/// Map asset names to their corresponding GBP trading pairs on Kraken
pub fn map_asset_to_gbp_pair(asset: &str) -> Option<String> {
    let pair = match asset {
        // Major cryptos with XX prefix
        "XXRP" => "XXRPZGBP",
        "XXBT" | "XBT" => "XXBTZGBP",
        "XETH" | "ETH" => "XETHZGBP",

        // Staked/wrapped tokens (map to base asset)
        "SUI.F" | "SUI.B" => "SUIZGBP",
        "ETH.F" => "ETHZGBP",
        "SOL.F" | "SOL03.S" => "SOLZGBP",
        "ATOM21.S" => "ATOMZGBP",

        // Altcoins
        "FET" => "FETZGBP",
        "KTA" => "KTAZGBP",

        // Stablecoins
        "USDT" | "U" => "USDTZGBP",

        // Default: try simple {ASSET}ZGBP format
        _ => return Some(format!("{}ZGBP", asset)),
    };

    Some(pair.to_string())
}

/// Normalize asset names by removing common Kraken prefixes and suffixes
pub fn normalize_asset_name(asset: &str) -> String {
    // Handle special cases first
    let asset = match asset {
        "U" => "USDT",
        "XL1" => "FLR",
        "XXRP" => "XRP",
        _ => {
            // Remove staked/wrapped suffixes
            if asset.contains("03.S") {
                return asset.replace("03.S", "");
            } else if asset.contains("21.S") {
                return asset.replace("21.S", "");
            }

            let cleaned = asset
                .trim_end_matches(".F")
                .trim_end_matches(".B")
                .trim_end_matches(".S");

            cleaned
        }
    };

    // Remove X/Z prefixes
    asset
        .trim_start_matches('X')
        .trim_start_matches('Z')
        .to_string()
}

/// Check if an asset is a fiat currency
pub fn is_fiat_currency(asset: &str) -> bool {
    asset.ends_with("GBP")
        || asset == "ZGBP"
        || asset == "GBP"
        || asset.ends_with("EUR")
        || asset == "ZEUR"
        || asset == "EUR"
        || asset.ends_with("USD")
        || asset == "ZUSD"
        || asset == "USD"
}

/// Get currency symbol for display
pub fn get_currency_symbol(currency: &str) -> &str {
    match currency {
        "GBP" => "£",
        "EUR" => "€",
        _ => "$",
    }
}

/// Calculate allocation percentage
pub fn calculate_allocation_percent(value: f64, total_value: f64) -> f64 {
    if total_value > 0.0 {
        (value / total_value) * 100.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_asset_to_gbp_pair_major_cryptos() {
        assert_eq!(map_asset_to_gbp_pair("XXRP"), Some("XXRPZGBP".to_string()));
        assert_eq!(map_asset_to_gbp_pair("XXBT"), Some("XXBTZGBP".to_string()));
        assert_eq!(map_asset_to_gbp_pair("XETH"), Some("XETHZGBP".to_string()));
    }

    #[test]
    fn test_map_asset_to_gbp_pair_staked_tokens() {
        assert_eq!(map_asset_to_gbp_pair("SUI.F"), Some("SUIZGBP".to_string()));
        assert_eq!(map_asset_to_gbp_pair("SUI.B"), Some("SUIZGBP".to_string()));
        assert_eq!(map_asset_to_gbp_pair("ETH.F"), Some("ETHZGBP".to_string()));
        assert_eq!(map_asset_to_gbp_pair("SOL.F"), Some("SOLZGBP".to_string()));
        assert_eq!(
            map_asset_to_gbp_pair("SOL03.S"),
            Some("SOLZGBP".to_string())
        );
        assert_eq!(
            map_asset_to_gbp_pair("ATOM21.S"),
            Some("ATOMZGBP".to_string())
        );
    }

    #[test]
    fn test_map_asset_to_gbp_pair_stablecoins() {
        assert_eq!(map_asset_to_gbp_pair("USDT"), Some("USDTZGBP".to_string()));
        assert_eq!(map_asset_to_gbp_pair("U"), Some("USDTZGBP".to_string()));
    }

    #[test]
    fn test_map_asset_to_gbp_pair_default_format() {
        // Unknown assets should get default format
        assert_eq!(map_asset_to_gbp_pair("DOGE"), Some("DOGEZGBP".to_string()));
        assert_eq!(map_asset_to_gbp_pair("ADA"), Some("ADAZGBP".to_string()));
    }

    #[test]
    fn test_normalize_asset_name_special_cases() {
        assert_eq!(normalize_asset_name("U"), "USDT");
        assert_eq!(normalize_asset_name("XL1"), "FLR");
        assert_eq!(normalize_asset_name("XXRP"), "RP");
    }

    #[test]
    fn test_normalize_asset_name_staked_suffixes() {
        assert_eq!(normalize_asset_name("SOL03.S"), "SOL");
        assert_eq!(normalize_asset_name("ATOM21.S"), "ATOM");
        assert_eq!(normalize_asset_name("ETH.F"), "ETH");
        assert_eq!(normalize_asset_name("SUI.B"), "SUI");
    }

    #[test]
    fn test_normalize_asset_name_prefixes() {
        assert_eq!(normalize_asset_name("XETH"), "ETH");
        assert_eq!(normalize_asset_name("XXBT"), "BT");
        assert_eq!(normalize_asset_name("ZGBP"), "GBP");
    }

    #[test]
    fn test_is_fiat_currency() {
        // GBP variants
        assert!(is_fiat_currency("GBP"));
        assert!(is_fiat_currency("ZGBP"));
        assert!(is_fiat_currency("USDGBP"));

        // EUR variants
        assert!(is_fiat_currency("EUR"));
        assert!(is_fiat_currency("ZEUR"));
        assert!(is_fiat_currency("USDEUR"));

        // USD variants
        assert!(is_fiat_currency("USD"));
        assert!(is_fiat_currency("ZUSD"));
        assert!(is_fiat_currency("BTCUSD"));

        // Cryptos should return false
        assert!(!is_fiat_currency("BTC"));
        assert!(!is_fiat_currency("ETH"));
        assert!(!is_fiat_currency("XXRP"));
    }

    #[test]
    fn test_get_currency_symbol() {
        assert_eq!(get_currency_symbol("GBP"), "£");
        assert_eq!(get_currency_symbol("EUR"), "€");
        assert_eq!(get_currency_symbol("USD"), "$");
        assert_eq!(get_currency_symbol("unknown"), "$");
    }

    #[test]
    fn test_calculate_allocation_percent() {
        assert_eq!(calculate_allocation_percent(100.0, 1000.0), 10.0);
        assert_eq!(calculate_allocation_percent(250.0, 1000.0), 25.0);
        assert_eq!(calculate_allocation_percent(1000.0, 1000.0), 100.0);
    }

    #[test]
    fn test_calculate_allocation_percent_zero_total() {
        // Should return 0 when total is 0 to avoid division by zero
        assert_eq!(calculate_allocation_percent(100.0, 0.0), 0.0);
    }

    #[test]
    fn test_calculate_allocation_percent_edge_cases() {
        assert_eq!(calculate_allocation_percent(0.0, 1000.0), 0.0);
        assert_eq!(calculate_allocation_percent(0.0, 0.0), 0.0);

        // Very small values
        let result = calculate_allocation_percent(0.001, 100.0);
        assert!(result > 0.0 && result < 0.01);
    }
}
