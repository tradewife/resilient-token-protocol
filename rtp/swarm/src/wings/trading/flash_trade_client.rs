//! Flash Trade REST API client — queries only, no execution.
//!
//! Execution happens on-chain via CPI (invoke_signed from rtp-treasury program).
//! This module queries the Flash Trade REST API for:
//! - Market data, prices, pool utilization
//! - Position monitoring
//! - Trade previews
//!
//! All endpoints are public (no auth required).

use serde::{Deserialize, Serialize};

const FLASH_API_BASE: &str = "https://flashapi.trade";

// ---- REST API Response Types ----

/// Market info from GET /raw/markets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashMarket {
    pub pool: String,
    pub target_symbol: String,
    pub collateral_symbol: String,
    pub side: String, // "Long" or "Short"
    pub market_address: String,
    pub target_custody_address: String,
    pub collateral_custody_address: String,
    pub target_oracle_address: String,
    pub collateral_oracle_address: String,
    pub collateral_token_account: String,
    pub max_initial_leverage: f64,
    pub max_leverage: f64,
    #[serde(default)]
    pub allow_open_position: bool,
    #[serde(default)]
    pub allow_close_position: bool,
}

/// Price entry from GET /prices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashPrice {
    pub symbol: String,
    pub oracle_price: String,
    pub oracle_price_decimals: i32,
    pub oracle_confidence: String,
    pub oracle_delay: i64,
    pub pool: String,
    pub custody: String,
}

/// Position data from GET /positions/owner/{owner}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashPosition {
    pub position_address: String,
    pub owner: String,
    pub pool: String,
    pub market: String,
    pub side: String,
    pub size: String,
    pub size_usd: String,
    pub collateral: String,
    pub collateral_usd: String,
    pub entry_price: String,
    pub mark_price: String,
    pub liquidation_price: String,
    pub unrealized_pnl_usd: String,
    pub leverage: String,
    pub created_at: String,
}

/// Pool data from GET /pool-data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashPoolData {
    pub pool: String,
    pub aum_usd: String,
    pub utilization: String,
}

use std::sync::Mutex;

// ---- REST API Client ----

/// Flash Trade REST API client (queries only, no execution).
/// Includes price caching for graceful degradation when API is unavailable.
pub struct FlashTradeClient {
    client: reqwest::Client,
    base_url: String,
    max_retries: u32,
    /// Cached prices: symbol → (price, timestamp_secs).
    price_cache: Mutex<std::collections::HashMap<String, (f64, i64)>>,
}

impl Default for FlashTradeClient {
    fn default() -> Self {
        Self::new()
    }
}

impl FlashTradeClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            base_url: FLASH_API_BASE.to_string(),
            max_retries: 3,
            price_cache: Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Execute a GET request with retry logic.
    async fn get_with_retry<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let url = format!("{}{}", self.base_url, path);
        let mut last_err = String::new();
        for attempt in 0..=self.max_retries {
            match self.client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    return resp.json::<T>().await.map_err(|e| format!("Parse error: {}", e));
                }
                Ok(resp) => {
                    last_err = format!("Flash API returned status {}", resp.status());
                }
                Err(e) => {
                    last_err = format!("Flash API request failed: {}", e);
                }
            }
            if attempt < self.max_retries {
                tokio::time::sleep(std::time::Duration::from_millis(500 * (attempt + 1) as u64)).await;
            }
        }
        Err(last_err)
    }

    /// Get all available markets.
    pub async fn get_markets(&self) -> Result<Vec<FlashMarket>, String> {
        self.get_with_retry("/raw/markets").await
    }

    /// Get current oracle prices for all assets.
    pub async fn get_prices(&self) -> Result<Vec<FlashPrice>, String> {
        self.get_with_retry("/prices").await
    }

    /// Get the oracle price for a specific symbol (e.g., "SOL").
    /// Caches the result for graceful degradation when API is unavailable.
    pub async fn get_price(&self, symbol: &str) -> Result<f64, String> {
        match self.get_prices().await {
            Ok(prices) => {
                if let Some(price) = prices
                    .iter()
                    .find(|p| p.symbol == symbol)
                    .and_then(|p| p.oracle_price.parse::<f64>().ok())
                {
                    // Cache the successful result
                    let ts = chrono::Utc::now().timestamp();
                    if let Ok(mut cache) = self.price_cache.lock() {
                        cache.insert(symbol.to_string(), (price, ts));
                    }
                    Ok(price)
                } else {
                    Err(format!("Price not found for symbol: {}", symbol))
                }
            }
            Err(api_err) => {
                // API failed — try cached price
                if let Ok(cache) = self.price_cache.lock() {
                    if let Some((cached_price, ts)) = cache.get(symbol) {
                        tracing::warn!(
                            "[FlashTradeClient] API failed for {}, using cached price (${:.2} from {}s ago): {}",
                            symbol,
                            cached_price,
                            chrono::Utc::now().timestamp() - ts,
                            api_err
                        );
                        return Ok(*cached_price);
                    }
                }
                Err(format!("Price unavailable for {} (API: {}, no cache)", symbol, api_err))
            }
        }
    }

    /// Get all positions for a given owner wallet address.
    pub async fn get_positions(&self, owner: &str) -> Result<Vec<FlashPosition>, String> {
        self.get_with_retry(&format!("/positions/owner/{}", owner)).await
    }

    /// Get pool utilization data.
    pub async fn get_pool_data(&self) -> Result<Vec<FlashPoolData>, String> {
        self.get_with_retry("/pool-data").await
    }

    /// Blocking wrapper: get all available markets.
    pub fn get_markets_blocking(&self) -> Result<Vec<FlashMarket>, String> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("tokio runtime: {}", e))?;
        rt.block_on(self.get_markets())
    }

    /// Blocking wrapper: get current oracle prices for all assets.
    pub fn get_prices_blocking(&self) -> Result<Vec<FlashPrice>, String> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("tokio runtime: {}", e))?;
        rt.block_on(self.get_prices())
    }

    /// Blocking wrapper: get the oracle price for a specific symbol.
    pub fn get_price_blocking(&self, symbol: &str) -> Result<f64, String> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("tokio runtime: {}", e))?;
        rt.block_on(self.get_price(symbol))
    }

    /// Blocking wrapper: get all positions for a given owner wallet address.
    pub fn get_positions_blocking(&self, owner: &str) -> Result<Vec<FlashPosition>, String> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("tokio runtime: {}", e))?;
        rt.block_on(self.get_positions(owner))
    }

    /// Blocking wrapper: get pool utilization data.
    pub fn get_pool_data_blocking(&self) -> Result<Vec<FlashPoolData>, String> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("tokio runtime: {}", e))?;
        rt.block_on(self.get_pool_data())
    }

    /// Expected Flash Trade program version (for discriminator compatibility).
    /// If Flash Trade upgrades and changes discriminators, CPI calls silently fail.
    /// This method queries the REST API as a lightweight health check.
    pub async fn check_program_health(&self) -> Result<String, String> {
        // Query markets as a liveness check — if the API responds, the program
        // is active. Discriminator version can't be checked directly, but API
        // availability is a strong signal.
        let markets = self.get_markets().await?;
        Ok(format!(
            "Flash Trade API healthy: {} markets available",
            markets.len()
        ))
    }
}

// ---- Flash Trade CPI Account Derivation ----

/// Pre-computed Flash Trade account addresses for a specific market.
///
/// These are derived offline using the same PDA seeds as Flash Trade's SDK.
/// The Rust agent holds this struct and passes the addresses as instruction
/// parameters when calling `open_flash_position` / `close_flash_position`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashTradeAccounts {
    // Program IDs
    pub program_id: String,
    pub composability_program_id: String,

    // Core PDAs
    pub perpetuals_pda: String,
    pub transfer_authority: String,
    pub event_authority: String,

    // Pool/Custody/Market
    pub pool_address: String,
    pub target_custody: String,
    pub target_oracle: String,
    pub collateral_custody: String,
    pub collateral_oracle: String,
    pub collateral_custody_token_account: String,
    pub market_address: String,

    // Position PDA (derived per owner + market)
    pub position_pda: String,
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flash_trade_client_creation() {
        let client = FlashTradeClient::new();
        assert_eq!(client.base_url, "https://flashapi.trade");
    }

    #[test]
    fn flash_trade_accounts_serialization() {
        let accounts = FlashTradeAccounts {
            program_id: "FLASH6Lo6h3iasJKWDs2F8TkW2UKf3s15C8PMGuVfgBn".to_string(),
            composability_program_id: "FSWAPViR8ny5K96hezav8jynVubP2dJ2L7SbKzds2hwm".to_string(),
            perpetuals_pda: "7DWCtB5Z8rPiyBMKUwqyC95R9tJpbhoQhLM9LbK3Z5QZ".to_string(),
            transfer_authority: "81xGAvJ27ZeRThU2JEfKAUeT4Fx6qCCd8WHZpujZbiiG".to_string(),
            event_authority: "9qb3KAyARHqhVGQjJmzSVJ1hTm3KDR2QL8EBW5paXkUB".to_string(),
            pool_address: "HfF7GCcEc76xubFCHLLXRdYcgRzwjEPdfKWqzRS8Ncog".to_string(),
            target_custody: "BjzZ33nMnbXZ7rw3Uy9Uu1W7BDCzzugqkiZoamJHRKF7".to_string(),
            target_oracle: "DXqtMo8qRBfHcK11kBnSaCSXkWKk1huMf94R6sAxLHtf".to_string(),
            collateral_custody: "BjzZ33nMnbXZ7rw3Uy9Uu1W7BDCzzugqkiZoamJHRKF7".to_string(),
            collateral_oracle: "DXqtMo8qRBfHcK11kBnSaCSXkWKk1huMf94R6sAxLHtf".to_string(),
            collateral_custody_token_account: "Hhed3wTHoVoPpnuBntGf236UfowMMAXfxqTLkMyJJENe"
                .to_string(),
            market_address: "3vHoXbUvGhEHFsLUmxyC6VWsbYDreb1zMn9TAp5ijN5K".to_string(),
            position_pda: String::default(),
        };
        let json = serde_json::to_string(&accounts).unwrap();
        assert!(json.contains("FLASH6Lo6h3iasJKWDs2F8TkW2UKf3s15C8PMGuVfgBn"));
    }
}
