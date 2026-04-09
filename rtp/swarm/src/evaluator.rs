//! Evaluator — protocol-level survival scoring.
//!
//! The evaluator defines what "winning" means for the agent swarm.
//! It computes the Treasury Survival Index (TSI), a single scalar that
//! drives memory promotion, heartbeat triggers, stagnation detection,
//! and improvement claims.
//!
//! Spec: EVALUATOR.md
//!
//! ## Layers
//!
//! - **Assessor** (`wings/evolve/assessor.rs`): scores individual wings
//! - **Evaluator** (this file): scores the protocol as a whole
//!
//! The evaluator consumes on-chain treasury state + bridge outputs and
//! produces TSI + secondary metrics. It does NOT enforce hard constraints
//! (that's the Anchor program and Soulguard) — it checks that enforcement
//! is working and flags terminal states.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Hard cap on drawdown — a strategy hitting 20% is failed, not degraded.
/// Matches soulcontract Core Value #5: "yield generation must never risk
/// the principal beyond defined risk budgets."
pub const MAX_DRAWDOWN: f64 = 0.20;

/// Default stagnation threshold: number of consecutive flat/declining TSI
/// readings before the redirect heartbeat fires. Overridable at construction.
pub const DEFAULT_STAGNATION_THRESHOLD: usize = 3;

/// Default consecutive zero-TSI readings before terminal state is declared.
pub const DEFAULT_TERMINAL_ZERO_COUNT: usize = 2;

/// Default consecutive bridge failures before entering safe mode.
pub const DEFAULT_BRIDGE_FAILURE_LIMIT: usize = 6;

// ---------------------------------------------------------------------------
// Input structs
// ---------------------------------------------------------------------------

/// On-chain treasury state, read via `getAccountInfo(treasury_pda)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnChainState {
    pub vault_balance: u64,
    pub total_fees_withdrawn: u64,
    pub total_distributed_holders: u64,
    pub total_distributed_dev: u64,
    pub total_distributed_ecosystem: u64,
    pub total_hydration: u64,
    pub phase: ProtocolPhase,
    pub min_runway_balance: u64,
}

/// Protocol phase — mirrors the Anchor program's `Phase` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolPhase {
    Sustenance,
    Ecosystem,
    Humanity,
}

/// Off-chain metrics from the Python fractal-swarm bridge response.
/// Available only when a strategy has actually run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeMetrics {
    pub yield_estimate: f64,
    pub confidence: f64,
    pub consistency: f64,
    pub folds_validated: u32,
    pub strategy: String,
    pub max_drawdown: f64,
}

/// Optional price oracle for converting vault balance to USDC.
/// If `None`, the evaluator uses a 1:1 assumption and flags `oracle_degraded`.
#[derive(Debug, Clone, Copy)]
pub struct PriceOracle {
    /// USDC value per native token (e.g. 1.23 means 1 token ≈ $1.23 USDC).
    pub price_usdc: f64,
}

// ---------------------------------------------------------------------------
// Output struct
// ---------------------------------------------------------------------------

/// Complete evaluation result — everything the dashboard and downstream
/// systems need from a single evaluation cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evaluation {
    /// Treasury Survival Index — the primary scalar score.
    pub tsi: f64,

    /// Individual factors that compose TSI.
    pub growth_factor: f64,
    pub safety_factor: f64,
    pub reliability_factor: f64,

    /// True if this evaluation used degraded inputs (bridge offline,
    /// no price oracle, etc.). Dashboard should render differently.
    pub score_degraded: bool,

    /// True if no price oracle was available — vault balance treated as 1:1.
    pub oracle_degraded: bool,

    /// True if bridge metrics were unavailable — on-chain-only score.
    pub bridge_degraded: bool,

    /// Secondary metrics (not composited into TSI).
    pub secondary: SecondaryMetrics,

    /// Timestamp of this evaluation.
    pub evaluated_at: DateTime<Utc>,
}

/// Secondary metrics tracked and dashboarded independently.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecondaryMetrics {
    /// Treasury NAV in USDC-equivalent (or native if oracle degraded).
    pub treasury_nav: f64,
    /// Fee accumulation rate: delta_fees / delta_time (fees per second).
    pub fee_rate: f64,
    /// What fraction of withdrawn fees reached beneficiaries.
    pub distribution_efficiency: f64,
    /// What fraction of withdrawn fees funded the swarm.
    pub hydration_ratio: f64,
    /// Vault balance / min_runway_balance. Below 1.0 = danger.
    pub price_floor_distance: f64,
    /// Current protocol phase.
    pub phase: ProtocolPhase,
    /// Bridge yield estimate (annualized), if available.
    pub strategy_yield: Option<f64>,
    /// Bridge strategy name, if available.
    pub strategy_name: Option<String>,
    /// Bridge fold count, if available.
    pub folds_validated: Option<u32>,
}

// ---------------------------------------------------------------------------
// Stagnation / failure tracking
// ---------------------------------------------------------------------------

/// Result of stagnation and terminal-state checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// True if TSI has not improved for `stagnation_threshold` consecutive cycles.
    pub stagnant: bool,
    /// True if TSI has been zero for `terminal_zero_count` consecutive cycles.
    pub terminal: bool,
    /// True if the bridge has been unreachable for `bridge_failure_limit` cycles.
    pub bridge_failed: bool,
    /// Recent TSI history (most recent last).
    pub tsi_history: Vec<f64>,
    /// Number of consecutive zero-TSI readings.
    pub consecutive_zeros: usize,
    /// Number of consecutive bridge failures.
    pub consecutive_bridge_failures: usize,
}

// ---------------------------------------------------------------------------
// Evaluator
// ---------------------------------------------------------------------------

/// The protocol-level evaluator.
///
/// Construction is configurable:
/// - `stagnation_threshold`: how many flat/declining TSI readings trigger redirect
/// - `terminal_zero_count`: how many consecutive zero-TSI readings = terminal
/// - `bridge_failure_limit`: how many consecutive bridge failures = safe mode
/// - `price_oracle`: USDC price feed, if available
pub struct Evaluator {
    /// Number of consecutive non-improving TSI readings to trigger redirect.
    stagnation_threshold: usize,
    /// Number of consecutive zero-TSI readings to declare terminal state.
    terminal_zero_count: usize,
    /// Number of consecutive bridge failures before declaring bridge_failed.
    bridge_failure_limit: usize,
    /// Optional price oracle for USDC conversion.
    price_oracle: Option<PriceOracle>,
    /// TSI history for trend detection.
    tsi_history: Vec<f64>,
    /// Previous on-chain state for delta computation.
    prev_state: Option<OnChainState>,
    /// Timestamp of previous evaluation.
    prev_evaluated_at: Option<DateTime<Utc>>,
    /// Consecutive zero-TSI count.
    consecutive_zeros: usize,
    /// Consecutive bridge failure count.
    consecutive_bridge_failures: usize,
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new(DEFAULT_STAGNATION_THRESHOLD)
    }
}

impl Evaluator {
    /// Create a new evaluator with a configurable stagnation threshold.
    ///
    /// For the hackathon demo: set to 2 or 3 to make redirects visible quickly.
    /// For production: tune based on heartbeat interval and market volatility.
    pub fn new(stagnation_threshold: usize) -> Self {
        Self {
            stagnation_threshold,
            terminal_zero_count: DEFAULT_TERMINAL_ZERO_COUNT,
            bridge_failure_limit: DEFAULT_BRIDGE_FAILURE_LIMIT,
            price_oracle: None,
            tsi_history: Vec::new(),
            prev_state: None,
            prev_evaluated_at: None,
            consecutive_zeros: 0,
            consecutive_bridge_failures: 0,
        }
    }

    /// Create with all parameters configurable.
    pub fn with_config(
        stagnation_threshold: usize,
        terminal_zero_count: usize,
        bridge_failure_limit: usize,
        price_oracle: Option<PriceOracle>,
    ) -> Self {
        Self {
            stagnation_threshold,
            terminal_zero_count,
            bridge_failure_limit,
            price_oracle,
            tsi_history: Vec::new(),
            prev_state: None,
            prev_evaluated_at: None,
            consecutive_zeros: 0,
            consecutive_bridge_failures: 0,
        }
    }

    /// Set or update the price oracle at runtime.
    pub fn set_oracle(&mut self, oracle: PriceOracle) {
        self.price_oracle = Some(oracle);
    }

    /// Clear the price oracle (e.g. on oracle failure).
    pub fn clear_oracle(&mut self) {
        self.price_oracle = None;
    }

    // ── Core scoring ──────────────────────────────────────────────────

    /// Compute the Treasury Survival Index.
    ///
    /// Evaluation order matters:
    /// 1. Safety check first — if runway is breached, short-circuit to 0.
    /// 2. Growth — distance above the runway floor (log-compressed).
    /// 3. Reliability — strategy robustness from bridge outputs.
    /// 4. TSI = growth × safety × reliability (zero-hardened).
    pub fn evaluate(
        &mut self,
        state: &OnChainState,
        bridge: Option<&BridgeMetrics>,
    ) -> Evaluation {
        let now = Utc::now();
        let oracle_degraded = self.price_oracle.is_none();

        // Convert vault balance to USDC-equivalent if oracle available.
        let vault_usdc = match self.price_oracle {
            Some(oracle) => state.vault_balance as f64 * oracle.price_usdc,
            None => state.vault_balance as f64,
        };
        let runway_usdc = match self.price_oracle {
            Some(oracle) => state.min_runway_balance as f64 * oracle.price_usdc,
            None => state.min_runway_balance as f64,
        };

        // ── Step 1: Safety (short-circuit if runway breached) ──────
        //
        // If vault is at or below the runway floor, the system cannot
        // pay for itself. TSI = 0 regardless of anything else.
        // This makes the hard constraint legible in code, not just spec.
        let (growth, safety, reliability, bridge_degraded) = if vault_usdc <= runway_usdc {
            // Short-circuit: runway breached. No need to compute further.
            (0.0, 0.0, 0.0, bridge.is_none())
        } else {
            // ── Step 2: Growth ──────────────────────────────────────
            // ln(vault / runway_floor) — log-compressed so a $200k vault
            // isn't 4× "better" than $50k in a way that drowns safety.
            let growth = (vault_usdc / runway_usdc).ln();

            // ── Step 3: Safety ──────────────────────────────────────
            // Drawdown from bridge if available, otherwise assume safe
            // (conservative: no penalty without evidence).
            let drawdown = bridge
                .map(|b| b.max_drawdown)
                .unwrap_or(0.0);
            let safety = (1.0 - drawdown / MAX_DRAWDOWN).clamp(0.0, 1.0);

            // ── Step 4: Reliability ─────────────────────────────────
            let (reliability, bridge_degraded) = match bridge {
                Some(b) => {
                    // Reset bridge failure counter on success.
                    self.consecutive_bridge_failures = 0;
                    (b.consistency * b.confidence, false)
                }
                None => {
                    // Degraded mode: use fee momentum as proxy.
                    self.consecutive_bridge_failures += 1;
                    let fee_momentum = self.compute_fee_momentum(state);
                    (fee_momentum, true)
                }
            };

            (growth, safety, reliability, bridge_degraded)
        };

        let tsi = growth * safety * reliability;

        // ── Secondary metrics ──────────────────────────────────────
        let fee_rate = self.compute_fee_rate(state, &now);
        let total_distributed = state.total_distributed_holders
            + state.total_distributed_dev
            + state.total_distributed_ecosystem;
        let distribution_efficiency = if state.total_fees_withdrawn > 0 {
            total_distributed as f64 / state.total_fees_withdrawn as f64
        } else {
            0.0
        };
        let hydration_ratio = if state.total_fees_withdrawn > 0 {
            state.total_hydration as f64 / state.total_fees_withdrawn as f64
        } else {
            0.0
        };
        let price_floor_distance = if runway_usdc > 0.0 {
            vault_usdc / runway_usdc
        } else {
            0.0
        };

        let secondary = SecondaryMetrics {
            treasury_nav: vault_usdc,
            fee_rate,
            distribution_efficiency,
            hydration_ratio,
            price_floor_distance,
            phase: state.phase,
            strategy_yield: bridge.map(|b| b.yield_estimate),
            strategy_name: bridge.map(|b| b.strategy.clone()),
            folds_validated: bridge.map(|b| b.folds_validated),
        };

        let score_degraded = oracle_degraded || bridge_degraded;

        // Update history.
        self.update_history(tsi, state, now);

        Evaluation {
            tsi,
            growth_factor: growth,
            safety_factor: safety,
            reliability_factor: reliability,
            score_degraded,
            oracle_degraded,
            bridge_degraded,
            secondary,
            evaluated_at: now,
        }
    }

    /// On-chain-only evaluation for when the bridge is down.
    ///
    /// Convenience wrapper that calls `evaluate` with `bridge: None`.
    pub fn evaluate_onchain(&mut self, state: &OnChainState) -> Evaluation {
        self.evaluate(state, None)
    }

    // ── Stagnation / terminal detection ────────────────────────────

    /// Check whether the protocol is stagnant, terminal, or bridge-failed.
    ///
    /// Must be called after `evaluate()` — reads internal history.
    pub fn health_check(&self) -> HealthCheck {
        HealthCheck {
            stagnant: self.is_stagnant(),
            terminal: self.is_terminal(),
            bridge_failed: self.consecutive_bridge_failures >= self.bridge_failure_limit,
            tsi_history: self.tsi_history.clone(),
            consecutive_zeros: self.consecutive_zeros,
            consecutive_bridge_failures: self.consecutive_bridge_failures,
        }
    }

    /// True if TSI has not improved for `stagnation_threshold` consecutive cycles.
    ///
    /// "Improved" means strictly greater than the previous reading.
    /// Three readings [1.20, 1.19, 1.18] → stagnant.
    /// Three readings [1.20, 1.19, 1.20] → not stagnant (third improved over second).
    fn is_stagnant(&self) -> bool {
        let n = self.stagnation_threshold;
        if self.tsi_history.len() < n {
            return false;
        }
        let recent: Vec<f64> = self.tsi_history.iter().rev().take(n).cloned().collect();
        // Stagnant if no reading is strictly greater than the one before it.
        // recent[0] is the newest, recent[n-1] is the oldest in the window.
        // We want: recent[0] <= recent[1] && recent[1] <= recent[2] && ...
        let mut stagnant = true;
        for i in 0..(n - 1) {
            if recent[i] > recent[i + 1] {
                stagnant = false;
                break;
            }
        }
        stagnant
    }

    /// True if TSI has been zero for `terminal_zero_count` consecutive cycles.
    fn is_terminal(&self) -> bool {
        self.consecutive_zeros >= self.terminal_zero_count
    }

    // ── Internal helpers ───────────────────────────────────────────

    /// Update TSI history and counters after an evaluation.
    fn update_history(&mut self, tsi: f64, state: &OnChainState, now: DateTime<Utc>) {
        // Update zero counter.
        if tsi == 0.0 {
            self.consecutive_zeros += 1;
        } else {
            self.consecutive_zeros = 0;
        }

        // Append to history (keep last 100 readings).
        self.tsi_history.push(tsi);
        if self.tsi_history.len() > 100 {
            self.tsi_history.remove(0);
        }

        // Store state for next delta computation.
        self.prev_state = Some(state.clone());
        self.prev_evaluated_at = Some(now);
    }

    /// Compute fee momentum as a reliability proxy when bridge is down.
    ///
    /// Returns 0.0–1.0: growing fees = system functioning, stagnant = degraded.
    fn compute_fee_momentum(&self, state: &OnChainState) -> f64 {
        match &self.prev_state {
            Some(prev) if prev.total_fees_withdrawn > 0 => {
                let delta = state
                    .total_fees_withdrawn
                    .saturating_sub(prev.total_fees_withdrawn);
                (delta as f64 / prev.total_fees_withdrawn as f64).min(1.0)
            }
            _ => 0.0,
        }
    }

    /// Compute fee rate (fees per second since last evaluation).
    fn compute_fee_rate(&self, state: &OnChainState, now: &DateTime<Utc>) -> f64 {
        match (&self.prev_state, self.prev_evaluated_at) {
            (Some(prev), Some(prev_time)) => {
                let delta_fees = state
                    .total_fees_withdrawn
                    .saturating_sub(prev.total_fees_withdrawn);
                let delta_secs = (*now - prev_time).num_seconds() as f64;
                if delta_secs > 0.0 {
                    delta_fees as f64 / delta_secs
                } else {
                    0.0
                }
            }
            _ => 0.0,
        }
    }

    /// Get the TSI history for external consumption.
    pub fn tsi_history(&self) -> &[f64] {
        &self.tsi_history
    }

    /// Get the current stagnation threshold.
    pub fn stagnation_threshold(&self) -> usize {
        self.stagnation_threshold
    }
}

// ---------------------------------------------------------------------------
// Standalone scoring function (for testing / embedding without state)
// ---------------------------------------------------------------------------

/// Compute TSI from raw inputs without maintaining state.
///
/// Useful for one-off calculations, backtesting, or embedding in other
/// systems that don't need the full Evaluator lifecycle.
pub fn compute_tsi(
    vault_balance: u64,
    min_runway_balance: u64,
    drawdown: f64,
    consistency: f64,
    confidence: f64,
) -> f64 {
    // Short-circuit: runway breached.
    if vault_balance <= min_runway_balance || min_runway_balance == 0 {
        return 0.0;
    }

    let growth = (vault_balance as f64 / min_runway_balance as f64).ln();
    let safety = (1.0 - drawdown / MAX_DRAWDOWN).clamp(0.0, 1.0);
    let reliability = consistency * confidence;

    growth * safety * reliability
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_state(vault_balance: u64) -> OnChainState {
        OnChainState {
            vault_balance,
            total_fees_withdrawn: 100_000,
            total_distributed_holders: 49_000,
            total_distributed_dev: 14_000,
            total_distributed_ecosystem: 7_000,
            total_hydration: 10_000,
            phase: ProtocolPhase::Sustenance,
            min_runway_balance: 10_000,
        }
    }

    fn good_bridge() -> BridgeMetrics {
        BridgeMetrics {
            yield_estimate: 118.3,
            confidence: 0.92,
            consistency: 0.78,
            folds_validated: 9,
            strategy: "mr_rsi_bb".to_string(),
            max_drawdown: 0.032,
        }
    }

    // ── Standalone compute_tsi ────────────────────────────────────────

    #[test]
    fn compute_tsi_healthy() {
        // vault=50k, runway=10k → ln(5) ≈ 1.609
        // drawdown=0.03 → safety = 1 - 0.03/0.20 = 0.85
        // reliability = 0.78 * 0.92 = 0.7176
        // TSI ≈ 1.609 * 0.85 * 0.7176 ≈ 0.981
        let tsi = compute_tsi(50_000, 10_000, 0.03, 0.78, 0.92);
        assert!((tsi - 0.981).abs() < 0.01, "TSI was {}", tsi);
    }

    #[test]
    fn compute_tsi_zero_at_runway() {
        // At runway floor → TSI = 0 (short-circuit).
        let tsi = compute_tsi(10_000, 10_000, 0.0, 1.0, 1.0);
        assert_eq!(tsi, 0.0);
    }

    #[test]
    fn compute_tsi_zero_below_runway() {
        let tsi = compute_tsi(5_000, 10_000, 0.0, 1.0, 1.0);
        assert_eq!(tsi, 0.0);
    }

    #[test]
    fn compute_tsi_zero_at_max_drawdown() {
        let tsi = compute_tsi(50_000, 10_000, 0.20, 0.78, 0.92);
        assert_eq!(tsi, 0.0);
    }

    #[test]
    fn compute_tsi_zero_above_max_drawdown() {
        let tsi = compute_tsi(50_000, 10_000, 0.30, 0.78, 0.92);
        assert_eq!(tsi, 0.0);
    }

    #[test]
    fn compute_tsi_zero_runway() {
        // min_runway_balance = 0 is invalid, return 0.
        let tsi = compute_tsi(50_000, 0, 0.0, 1.0, 1.0);
        assert_eq!(tsi, 0.0);
    }

    // ── Full Evaluator ────────────────────────────────────────────────

    #[test]
    fn evaluate_healthy() {
        // Provide oracle so score_degraded is false — no oracle = degraded.
        let mut evaluator = Evaluator::with_config(
            3, 2, 6, Some(PriceOracle { price_usdc: 1.0 }),
        );
        let state = default_state(50_000);
        let bridge = good_bridge();

        let result = evaluator.evaluate(&state, Some(&bridge));

        assert!(result.tsi > 0.0, "TSI should be positive");
        assert!(result.growth_factor > 0.0);
        assert!(result.safety_factor > 0.0);
        assert!(result.reliability_factor > 0.0);
        assert!(!result.score_degraded);
        assert!(!result.oracle_degraded);
        assert!(!result.bridge_degraded);
    }

    #[test]
    fn evaluate_healthy_no_oracle_flags_degraded() {
        // Without oracle: score_degraded = true (oracle_degraded).
        let mut evaluator = Evaluator::new(3);
        let state = default_state(50_000);
        let bridge = good_bridge();

        let result = evaluator.evaluate(&state, Some(&bridge));

        assert!(result.tsi > 0.0);
        assert!(result.oracle_degraded);
        assert!(result.score_degraded);
        assert!(!result.bridge_degraded); // Bridge was provided.
    }

    #[test]
    fn evaluate_runway_breached_short_circuit() {
        let mut evaluator = Evaluator::new(3);
        let state = default_state(8_000); // Below min_runway_balance (10k)
        let bridge = good_bridge();

        let result = evaluator.evaluate(&state, Some(&bridge));

        assert_eq!(result.tsi, 0.0);
        assert_eq!(result.growth_factor, 0.0);
        assert_eq!(result.safety_factor, 0.0);
        assert_eq!(result.reliability_factor, 0.0);
    }

    #[test]
    fn evaluate_no_bridge_degraded() {
        let mut evaluator = Evaluator::new(3);
        let state = default_state(50_000);

        let result = evaluator.evaluate(&state, None);

        // First evaluation: no prev_state, so fee_momentum = 0.0.
        // TSI = growth * safety * 0.0 = 0.0
        assert!(result.bridge_degraded);
        assert!(result.score_degraded);
        // But second evaluation with new fees should give positive momentum.
    }

    #[test]
    fn evaluate_bridge_degraded_with_momentum() {
        let mut evaluator = Evaluator::new(3);

        // First evaluation to establish baseline.
        let state1 = default_state(50_000);
        let _ = evaluator.evaluate(&state1, None);

        // Second evaluation with increased fees (simulating growth).
        let mut state2 = state1.clone();
        state2.total_fees_withdrawn += 5_000;
        let result = evaluator.evaluate(&state2, None);

        // Should have positive reliability from fee momentum.
        assert!(result.bridge_degraded);
        assert!(result.reliability_factor > 0.0);
        // Growth and safety should be non-zero (vault > runway, no drawdown data).
        assert!(result.growth_factor > 0.0);
        assert!(result.safety_factor > 0.0);
        assert!(result.tsi > 0.0);
    }

    #[test]
    fn evaluate_with_price_oracle() {
        let oracle = PriceOracle { price_usdc: 1.50 };
        let mut evaluator = Evaluator::with_config(3, 2, 6, Some(oracle));

        let state = default_state(50_000);
        let result = evaluator.evaluate(&state, None);

        // NAV should be 50_000 * 1.5 = 75_000.
        assert!((result.secondary.treasury_nav - 75_000.0).abs() < 1.0);
        assert!(!result.oracle_degraded);
    }

    // ── Stagnation detection ──────────────────────────────────────────

    #[test]
    fn stagnant_after_threshold_declining() {
        let mut evaluator = Evaluator::new(3);

        // Simulate 3 declining TSI readings.
        let bridge = good_bridge();
        let state_high = default_state(60_000);
        let state_mid = default_state(55_000);
        let state_low = default_state(50_000);

        evaluator.evaluate(&state_high, Some(&bridge));
        assert!(!evaluator.is_stagnant());

        evaluator.evaluate(&state_mid, Some(&bridge));
        assert!(!evaluator.is_stagnant()); // Only 2 readings.

        evaluator.evaluate(&state_low, Some(&bridge));
        assert!(evaluator.is_stagnant()); // 3 declining readings.
    }

    #[test]
    fn not_stagnant_if_improving() {
        let mut evaluator = Evaluator::new(3);

        let bridge = good_bridge();
        let state_low = default_state(50_000);
        let state_mid = default_state(55_000);
        let state_high = default_state(60_000);

        evaluator.evaluate(&state_low, Some(&bridge));
        evaluator.evaluate(&state_mid, Some(&bridge));
        evaluator.evaluate(&state_high, Some(&bridge));

        assert!(!evaluator.is_stagnant());
    }

    #[test]
    fn not_stagnant_with_insufficient_data() {
        let evaluator = Evaluator::new(3);
        assert!(!evaluator.is_stagnant());
    }

    #[test]
    fn stagnant_configurable_threshold() {
        let mut evaluator = Evaluator::new(2); // Threshold = 2

        let bridge = good_bridge();
        let state_high = default_state(60_000);
        let state_low = default_state(50_000);

        evaluator.evaluate(&state_high, Some(&bridge));
        assert!(!evaluator.is_stagnant());

        evaluator.evaluate(&state_low, Some(&bridge));
        assert!(evaluator.is_stagnant()); // Triggers at 2, not 3.
    }

    #[test]
    fn stagnation_broken_by_improvement() {
        let mut evaluator = Evaluator::new(3);

        let bridge = good_bridge();
        let state_high = default_state(60_000);
        let state_low = default_state(50_000);

        evaluator.evaluate(&state_high, Some(&bridge));
        evaluator.evaluate(&state_low, Some(&bridge));
        assert!(!evaluator.is_stagnant()); // Only 2 readings.

        // Improve on third reading — not stagnant.
        evaluator.evaluate(&state_high, Some(&bridge));
        assert!(!evaluator.is_stagnant());
    }

    // ── Terminal state detection ───────────────────────────────────────

    #[test]
    fn terminal_after_consecutive_zeros() {
        let mut evaluator = Evaluator::new(3);

        // Below runway → TSI = 0.
        let state = default_state(5_000);

        evaluator.evaluate(&state, None);
        assert!(!evaluator.is_terminal()); // 1 zero.

        evaluator.evaluate(&state, None);
        assert!(evaluator.is_terminal()); // 2 zeros = default threshold.
    }

    #[test]
    fn not_terminal_if_tsi_recovers() {
        // First: below runway → zero TSI.
        let mut evaluator = Evaluator::new(3);
        let state_dead = default_state(5_000);
        evaluator.evaluate(&state_dead, None);

        // Second: recovered with bridge → positive TSI, zero counter resets.
        let state_alive = default_state(50_000);
        let result = evaluator.evaluate(&state_alive, Some(&good_bridge()));
        assert!(result.tsi > 0.0);
        assert!(!evaluator.is_terminal()); // Counter reset on non-zero TSI.
    }

    // ── Bridge failure tracking ────────────────────────────────────────

    #[test]
    fn bridge_failed_after_limit() {
        let mut evaluator = Evaluator::new(3);
        let state = default_state(50_000);

        // 5 consecutive bridge failures — not yet at limit (default 6).
        for _ in 0..5 {
            evaluator.evaluate(&state, None);
        }
        let health = evaluator.health_check();
        assert!(!health.bridge_failed);

        // 6th failure triggers.
        evaluator.evaluate(&state, None);
        let health = evaluator.health_check();
        assert!(health.bridge_failed);
    }

    #[test]
    fn bridge_failure_counter_resets_on_success() {
        let mut evaluator = Evaluator::new(3);
        let state = default_state(50_000);

        // 5 failures.
        for _ in 0..5 {
            evaluator.evaluate(&state, None);
        }

        // One success resets the counter.
        evaluator.evaluate(&state, Some(&good_bridge()));

        // 5 more failures — still not at limit (counter restarted).
        for _ in 0..5 {
            evaluator.evaluate(&state, None);
        }
        let health = evaluator.health_check();
        assert!(!health.bridge_failed);
    }

    // ── Secondary metrics ─────────────────────────────────────────────

    #[test]
    fn secondary_metrics_populated() {
        let mut evaluator = Evaluator::new(3);
        let state = default_state(50_000);
        let bridge = good_bridge();

        let result = evaluator.evaluate(&state, Some(&bridge));

        assert!((result.secondary.treasury_nav - 50_000.0).abs() < 1.0);
        assert_eq!(result.secondary.phase, ProtocolPhase::Sustenance);
        assert_eq!(result.secondary.strategy_name.as_deref(), Some("mr_rsi_bb"));
        assert_eq!(result.secondary.folds_validated, Some(9));
        assert!(result.secondary.strategy_yield.is_some());
    }

    #[test]
    fn distribution_efficiency() {
        let mut evaluator = Evaluator::new(3);
        let state = default_state(50_000);
        // distributed = 49k + 14k + 7k = 70k. withdrawn = 100k.
        // efficiency = 70k / 100k = 0.70
        let result = evaluator.evaluate(&state, None);
        assert!((result.secondary.distribution_efficiency - 0.70).abs() < 0.01);
    }

    #[test]
    fn price_floor_distance() {
        let mut evaluator = Evaluator::new(3);
        let state = default_state(50_000);
        // vault = 50k, runway = 10k → distance = 5.0
        let result = evaluator.evaluate(&state, None);
        assert!((result.secondary.price_floor_distance - 5.0).abs() < 0.01);
    }

    #[test]
    fn price_floor_distance_danger() {
        let mut evaluator = Evaluator::new(3);
        let state = default_state(12_000);
        // vault = 12k, runway = 10k → distance = 1.2
        let result = evaluator.evaluate(&state, None);
        assert!((result.secondary.price_floor_distance - 1.2).abs() < 0.01);
    }

    // ── History management ─────────────────────────────────────────────

    #[test]
    fn history_capped_at_100() {
        let mut evaluator = Evaluator::new(3);
        let state = default_state(50_000);
        let bridge = good_bridge();

        for _ in 0..150 {
            evaluator.evaluate(&state, Some(&bridge));
        }

        assert_eq!(evaluator.tsi_history().len(), 100);
    }

    // ── Oracle ─────────────────────────────────────────────────────────

    #[test]
    fn oracle_set_and_clear() {
        let mut evaluator = Evaluator::new(3);
        assert!(evaluator.price_oracle.is_none());

        evaluator.set_oracle(PriceOracle { price_usdc: 2.0 });
        assert!(evaluator.price_oracle.is_some());

        evaluator.clear_oracle();
        assert!(evaluator.price_oracle.is_none());
    }

    #[test]
    fn oracle_affects_growth_calculation() {
        let state = default_state(50_000);

        // Without oracle: vault=50k, runway=10k → growth = ln(5) ≈ 1.609
        let mut eval_no_oracle = Evaluator::with_config(3, 2, 6, None);
        let result_no = eval_no_oracle.evaluate(&state, Some(&good_bridge()));

        // With oracle (price=2.0): vault=100k, runway=20k → growth = ln(5) ≈ 1.609
        // Same ratio → same growth factor. But NAV doubles.
        let mut eval_oracle = Evaluator::with_config(3, 2, 6, Some(PriceOracle { price_usdc: 2.0 }));
        let result_oracle = eval_oracle.evaluate(&state, Some(&good_bridge()));

        // Growth should be the same (same ratio).
        assert!((result_no.growth_factor - result_oracle.growth_factor).abs() < 0.001);
        // But NAV should be 2×.
        assert!(
            (result_oracle.secondary.treasury_nav - 2.0 * result_no.secondary.treasury_nav).abs() < 1.0
        );
    }

    // ── Health check composite ─────────────────────────────────────────

    #[test]
    fn health_check_all_healthy() {
        // Use improving states to avoid stagnation detection.
        let mut evaluator = Evaluator::new(3);
        let bridge = good_bridge();

        for i in 0..5 {
            let state = default_state(50_000 + (i as u64 * 5_000));
            evaluator.evaluate(&state, Some(&bridge.clone()));
        }

        let health = evaluator.health_check();
        assert!(!health.stagnant, "Should not be stagnant with improving states");
        assert!(!health.terminal);
        assert!(!health.bridge_failed);
        assert_eq!(health.consecutive_zeros, 0);
        assert_eq!(health.consecutive_bridge_failures, 0);
    }
}
