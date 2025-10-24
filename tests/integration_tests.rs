// Integration tests for Kraken API
// These tests use real API calls but with read-only operations
// Set up test credentials in .env.test

#[cfg(test)]
mod kraken_api_tests {
    // Helper to check if we should run integration tests
    fn should_run_integration_tests() -> bool {
        std::env::var("RUN_INTEGRATION_TESTS").is_ok()
    }

    #[tokio::test]
    #[ignore] // Run with: cargo test --ignored -- --test-threads=1
    async fn test_get_ticker_integration() {
        if !should_run_integration_tests() {
            println!("Skipping integration test. Set RUN_INTEGRATION_TESTS=1 to run.");
            return;
        }

        // This test only reads public data - completely safe
        // Note: We're using the production API, not futures
        // Public endpoints work without authentication
        let client = reqwest::Client::new();
        let response = client
            .get("https://api.kraken.com/0/public/Ticker")
            .query(&[("pair", "XBTUSD")])
            .send()
            .await
            .expect("Failed to fetch ticker");

        assert!(response.status().is_success());

        let json: serde_json::Value = response.json().await.unwrap();
        assert!(json["error"].as_array().unwrap().is_empty());
        assert!(json["result"].is_object());

        println!("✅ Public ticker API working correctly");
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_asset_pairs_integration() {
        if !should_run_integration_tests() {
            return;
        }

        // Another safe public endpoint test
        let client = reqwest::Client::new();
        let response = client
            .get("https://api.kraken.com/0/public/AssetPairs")
            .send()
            .await
            .expect("Failed to fetch asset pairs");

        assert!(response.status().is_success());

        let json: serde_json::Value = response.json().await.unwrap();
        assert!(json["error"].as_array().unwrap().is_empty());

        let pairs = json["result"].as_object().unwrap();
        assert!(!pairs.is_empty(), "Should return trading pairs");

        println!("✅ Asset pairs API working correctly");
    }

    // Note: Private endpoint tests would require your credentials
    // These are commented out for safety - only uncomment when you have
    // READ-ONLY test credentials set up

    /*
    #[tokio::test]
    #[ignore]
    async fn test_get_balance_integration() {
        if !should_run_integration_tests() {
            return;
        }

        let (api_key, private_key) = load_test_credentials();

        // This would test your read-only credentials
        // Only runs when explicitly enabled

        println!("⚠️  This test uses your API credentials");
        println!("⚠️  Ensure your .env.test uses READ-ONLY keys");

        // Implementation here...
    }
    */
}

#[cfg(test)]
mod mock_api_tests {
    // Mock-based tests that don't hit real API
    // These are always safe to run

    #[test]
    fn test_parse_ticker_response() {
        let mock_response = r#"{
            "error": [],
            "result": {
                "XXBTZUSD": {
                    "a": ["50000.00000", "1", "1.000"],
                    "b": ["49999.00000", "2", "2.000"],
                    "c": ["50000.00000", "0.00100000"],
                    "v": ["1000.12345678", "2000.12345678"],
                    "p": ["49500.00000", "49600.00000"],
                    "t": [1000, 2000],
                    "l": ["49000.00000", "49000.00000"],
                    "h": ["51000.00000", "51000.00000"],
                    "o": "49500.00000"
                }
            }
        }"#;

        let parsed: serde_json::Value = serde_json::from_str(mock_response).unwrap();

        assert!(parsed["error"].as_array().unwrap().is_empty());
        assert!(parsed["result"]["XXBTZUSD"].is_object());

        let ticker = &parsed["result"]["XXBTZUSD"];
        assert_eq!(ticker["c"][0], "50000.00000");

        println!("✅ Ticker parsing works correctly");
    }

    #[test]
    fn test_parse_position_response() {
        let mock_response = r#"{
            "error": [],
            "result": {
                "ABCDEF-GHIJK-LMNOP": {
                    "ordertxid": "OABCD-EFGHI-JKLMNO",
                    "posstatus": "open",
                    "pair": "XBTUSD",
                    "time": 1234567890.1234,
                    "type": "buy",
                    "ordertype": "limit",
                    "cost": "1000.00",
                    "fee": "2.50",
                    "vol": "0.02000000",
                    "vol_closed": "0.00000000",
                    "margin": "100.00",
                    "value": "1050.00",
                    "net": "47.50"
                }
            }
        }"#;

        let parsed: serde_json::Value = serde_json::from_str(mock_response).unwrap();

        assert!(parsed["error"].as_array().unwrap().is_empty());
        assert!(parsed["result"].is_object());

        let position = &parsed["result"]["ABCDEF-GHIJK-LMNOP"];
        assert_eq!(position["net"], "47.50");
        assert_eq!(position["pair"], "XBTUSD");

        println!("✅ Position parsing works correctly");
    }
}
