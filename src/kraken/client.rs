use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256, Sha512};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{AccountBalance, AssetPairInfo, KrakenResponse, OHLCResponse, OpenPositions, Position, TickerData, TickerInfo};

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
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        nonce.to_string()
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
        let mut all_params = params.clone();
        all_params.insert("nonce".to_string(), nonce.clone());

        let postdata = all_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

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

        let kraken_response: KrakenResponse<T> = response
            .json()
            .await
            .context("Failed to parse response")?;

        if !kraken_response.error.is_empty() {
            anyhow::bail!("Kraken API error: {:?}", kraken_response.error);
        }

        kraken_response
            .result
            .context("No result in response")
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

        let kraken_response: KrakenResponse<T> = response
            .json()
            .await
            .context("Failed to parse response")?;

        if !kraken_response.error.is_empty() {
            anyhow::bail!("Kraken API error: {:?}", kraken_response.error);
        }

        kraken_response
            .result
            .context("No result in response")
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
                "XL1" => "FLR".to_string(),          // Flare
                "XXRP" => "XRP".to_string(),         // XRP (already has XX prefix)
                "SOSO" => "SOSO".to_string(),        // Keep as-is
                "KTA" => "KTA".to_string(),          // Katana
                "KGEN" => "KGEN".to_string(),        // Keep as-is  
                "SNX" => "SNX".to_string(),          // Synthetix
                "REPV2" => "REPV2".to_string(),      // Augur v2
                "FET" => "FET".to_string(),          // Fetch.ai
                "TAO" => "TAO".to_string(),          // Bittensor
                _ => {
                    if asset.contains("03.S") {
                        asset.replace("03.S", "")  // SOL03.S -> SOL
                    } else if asset.contains("21.S") {
                        asset.replace("21.S", "")  // ATOM21.S -> ATOM
                    } else {
                        // Remove common suffixes
                        asset
                            .trim_end_matches(".F")   // Staked/Flex
                            .trim_end_matches(".B")   // Bonded  
                            .trim_end_matches(".S")   // Staked
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
                    let pair_base_normalized = pair_info.base
                        .trim_start_matches('X')
                        .trim_start_matches('Z');
                    
                    // Check various matching patterns
                    let matches = 
                        pair_info.base == *asset ||                           // Exact match
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
