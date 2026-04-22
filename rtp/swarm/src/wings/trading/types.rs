//! Trading Wing type definitions.
//!
//! Extracted from `mod.rs` for reuse and organization. These types represent
//! Hyperliquid signing structures, yield reporting, strategy configuration,
//! and position tracking.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

//  Hyperliquid Signing Types

/// ECDSA signature components for Hyperliquid EIP-712 signing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlSignature {
    pub r: String,
    pub s: String,
    pub v: u64,
}

/// Key file structure for the HL testnet ETH keypair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlKeyFile {
    pub address: String,
    pub private_key: String,
    pub network: String,
}

//  Yield Reporting

/// Yield report data emitted after a confirmed Hyperliquid fill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldReportData {
    pub symbol: String,
    pub side: String,
    pub fill_price: String,
    pub size: String,
    /// Entry price from the opening fill. Stored for PnL calculation on close.
    pub entry_price: Option<String>,
    /// Realized PnL in USDC. `None` means the position is still open (no PnL
    /// realized yet). `Some(value)` means the position was closed and this is
    /// the actual realized profit/loss.
    /// For opening fills: `(fill_price - entry_price) * size` is meaningless
    /// because entry_price == fill_price, so PnL = None.
    /// For closing fills: `(exit_price - entry_price) * size` for longs,
    /// `(entry_price - exit_price) * size` for shorts.
    pub realized_pnl_usdc: Option<f64>,
    pub timestamp: String,
}

//  Strategy Configuration

/// Active strategy configuration for the Trading Wing.
///
/// Default values are SOL/USDT Survivor 2.69 — confirmed Apr 12 cycle report, OOS Sharpe 3.96.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub signal_threshold: f64,
    pub tp_atr: f64,
    pub sl_atr: f64,
    pub max_hold_hours: f64,
    pub trailing_stop_atr: f64,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        // SOL/USDT Survivor 2.69 — confirmed Apr 12 cycle report, OOS Sharpe 3.96
        Self {
            signal_threshold: 0.3,
            tp_atr: 3.0,
            sl_atr: 1.5,
            max_hold_hours: 36.0,
            trailing_stop_atr: 0.5,
        }
    }
}

//  Position Tracking

/// Tracks an open position in the Trading Wing's in-memory state.
///
/// Created when an opening fill is confirmed. Consumed when the position
/// is closed, at which point realized PnL is calculated:
///   - Long close:  `(fill_price - entry_price) * size`
///   - Short close: `(entry_price - fill_price) * size`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionState {
    pub symbol: String,
    pub side: String,
    pub entry_price: f64,
    pub size: f64,
    pub opened_at: String,
}

impl PositionState {
    /// Calculate realized PnL when closing this position.
    ///
    /// `close_price` is the fill price of the closing order.
    /// `close_size` is the size being closed (may be partial).
    pub fn realized_pnl(&self, close_price: f64, close_size: f64) -> f64 {
        match self.side.as_str() {
            "BUY" => (close_price - self.entry_price) * close_size,
            "SELL" => (self.entry_price - close_price) * close_size,
            _ => 0.0,
        }
    }
}

//  Per-Token Wallet Mapping

/// In-memory trading state for the Trading Wing.
///
/// Tracks per-token derivation indices for Phantom MCP wallet isolation.
/// Each registered token gets its own `derivationIndex` — yielding a
/// separate Solana address, EVM address, and Hyperliquid account.
///
/// Index 0 is the default agent wallet. Tokens are assigned 1, 2, 3, ...
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingState {
    /// Maps token mint (base58) → Phantom derivation index.
    pub token_wallet_map: HashMap<String, u32>,
    /// Next available derivation index for a new token.
    pub next_derivation_index: u32,
    /// Last proposal received by the wing.
    pub last_proposal: Option<serde_json::Value>,
    /// Execution count.
    pub execution_count: u64,
    /// Open positions by symbol.
    pub open_positions: HashMap<String, PositionState>,
}

impl Default for TradingState {
    fn default() -> Self {
        Self {
            token_wallet_map: HashMap::new(),
            next_derivation_index: 1, // 0 is the default agent wallet
            last_proposal: None,
            execution_count: 0,
            open_positions: HashMap::new(),
        }
    }
}

impl TradingState {
    /// Assign a derivation index for a new token mint.
    /// Returns the assigned index and increments the counter.
    pub fn assign_derivation_index(&mut self, mint: &str) -> u32 {
        let di = self.next_derivation_index;
        self.token_wallet_map.insert(mint.to_string(), di);
        self.next_derivation_index += 1;
        di
    }

    /// Look up the derivation index for a token mint.
    /// Returns 0 (default wallet) if not found.
    pub fn derivation_index_for(&self, mint: &str) -> u32 {
        self.token_wallet_map.get(mint).copied().unwrap_or(0)
    }
}
