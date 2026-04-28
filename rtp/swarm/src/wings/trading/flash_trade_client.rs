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

// ---- REST API Client ----

/// Flash Trade REST API client (queries only, no execution).
pub struct FlashTradeClient {
    client: reqwest::blocking::Client,
    base_url: String,
}

impl Default for FlashTradeClient {
    fn default() -> Self {
        Self::new()
    }
}

impl FlashTradeClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::blocking::Client::new()),
            base_url: FLASH_API_BASE.to_string(),
        }
    }

    /// Get all available markets.
    pub fn get_markets(&self) -> Result<Vec<FlashMarket>, String> {
        let url = format!("{}/raw/markets", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .map_err(|e| format!("Flash API request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Flash API returned status {}", resp.status()));
        }

        resp.json::<Vec<FlashMarket>>()
            .map_err(|e| format!("Failed to parse markets response: {}", e))
    }

    /// Get current oracle prices for all assets.
    pub fn get_prices(&self) -> Result<Vec<FlashPrice>, String> {
        let url = format!("{}/prices", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .map_err(|e| format!("Flash API request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Flash API returned status {}", resp.status()));
        }

        resp.json::<Vec<FlashPrice>>()
            .map_err(|e| format!("Failed to parse prices response: {}", e))
    }

    /// Get the oracle price for a specific symbol (e.g., "SOL").
    pub fn get_price(&self, symbol: &str) -> Result<f64, String> {
        let prices = self.get_prices()?;
        prices
            .iter()
            .find(|p| p.symbol == symbol)
            .and_then(|p| p.oracle_price.parse::<f64>().ok())
            .ok_or_else(|| format!("Price not found for symbol: {}", symbol))
    }

    /// Get all positions for a given owner wallet address.
    pub fn get_positions(&self, owner: &str) -> Result<Vec<FlashPosition>, String> {
        let url = format!("{}/positions/owner/{}", self.base_url, owner);
        let resp = self
            .client
            .get(&url)
            .send()
            .map_err(|e| format!("Flash API request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Flash API returned status {}", resp.status()));
        }

        resp.json::<Vec<FlashPosition>>()
            .map_err(|e| format!("Failed to parse positions response: {}", e))
    }

    /// Get pool utilization data.
    pub fn get_pool_data(&self) -> Result<Vec<FlashPoolData>, String> {
        let url = format!("{}/pool-data", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .map_err(|e| format!("Flash API request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Flash API returned status {}", resp.status()));
        }

        resp.json::<Vec<FlashPoolData>>()
            .map_err(|e| format!("Failed to parse pool-data response: {}", e))
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
