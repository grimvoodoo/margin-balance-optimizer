use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256, Sha512};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{
    AccountBalance, AssetPairInfo, KrakenResponse, OpenPositions, Position, TickerData, TickerInfo,
};

type HmacSha512 = Hmac<Sha512>;

const KRAKEN_API_URL: &str = "https://api.kraken.com";

#[derive(Clone)]
pub struct KrakenClient {
    api_key: String,
    private_key: Vec<u8>,
    client: reqwest::Client,
}

impl KrakenClient {
    pub fn new(api_key: String, private_key_b64: String) -> Result<Self> {
        let private_key = general_purpose::STANDARD
            .decode(private_key_b64)
            .context("Failed to decode private key")?;

        Ok(Self {
            api_key,
            private_key,
            client: reqwest::Client::new(),
        })
    }

    fn get_nonce(&self) -> String {
        // Use microseconds and atomic monotonic fallback to prevent nonce collisions
        // under concurrent API calls
        static LAST_NONCE: AtomicU64 = AtomicU64::new(0);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        let mut prev = LAST_NONCE.load(Ordering::Relaxed);
        loop {
            // Ensure strictly monotonic: use timestamp if newer, otherwise increment
            let candidate = if now > prev { now } else { prev + 1 };
            match LAST_NONCE.compare_exchange(prev, candidate, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => return candidate.to_string(),
                Err(p) => prev = p,
            }
        }
    }

    fn sign_request(&self, urlpath: &str, nonce: &str, postdata: &str) -> String {
        // Create SHA256 hash of nonce + postdata
        let mut sha256 = Sha256::new();
        sha256.update(format!("{}{}", nonce, postdata));
        let hash = sha256.finalize();

        // Create message: urlpath + hash
        let mut message = urlpath.as_bytes().to_vec();
        message.extend_from_slice(&hash);

        // Create HMAC-SHA512 signature
        let mut mac = HmacSha512::new_from_slice(&self.private_key).unwrap();
        mac.update(&message);
        let result = mac.finalize();

        general_purpose::STANDARD.encode(result.into_bytes())
    }

    async fn private_request<T: for<'de> serde::Deserialize<'de>>(
        &self,
        endpoint: &str,
        params: &HashMap<String, String>,
    ) -> Result<T> {
        let nonce = self.get_nonce();

        // Use BTreeMap for deterministic ordering and proper URL encoding
        let mut all_params = BTreeMap::new();
        all_params.extend(params.iter().map(|(k, v)| (k.clone(), v.clone())));
        all_params.insert("nonce".to_string(), nonce.clone());

        let postdata =
            serde_urlencoded::to_string(&all_params).expect("Failed to form-encode params");

        let urlpath = format!("/0/private/{}", endpoint);
        let signature = self.sign_request(&urlpath, &nonce, &postdata);

        let url = format!("{}{}", KRAKEN_API_URL, urlpath);

        let response = self
            .client
            .post(&url)
            .header("API-Key", &self.api_key)
            .header("API-Sign", signature)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(postdata)
            .send()
            .await
            .context("Failed to send request")?;

        let kraken_response: KrakenResponse<T> =
            response.json().await.context("Failed to parse response")?;

        if !kraken_response.error.is_empty() {
            anyhow::bail!("Kraken API error: {:?}", kraken_response.error);
        }

        kraken_response.result.context("No result in response")
    }

    async fn public_request<T: for<'de> serde::Deserialize<'de>>(
        &self,
        endpoint: &str,
        params: &HashMap<String, String>,
    ) -> Result<T> {
        let url = format!("{}/0/public/{}", KRAKEN_API_URL, endpoint);

        let response = self
            .client
            .get(&url)
            .query(params)
            .send()
            .await
            .context("Failed to send request")?;

        let kraken_response: KrakenResponse<T> =
            response.json().await.context("Failed to parse response")?;

        if !kraken_response.error.is_empty() {
            anyhow::bail!("Kraken API error: {:?}", kraken_response.error);
        }

        kraken_response.result.context("No result in response")
    }

    pub async fn get_open_positions(&self) -> Result<HashMap<String, Position>> {
        let mut params = HashMap::new();
        params.insert("docalcs".to_string(), "true".to_string());
        let result: OpenPositions = self.private_request("OpenPositions", &params).await?;
        Ok(result.positions)
    }

    pub async fn get_ticker(&self, pairs: Vec<String>) -> Result<HashMap<String, TickerData>> {
        let mut params = HashMap::new();
        params.insert("pair".to_string(), pairs.join(","));

        let result: TickerInfo = self.public_request("Ticker", &params).await?;
        Ok(result.pairs)
    }

    pub async fn get_balance(&self) -> Result<HashMap<String, String>> {
        let params = HashMap::new();
        let result: AccountBalance = self.private_request("Balance", &params).await?;
        Ok(result.balances)
    }

    /// Get all available asset pairs from Kraken
    pub async fn get_asset_pairs(&self) -> Result<AssetPairInfo> {
        let params = HashMap::new();
        let result: AssetPairInfo = self.public_request("AssetPairs", &params).await?;
        Ok(result)
    }

    /// Find valid ticker pairs for given assets against a quote currency (e.g., "GBP" or "EUR")
    /// Will try GBP first, then fall back to EUR and USD if no GBP pair exists
    pub async fn find_pairs_for_assets(
        &self,
        assets: &[String],
        quote_currency: &str,
    ) -> Result<HashMap<String, String>> {
        let asset_pairs = self.get_asset_pairs().await?;
        let mut asset_to_pair = HashMap::new();

        // Try multiple quote currencies in order of preference
        let quote_currencies = vec![quote_currency, "EUR", "USD"];

        for asset in assets {
            // Normalize asset name by removing common suffixes and prefixes
            // Handle special cases first (known asset name mappings)
            let mut base_asset = match asset.as_str() {
                "U" => "USDT".to_string(),
                "XL1" => "FLR".to_string(),     // Flare
                "XXRP" => "XRP".to_string(),    // XRP (already has XX prefix)
                "SOSO" => "SOSO".to_string(),   // Keep as-is
                "KTA" => "KTA".to_string(),     // Katana
                "KGEN" => "KGEN".to_string(),   // Keep as-is
                "SNX" => "SNX".to_string(),     // Synthetix
                "REPV2" => "REPV2".to_string(), // Augur v2
                "FET" => "FET".to_string(),     // Fetch.ai
                "TAO" => "TAO".to_string(),     // Bittensor
                _ => {
                    if asset.contains("03.S") {
                        asset.replace("03.S", "") // SOL03.S -> SOL
                    } else if asset.contains("21.S") {
                        asset.replace("21.S", "") // ATOM21.S -> ATOM
                    } else {
                        // Remove common suffixes
                        asset
                            .trim_end_matches(".F") // Staked/Flex
                            .trim_end_matches(".B") // Bonded
                            .trim_end_matches(".S") // Staked
                            .to_string()
                    }
                }
            };

            // Now remove X/Z prefixes
            base_asset = base_asset
                .trim_start_matches('X')
                .trim_start_matches('Z')
                .to_string();

            // Try to find a pair for this asset, trying each quote currency in order
            let mut found = false;
            for quote_curr in &quote_currencies {
                if found {
                    break;
                }

                for (pair_name, pair_info) in &asset_pairs.pairs {
                    if !pair_info.quote.contains(*quote_curr) {
                        continue;
                    }

                    // Normalize the pair's base asset for comparison
                    let pair_base_normalized = pair_info
                        .base
                        .trim_start_matches('X')
                        .trim_start_matches('Z');

                    // Check various matching patterns
                    let matches = pair_info.base == *asset ||                           // Exact match
                        pair_info.base == base_asset ||                       // Match without suffixes
                        pair_base_normalized == base_asset ||                 // Normalized match
                        pair_info.base.trim_start_matches('X') == asset.trim_start_matches('X') ||
                        pair_base_normalized == asset.trim_start_matches('X').trim_start_matches('Z');

                    if matches {
                        asset_to_pair.insert(asset.clone(), pair_name.clone());
                        found = true;
                        break;
                    }
                }
            }
        }

        Ok(asset_to_pair)
    }

    /// Get OHLC (candlestick) data for a pair
    /// Returns raw JSON for flexible parsing
    pub async fn get_ohlc(&self, pair: &str, interval: Option<u32>) -> Result<serde_json::Value> {
        let mut params = HashMap::new();
        params.insert("pair".to_string(), pair.to_string());
        params.insert("interval".to_string(), interval.unwrap_or(1440).to_string());

        let url = format!("{}/0/public/OHLC", "https://api.kraken.com");
        let response = self.client.get(&url).query(&params).send().await?;
        let json: serde_json::Value = response.json().await?;
        Ok(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_kraken_client_valid_key() {
        // Test creating a client with a valid base64 private key
        let api_key = "test_api_key".to_string();
        let private_key = general_purpose::STANDARD.encode(b"test_secret_key");

        let client = KrakenClient::new(api_key.clone(), private_key);
        assert!(client.is_ok());

        let client = client.unwrap();
        assert_eq!(client.api_key, "test_api_key");
    }

    #[test]
    fn test_new_kraken_client_invalid_base64() {
        // Test that invalid base64 returns an error
        let api_key = "test_api_key".to_string();
        let invalid_private_key = "not!valid!base64!!!".to_string();

        let client = KrakenClient::new(api_key, invalid_private_key);
        assert!(client.is_err());
    }

    #[test]
    fn test_get_nonce() {
        // Test that nonce is always increasing
        let api_key = "test".to_string();
        let private_key = general_purpose::STANDARD.encode(b"test");
        let client = KrakenClient::new(api_key, private_key).unwrap();

        let nonce1 = client.get_nonce();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let nonce2 = client.get_nonce();

        let n1: u128 = nonce1.parse().unwrap();
        let n2: u128 = nonce2.parse().unwrap();
        assert!(n2 > n1, "Nonce should be strictly increasing");
    }

    #[test]
    fn test_sign_request() {
        // Test that signing produces consistent results for same input
        let api_key = "test_key".to_string();
        let private_key = general_purpose::STANDARD.encode(b"test_secret");
        let client = KrakenClient::new(api_key, private_key).unwrap();

        let urlpath = "/0/private/Balance";
        let nonce = "1234567890";
        let postdata = "nonce=1234567890";

        let sig1 = client.sign_request(urlpath, nonce, postdata);
        let sig2 = client.sign_request(urlpath, nonce, postdata);

        // Same inputs should produce same signature
        assert_eq!(sig1, sig2);

        // Signature should not be empty
        assert!(!sig1.is_empty());

        // Different nonce should produce different signature
        let sig3 = client.sign_request(urlpath, "9999999999", postdata);
        assert_ne!(sig1, sig3);
    }

    #[test]
    fn test_sign_request_different_endpoints() {
        // Test that different endpoints produce different signatures
        let api_key = "test_key".to_string();
        let private_key = general_purpose::STANDARD.encode(b"test_secret");
        let client = KrakenClient::new(api_key, private_key).unwrap();

        let nonce = "1234567890";
        let postdata = "nonce=1234567890";

        let sig_balance = client.sign_request("/0/private/Balance", nonce, postdata);
        let sig_positions = client.sign_request("/0/private/OpenPositions", nonce, postdata);

        assert_ne!(sig_balance, sig_positions);
    }
}
