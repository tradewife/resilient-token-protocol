//! Orchestrator — the daemon loop that wires the autonomous runtime.
//!
//! The orchestrator is the Symphony-style long-running process that ties
//! together the evaluator, heartbeat, and memory promotion layers into
//! a single autonomous loop. It does NOT make strategy decisions — it
//! dispatches on heartbeat signals and delegates to hooks.
//!
//! ## Architecture
//!
//! ```text
//! TreasuryFetcher ──→ OnChainState
//! BridgeFetcher   ──→ Option<BridgeMetrics>
//!                          │
//!                     Evaluator
//!                          │
//!                     HeartbeatEngine
//!                          │
//!                     MemoryPromotion
//!                          │
//!                   dispatch on signal
//!                    │  │  │  │
//!               Continue Consolidate Redirect Halt
//! ```
//!
//! ## Integration seam
//!
//! Treasury and bridge fetching are behind traits. Production wires in
//! real Solana RPC and bridge binary. Demo uses mock fetchers with
//! scripted state sequences.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::evaluator::{BridgeMetrics, Evaluation, Evaluator, OnChainState, PriceOracle};
use crate::heartbeat::{HeartbeatEngine, HeartbeatSignal, HeartbeatType, RecommendedAction};
use crate::memory_promotion::{MemoryConfig, MemoryPromotion};

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

/// Hook called after every orchestrator cycle completes.
type CycleHook = Box<dyn Fn(&HeartbeatSignal, &Evaluation) + Send + Sync>;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default poll interval for demo mode (1 second).
pub const DEFAULT_POLL_INTERVAL_MS: u64 = 1000;

/// Default poll interval for production (30 seconds).
pub const PRODUCTION_POLL_INTERVAL_MS: u64 = 30_000;

/// Maximum consecutive halts before terminal escalation.
pub const DEFAULT_MAX_CONSECUTIVE_HALTS: usize = 3;

// ---------------------------------------------------------------------------
// Fetcher traits — integration seam for Solana RPC and bridge binary
// ---------------------------------------------------------------------------

/// Fetches on-chain treasury state. Production: Solana RPC via getAccountInfo.
/// Demo: MockTreasuryFetcher with scripted state.
pub trait TreasuryFetcher: Send + Sync {
    fn fetch(&self) -> Result<OnChainState, String>;
}

/// Fetches off-chain bridge metrics. Production: bridge::call_bridge().
/// Demo: MockBridgeFetcher with scripted responses.
pub trait BridgeFetcher: Send + Sync {
    fn fetch(&self) -> Result<Option<BridgeMetrics>, String>;
}

// ---------------------------------------------------------------------------
// Mock fetchers (for tests and demo)
// ---------------------------------------------------------------------------

/// A scripted sequence of treasury states for testing.
pub struct MockTreasuryFetcher {
    states: Vec<OnChainState>,
    index: std::sync::atomic::AtomicUsize,
}

impl MockTreasuryFetcher {
    pub fn new(states: Vec<OnChainState>) -> Self {
        Self {
            states,
            index: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Single repeating state.
    pub fn constant(state: OnChainState) -> Self {
        Self::new(vec![state])
    }
}

impl TreasuryFetcher for MockTreasuryFetcher {
    fn fetch(&self) -> Result<OnChainState, String> {
        if self.states.is_empty() {
            return Err("no mock states configured".to_string());
        }
        let idx = self.index.fetch_add(1, Ordering::Relaxed) % self.states.len();
        Ok(self.states[idx].clone())
    }
}

/// A scripted sequence of bridge responses for testing.
pub struct MockBridgeFetcher {
    responses: Vec<Option<BridgeMetrics>>,
    index: std::sync::atomic::AtomicUsize,
}

impl MockBridgeFetcher {
    pub fn new(responses: Vec<Option<BridgeMetrics>>) -> Self {
        Self {
            responses,
            index: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Always returns the same bridge response.
    pub fn constant(resp: Option<BridgeMetrics>) -> Self {
        Self::new(vec![resp])
    }

    /// Always returns None (bridge offline).
    pub fn offline() -> Self {
        Self::constant(None)
    }
}

impl BridgeFetcher for MockBridgeFetcher {
    fn fetch(&self) -> Result<Option<BridgeMetrics>, String> {
        if self.responses.is_empty() {
            return Ok(None);
        }
        let idx = self.index.fetch_add(1, Ordering::Relaxed) % self.responses.len();
        Ok(self.responses[idx].clone())
    }
}

// ---------------------------------------------------------------------------
// Hooks — called by the orchestrator on signal dispatch
// ---------------------------------------------------------------------------

/// Hook functions called by the orchestrator. Set to no-ops by default.
/// Override for strategy pivoting, alerting, human escalation, etc.
pub struct Hooks {
    /// Called when the swarm should pivot strategy.
    pub on_redirect: Box<dyn Fn(&HeartbeatSignal) + Send + Sync>,
    /// Called on terminal state — human escalation required.
    pub on_halt: Box<dyn Fn(&HeartbeatSignal) + Send + Sync>,
    /// Called after every cycle completes.
    pub on_cycle_complete: CycleHook,
}

impl Default for Hooks {
    fn default() -> Self {
        Self {
            on_redirect: Box::new(|_| {}),
            on_halt: Box::new(|_| {}),
            on_cycle_complete: Box::new(|_, _| {}),
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Full orchestrator configuration. Every parameter is configurable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Milliseconds between evaluation cycles.
    pub poll_interval_ms: u64,
    /// Consecutive non-improving TSI readings before redirect.
    pub stagnation_threshold: usize,
    /// Heartbeat cycles between consolidation events.
    pub consolidation_interval: usize,
    /// TSI threshold for working → project memory promotion.
    pub tsi_promotion_threshold: f64,
    /// Consecutive positive deltas for project → overview promotion.
    pub improvement_window: usize,
    /// Root directory for persistent memory files.
    pub memory_base_path: PathBuf,
    /// Consecutive halts before terminal escalation.
    pub max_consecutive_halts: usize,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: DEFAULT_POLL_INTERVAL_MS,
            stagnation_threshold: 3,
            consolidation_interval: 10,
            tsi_promotion_threshold: 0.6,
            improvement_window: 5,
            memory_base_path: PathBuf::from("memory"),
            max_consecutive_halts: DEFAULT_MAX_CONSECUTIVE_HALTS,
        }
    }
}

// ---------------------------------------------------------------------------
// Status — queryable at any time
// ---------------------------------------------------------------------------

/// Runtime status of the orchestrator. Safe to read from any thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorStatus {
    pub current_tsi: f64,
    pub cycles_run: u64,
    pub last_heartbeat_type: Option<HeartbeatType>,
    pub last_recommended_action: Option<RecommendedAction>,
    pub consecutive_halts: usize,
    pub is_running: bool,
    pub last_cycle_at: Option<DateTime<Utc>>,
}

impl Default for OrchestratorStatus {
    fn default() -> Self {
        Self {
            current_tsi: 0.0,
            cycles_run: 0,
            last_heartbeat_type: None,
            last_recommended_action: None,
            consecutive_halts: 0,
            is_running: false,
            last_cycle_at: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Cycle result — returned after each cycle
// ---------------------------------------------------------------------------

/// Result of a single orchestrator cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleResult {
    pub cycle: u64,
    pub tsi: f64,
    pub tsi_delta: f64,
    pub heartbeat_type: HeartbeatType,
    pub recommended_action: RecommendedAction,
    pub memory_promoted: bool,
    pub degraded: bool,
    pub terminal: bool,
}

// ---------------------------------------------------------------------------
// Orchestrator
// ---------------------------------------------------------------------------

/// The autonomous protocol runtime.
///
/// Wires evaluator → heartbeat → memory promotion into a single loop.
/// Does NOT make strategy decisions — dispatches on signals and delegates
/// to hooks.
pub struct Orchestrator {
    config: OrchestratorConfig,
    evaluator: Evaluator,
    heartbeat: HeartbeatEngine,
    memory: MemoryPromotion,
    hooks: Hooks,
    status: std::sync::Mutex<OrchestratorStatus>,
    shutdown: Arc<AtomicBool>,
    // Price oracle, settable at runtime.
    oracle: std::sync::Mutex<Option<PriceOracle>>,
}

impl Orchestrator {
    /// Create a new orchestrator with the given configuration and hooks.
    pub fn new(config: OrchestratorConfig, hooks: Hooks) -> Self {
        let evaluator = Evaluator::new(config.stagnation_threshold);
        let heartbeat = HeartbeatEngine::with_consolidation_interval(config.consolidation_interval);
        let memory_config = MemoryConfig {
            project_tsi_threshold: config.tsi_promotion_threshold,
            overview_improvement_cycles: config.improvement_window,
            working_cap: 100,
            memory_dir: config.memory_base_path.clone(),
        };
        // Orchestrator controls persistence: disable for tests, enable for production.
        let memory = MemoryPromotion::new(memory_config, false);

        Self {
            config,
            evaluator,
            heartbeat,
            memory,
            hooks,
            status: std::sync::Mutex::new(OrchestratorStatus::default()),
            oracle: std::sync::Mutex::new(None),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create with default configuration and no-op hooks.
    pub fn with_defaults() -> Self {
        Self::new(OrchestratorConfig::default(), Hooks::default())
    }

    /// Create for demo — disk persistence enabled, no sleep.
    pub fn new_for_demo(config: OrchestratorConfig) -> Self {
        let evaluator = Evaluator::new(config.stagnation_threshold);
        let heartbeat = HeartbeatEngine::with_consolidation_interval(config.consolidation_interval);
        let memory_config = MemoryConfig {
            project_tsi_threshold: config.tsi_promotion_threshold,
            overview_improvement_cycles: config.improvement_window,
            working_cap: 100,
            memory_dir: config.memory_base_path.clone(),
        };
        let memory = MemoryPromotion::new(memory_config, true); // persist=true

        Self {
            config,
            evaluator,
            heartbeat,
            memory,
            hooks: Hooks::default(),
            status: std::sync::Mutex::new(OrchestratorStatus::default()),
            oracle: std::sync::Mutex::new(None),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create for testing — no disk persistence, no sleep.
    pub fn new_for_test(config: OrchestratorConfig) -> Self {
        let evaluator = Evaluator::new(config.stagnation_threshold);
        let heartbeat = HeartbeatEngine::with_consolidation_interval(config.consolidation_interval);
        let memory_config = MemoryConfig {
            project_tsi_threshold: config.tsi_promotion_threshold,
            overview_improvement_cycles: config.improvement_window,
            working_cap: 100,
            memory_dir: PathBuf::from("/tmp/rtp-orchestrator-test"),
        };
        let memory = MemoryPromotion::new_in_memory(memory_config);

        Self {
            config,
            evaluator,
            heartbeat,
            memory,
            hooks: Hooks::default(),
            status: std::sync::Mutex::new(OrchestratorStatus::default()),
            oracle: std::sync::Mutex::new(None),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Set the price oracle at runtime.
    pub fn set_oracle(&self, oracle: PriceOracle) {
        if let Ok(mut guard) = self.oracle.lock() {
            *guard = Some(oracle);
        }
    }

    /// Signal the orchestrator to shut down gracefully.
    pub fn request_shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Get the current status (safe to call from any thread).
    pub fn status(&self) -> OrchestratorStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Get a reference to the memory promotion engine.
    pub fn memory(&self) -> &MemoryPromotion {
        &self.memory
    }

    // ── Core loop ────────────────────────────────────────────────────

    /// Run a single orchestrator cycle.
    ///
    /// This is the atomic unit of the autonomous loop. Returns the
    /// cycle result for logging and dashboard display.
    pub fn run_cycle(
        &mut self,
        treasury: &dyn TreasuryFetcher,
        bridge: &dyn BridgeFetcher,
    ) -> Result<CycleResult, String> {
        // a. Fetch treasury state.
        let state = treasury.fetch()?;

        // b. Fetch bridge response.
        let bridge_metrics = bridge.fetch()?;

        // c. Update oracle if set.
        if let Ok(guard) = self.oracle.lock()
            && let Some(ref oracle) = *guard
        {
            self.evaluator.set_oracle(PriceOracle {
                price_usdc: oracle.price_usdc,
            });
        }

        // d. Evaluate.
        let evaluation = match &bridge_metrics {
            Some(bm) => self.evaluator.evaluate(&state, Some(bm)),
            None => self.evaluator.evaluate_onchain(&state),
        };

        // e. Heartbeat.
        let health = self.evaluator.health_check();
        let signal = self.heartbeat.process(&evaluation, &health);

        // f. Memory promotion.
        let promotion = self.memory.process(&evaluation, &signal);

        // g. Dispatch on recommended action.
        self.dispatch(&signal, &evaluation);

        // Update status.
        let cycle_result = CycleResult {
            cycle: signal.cycle as u64,
            tsi: signal.current_tsi,
            tsi_delta: signal.tsi_delta,
            heartbeat_type: signal.heartbeat_type,
            recommended_action: signal.recommended_action,
            memory_promoted: promotion.project_created.is_some()
                || promotion.overview_promoted.is_some(),
            degraded: signal.degraded,
            terminal: signal.terminal,
        };

        self.update_status(&signal, &cycle_result);

        // Structured logging.
        self.log_cycle(&cycle_result, &signal);

        Ok(cycle_result)
    }

    /// Run for exactly N cycles. Used by demo.sh.
    ///
    /// Returns all cycle results. Stops early on terminal escalation
    /// (max consecutive halts exceeded).
    pub fn run_for_cycles(
        &mut self,
        n: usize,
        treasury: &dyn TreasuryFetcher,
        bridge: &dyn BridgeFetcher,
    ) -> Vec<CycleResult> {
        let mut results = Vec::new();

        {
            let mut status = self.status.lock().unwrap();
            status.is_running = true;
        }

        for _ in 0..n {
            if self.shutdown.load(Ordering::Relaxed) {
                info!(
                    "Shutdown requested, stopping after {} cycles",
                    results.len()
                );
                break;
            }

            match self.run_cycle(treasury, bridge) {
                Ok(result) => {
                    let is_halt = result.recommended_action == RecommendedAction::Halt;
                    results.push(result);

                    if is_halt {
                        let status = self.status.lock().unwrap();
                        if status.consecutive_halts >= self.config.max_consecutive_halts {
                            error!(
                                "Terminal escalation: {} consecutive halts. Stopping.",
                                status.consecutive_halts
                            );
                            break;
                        }
                    }
                }
                Err(e) => {
                    error!("Cycle failed: {}", e);
                    break;
                }
            }
        }

        {
            let mut status = self.status.lock().unwrap();
            status.is_running = false;
        }

        results
    }

    /// Run indefinitely until shutdown, halt escalation, or error.
    /// Production entry point.
    pub fn run(&mut self, treasury: &dyn TreasuryFetcher, bridge: &dyn BridgeFetcher) {
        {
            let mut status = self.status.lock().unwrap();
            status.is_running = true;
        }

        loop {
            if self.shutdown.load(Ordering::Relaxed) {
                info!("Shutdown requested, exiting gracefully");
                break;
            }

            match self.run_cycle(treasury, bridge) {
                Ok(result) => {
                    if result.recommended_action == RecommendedAction::Halt {
                        let status = self.status.lock().unwrap();
                        if status.consecutive_halts >= self.config.max_consecutive_halts {
                            error!(
                                "Terminal escalation: {} consecutive halts",
                                status.consecutive_halts
                            );
                            break;
                        }
                    }
                }
                Err(e) => {
                    error!("Cycle failed: {}. Continuing.", e);
                }
            }

            // Sleep between cycles (skip in test mode with interval 0).
            if self.config.poll_interval_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(
                    self.config.poll_interval_ms,
                ));
            }
        }

        {
            let mut status = self.status.lock().unwrap();
            status.is_running = false;
        }
    }

    // ── Dispatch ─────────────────────────────────────────────────────

    /// Dispatch on the heartbeat signal's recommended action.
    fn dispatch(&self, signal: &HeartbeatSignal, evaluation: &Evaluation) {
        match signal.recommended_action {
            RecommendedAction::Continue => {
                // Normal operation — no hook needed.
            }
            RecommendedAction::Consolidate => {
                // Periodic consolidation — logged in log_cycle.
            }
            RecommendedAction::Redirect => {
                (self.hooks.on_redirect)(signal);
            }
            RecommendedAction::Halt => {
                (self.hooks.on_halt)(signal);
            }
        }
        (self.hooks.on_cycle_complete)(signal, evaluation);
    }

    // ── Status management ────────────────────────────────────────────

    fn update_status(&self, _signal: &HeartbeatSignal, result: &CycleResult) {
        let mut status = self.status.lock().unwrap();
        status.current_tsi = result.tsi;
        status.cycles_run = result.cycle;
        status.last_heartbeat_type = Some(result.heartbeat_type);
        status.last_recommended_action = Some(result.recommended_action);
        status.last_cycle_at = Some(Utc::now());

        // Track consecutive halts.
        if result.recommended_action == RecommendedAction::Halt {
            status.consecutive_halts += 1;
        } else {
            status.consecutive_halts = 0;
        }
    }

    // ── Structured logging ───────────────────────────────────────────

    fn log_cycle(&self, result: &CycleResult, signal: &HeartbeatSignal) {
        match result.recommended_action {
            RecommendedAction::Continue | RecommendedAction::Consolidate => {
                info!(
                    cycle = result.cycle,
                    tsi = format!("{:.3}", result.tsi),
                    delta = format!("{:+.3}", result.tsi_delta),
                    action = %result.recommended_action,
                    degraded = result.degraded,
                    "Cycle complete"
                );
            }
            RecommendedAction::Redirect => {
                warn!(
                    cycle = result.cycle,
                    tsi = format!("{:.3}", result.tsi),
                    delta = format!("{:+.3}", result.tsi_delta),
                    stagnating = signal.stagnating,
                    terminal = signal.terminal,
                    "Redirect: swarm pivoting"
                );
            }
            RecommendedAction::Halt => {
                error!(
                    cycle = result.cycle,
                    tsi = format!("{:.3}", result.tsi),
                    terminal = signal.terminal,
                    consecutive_halts = self.status().consecutive_halts,
                    "HALT: terminal state detected"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluator::{OnChainState, ProtocolPhase};

    // ── Helpers ─────────────────────────────────────────────────────────

    fn test_config() -> OrchestratorConfig {
        OrchestratorConfig {
            poll_interval_ms: 0, // No sleep in tests.
            stagnation_threshold: 3,
            consolidation_interval: 5,
            tsi_promotion_threshold: 0.6,
            improvement_window: 4,
            memory_base_path: PathBuf::from("/tmp/rtp-orch-test"),
            max_consecutive_halts: 3,
        }
    }

    fn healthy_state() -> OnChainState {
        OnChainState {
            vault_balance: 50_000,
            total_fees_withdrawn: 100_000,
            total_distributed_holders: 49_000,
            total_distributed_dev: 14_000,
            total_distributed_ecosystem: 7_000,
            total_hydration: 10_000,
            phase: ProtocolPhase::Sustenance,
            min_runway_balance: 10_000,
        }
    }

    fn improving_states(n: usize) -> Vec<OnChainState> {
        (0..n)
            .map(|i| OnChainState {
                vault_balance: 50_000 + (i as u64 + 1) * 5_000,
                total_fees_withdrawn: 100_000 + (i as u64 + 1) * 10_000,
                total_distributed_holders: 49_000,
                total_distributed_dev: 14_000,
                total_distributed_ecosystem: 7_000,
                total_hydration: 10_000,
                phase: ProtocolPhase::Sustenance,
                min_runway_balance: 10_000,
            })
            .collect()
    }

    fn declining_states(n: usize) -> Vec<OnChainState> {
        (0..n)
            .map(|i| OnChainState {
                vault_balance: 50_000 - (i as u64) * 5_000,
                total_fees_withdrawn: 100_000,
                total_distributed_holders: 49_000,
                total_distributed_dev: 14_000,
                total_distributed_ecosystem: 7_000,
                total_hydration: 10_000,
                phase: ProtocolPhase::Sustenance,
                min_runway_balance: 10_000,
            })
            .collect()
    }

    fn dead_state() -> OnChainState {
        OnChainState {
            vault_balance: 5_000, // Below runway.
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

    // ── Continue path ───────────────────────────────────────────────────

    #[test]
    fn full_loop_continue_path() {
        let mut config = test_config();
        config.consolidation_interval = 100; // Won't fire in 5 cycles.

        let mut orch = Orchestrator::new_for_test(config);
        orch.set_oracle(PriceOracle { price_usdc: 1.0 });
        let treasury = MockTreasuryFetcher::new(improving_states(5));
        let bridge = MockBridgeFetcher::constant(Some(good_bridge()));

        let results = orch.run_for_cycles(5, &treasury, &bridge);

        assert_eq!(results.len(), 5);
        for result in &results {
            assert_eq!(result.recommended_action, RecommendedAction::Continue);
            assert!(result.tsi > 0.0);
            assert!(!result.terminal);
        }
    }

    // ── Redirect path ───────────────────────────────────────────────────

    #[test]
    fn full_loop_redirect_path() {
        let mut config = test_config();
        config.stagnation_threshold = 3;

        let mut orch = Orchestrator::new_for_test(config);

        // First 3 healthy, then 3 declining → stagnation triggers redirect.
        let states: Vec<OnChainState> = vec![
            healthy_state(),
            healthy_state(),
            healthy_state(),
            declining_states(1).into_iter().next().unwrap(),
            declining_states(2).into_iter().nth(1).unwrap(),
            declining_states(3).into_iter().nth(2).unwrap(),
        ];

        let treasury = MockTreasuryFetcher::new(states);
        let bridge = MockBridgeFetcher::constant(Some(good_bridge()));

        let results = orch.run_for_cycles(6, &treasury, &bridge);

        // Last 3 cycles should trigger stagnation → redirect.
        let redirects: Vec<_> = results
            .iter()
            .filter(|r| r.recommended_action == RecommendedAction::Redirect)
            .collect();
        assert!(!redirects.is_empty(), "Expected at least one redirect");

        let last = results.last().unwrap();
        assert_eq!(last.heartbeat_type, HeartbeatType::Redirect);
    }

    // ── Halt path ───────────────────────────────────────────────────────

    #[test]
    fn full_loop_halt_path() {
        let config = test_config();
        let mut orch = Orchestrator::new_for_test(config);

        // All dead states → TSI=0 → terminal after 2 consecutive zeros.
        let states: Vec<OnChainState> = (0..10).map(|_| dead_state()).collect();
        let treasury = MockTreasuryFetcher::new(states);
        let bridge = MockBridgeFetcher::offline();

        let results = orch.run_for_cycles(10, &treasury, &bridge);

        // Should have halt results.
        let halts: Vec<_> = results
            .iter()
            .filter(|r| r.recommended_action == RecommendedAction::Halt)
            .collect();
        assert!(!halts.is_empty(), "Expected halt results");

        // Should stop early due to max_consecutive_halts.
        assert!(
            results.len() <= 6,
            "Should have stopped early, got {} cycles",
            results.len()
        );
    }

    // ── Halt counter resets on recovery ─────────────────────────────────

    #[test]
    fn halt_counter_resets_on_recovery() {
        let config = test_config();
        let mut orch = Orchestrator::new_for_test(config);

        // 2 dead (TSI=0 → halt), then recovery (healthy → continue).
        let states: Vec<OnChainState> =
            vec![dead_state(), dead_state(), healthy_state(), healthy_state()];
        let bridge_states: Vec<Option<BridgeMetrics>> = vec![
            None, // Dead cycle 1: no bridge.
            None, // Dead cycle 2: no bridge.
            Some(good_bridge()),
            Some(good_bridge()),
        ];

        let treasury = MockTreasuryFetcher::new(states);
        let bridge = MockBridgeFetcher::new(bridge_states);

        let results = orch.run_for_cycles(4, &treasury, &bridge);

        // Cycles 1-2 should be redirect/halt (TSI=0).
        assert!(results[0].tsi == 0.0);
        assert!(results[1].tsi == 0.0);

        // Cycles 3-4 should recover.
        assert!(results[2].tsi > 0.0);
        assert_eq!(results[3].recommended_action, RecommendedAction::Continue);

        // Consecutive halts should have reset.
        let status = orch.status();
        assert_eq!(status.consecutive_halts, 0);
    }

    // ── run_for_cycles terminates correctly ──────────────────────────────

    #[test]
    fn run_for_cycles_terminates_correctly() {
        let config = test_config();
        let mut orch = Orchestrator::new_for_test(config);
        let treasury = MockTreasuryFetcher::new(improving_states(10));
        let bridge = MockBridgeFetcher::constant(Some(good_bridge()));

        let results = orch.run_for_cycles(10, &treasury, &bridge);

        assert_eq!(results.len(), 10);
        assert_eq!(results[0].cycle, 1);
        assert_eq!(results[9].cycle, 10);

        let status = orch.status();
        assert!(!status.is_running);
        assert_eq!(status.cycles_run, 10);
    }

    // ── Consolidation fires at interval ──────────────────────────────────

    #[test]
    fn consolidation_fires_at_interval() {
        let mut config = test_config();
        config.consolidation_interval = 3;

        let mut orch = Orchestrator::new_for_test(config);
        let treasury = MockTreasuryFetcher::new(improving_states(10));
        let bridge = MockBridgeFetcher::constant(Some(good_bridge()));

        let results = orch.run_for_cycles(10, &treasury, &bridge);

        // Check that consolidation heartbeats fire at cycles 3, 6, 9.
        let consolidations: Vec<u64> = results
            .iter()
            .filter(|r| r.heartbeat_type == HeartbeatType::Consolidation)
            .map(|r| r.cycle)
            .collect();
        assert_eq!(consolidations, vec![3, 6, 9]);
    }

    // ── Status reflects current state ────────────────────────────────────

    #[test]
    fn status_reflects_current_state() {
        let config = test_config();
        let mut orch = Orchestrator::new_for_test(config);
        let treasury = MockTreasuryFetcher::new(improving_states(3));
        let bridge = MockBridgeFetcher::constant(Some(good_bridge()));

        let _results = orch.run_for_cycles(3, &treasury, &bridge);

        let status = orch.status();
        assert!(!status.is_running);
        assert_eq!(status.cycles_run, 3);
        assert!(status.current_tsi > 0.0);
        assert_eq!(
            status.last_heartbeat_type,
            Some(HeartbeatType::PerIteration)
        );
        assert_eq!(
            status.last_recommended_action,
            Some(RecommendedAction::Continue)
        );
        assert_eq!(status.consecutive_halts, 0);
        assert!(status.last_cycle_at.is_some());
    }

    // ── Shutdown signal ──────────────────────────────────────────────────

    #[test]
    fn shutdown_stops_loop() {
        let config = test_config();
        let mut orch = Orchestrator::new_for_test(config);
        let treasury = MockTreasuryFetcher::constant(healthy_state());
        let bridge = MockBridgeFetcher::constant(Some(good_bridge()));

        orch.request_shutdown();

        let results = orch.run_for_cycles(10, &treasury, &bridge);
        assert!(results.is_empty(), "Should stop immediately on shutdown");
    }

    // ── Memory promotion integration ─────────────────────────────────────

    #[test]
    fn memory_promotion_fires_on_consolidation() {
        let mut config = test_config();
        config.consolidation_interval = 3;

        let mut orch = Orchestrator::new_for_test(config);
        orch.set_oracle(PriceOracle { price_usdc: 1.0 });
        let treasury = MockTreasuryFetcher::new(improving_states(5));
        let bridge = MockBridgeFetcher::constant(Some(good_bridge()));

        let results = orch.run_for_cycles(5, &treasury, &bridge);

        // Cycle 3 is consolidation — should promote to project memory.
        let cycle3 = &results[2]; // 0-indexed.
        assert_eq!(cycle3.heartbeat_type, HeartbeatType::Consolidation);

        // Check memory has project entries.
        let project = orch.memory().project_consolidations();
        assert!(
            !project.is_empty(),
            "Expected project memory after consolidation"
        );
    }

    // ── Degraded mode ────────────────────────────────────────────────────

    #[test]
    fn degraded_mode_when_bridge_offline() {
        let config = test_config();
        let mut orch = Orchestrator::new_for_test(config);

        // Improving fee states to avoid stagnation, but no bridge → degraded.
        let states: Vec<OnChainState> = (0..5)
            .map(|i| OnChainState {
                vault_balance: 50_000,
                total_fees_withdrawn: 100_000 + (i as u64 + 1) * 5_000,
                total_distributed_holders: 49_000,
                total_distributed_dev: 14_000,
                total_distributed_ecosystem: 7_000,
                total_hydration: 10_000,
                phase: ProtocolPhase::Sustenance,
                min_runway_balance: 10_000,
            })
            .collect();
        let treasury = MockTreasuryFetcher::new(states);
        let bridge = MockBridgeFetcher::offline();

        let results = orch.run_for_cycles(3, &treasury, &bridge);

        // Results should have degraded flag.
        assert!(results.iter().any(|r| r.degraded));
    }

    // ── Hooks fire correctly ─────────────────────────────────────────────

    #[test]
    fn redirect_hook_fires() {
        use std::sync::atomic::AtomicUsize;

        let redirect_count = Arc::new(AtomicUsize::new(0));
        let rc = redirect_count.clone();

        let hooks = Hooks {
            on_redirect: Box::new(move |_| {
                rc.fetch_add(1, Ordering::Relaxed);
            }),
            ..Hooks::default()
        };

        let mut config = test_config();
        config.stagnation_threshold = 2;

        let mut orch = Orchestrator::new_for_test(config);
        orch.hooks = hooks;

        let states: Vec<OnChainState> = vec![
            healthy_state(),
            declining_states(1).into_iter().next().unwrap(),
            declining_states(2).into_iter().nth(1).unwrap(),
            declining_states(3).into_iter().nth(2).unwrap(),
        ];
        let treasury = MockTreasuryFetcher::new(states);
        let bridge = MockBridgeFetcher::constant(Some(good_bridge()));

        orch.run_for_cycles(4, &treasury, &bridge);

        assert!(
            redirect_count.load(Ordering::Relaxed) > 0,
            "Redirect hook should have fired"
        );
    }

    // ── Cycle complete hook fires every cycle ────────────────────────────

    #[test]
    fn cycle_complete_hook_fires_every_cycle() {
        use std::sync::atomic::AtomicUsize;

        let cycle_count = Arc::new(AtomicUsize::new(0));
        let cc = cycle_count.clone();

        let hooks = Hooks {
            on_cycle_complete: Box::new(move |_, _| {
                cc.fetch_add(1, Ordering::Relaxed);
            }),
            ..Hooks::default()
        };

        let config = test_config();
        let mut orch = Orchestrator::new(config, hooks);
        let treasury = MockTreasuryFetcher::constant(healthy_state());
        let bridge = MockBridgeFetcher::constant(Some(good_bridge()));

        orch.run_for_cycles(5, &treasury, &bridge);

        assert_eq!(cycle_count.load(Ordering::Relaxed), 5);
    }

    // ── Oracle integration ───────────────────────────────────────────────

    #[test]
    fn oracle_affects_evaluation() {
        let config = test_config();
        let mut orch = Orchestrator::new_for_test(config);
        orch.set_oracle(PriceOracle { price_usdc: 2.0 });

        let treasury = MockTreasuryFetcher::constant(healthy_state());
        let bridge = MockBridgeFetcher::constant(Some(good_bridge()));

        let results = orch.run_for_cycles(1, &treasury, &bridge);
        assert_eq!(results.len(), 1);
        // The TSI should still be positive with oracle set.
        assert!(results[0].tsi > 0.0);
    }

    // ── Full autonomous lifecycle (integration) ──────────────────────────

    #[test]
    fn full_autonomous_lifecycle() {
        let mut config = test_config();
        config.stagnation_threshold = 3;
        config.consolidation_interval = 4;
        config.improvement_window = 3;

        let mut orch = Orchestrator::new_for_test(config);

        // Script a full lifecycle:
        // Cycles 1-4: healthy, improving → working fills
        // Cycle 4: consolidation fires
        // Cycles 5-7: declining → stagnation → redirect
        // Cycles 8-10: recovery → improving → continue
        let states: Vec<OnChainState> = vec![
            // 1-4: healthy/improving
            OnChainState {
                vault_balance: 50_000,
                total_fees_withdrawn: 100_000,
                ..healthy_state()
            },
            OnChainState {
                vault_balance: 55_000,
                total_fees_withdrawn: 110_000,
                ..healthy_state()
            },
            OnChainState {
                vault_balance: 60_000,
                total_fees_withdrawn: 120_000,
                ..healthy_state()
            },
            OnChainState {
                vault_balance: 65_000,
                total_fees_withdrawn: 130_000,
                ..healthy_state()
            },
            // 5-7: declining → stagnation
            OnChainState {
                vault_balance: 60_000,
                total_fees_withdrawn: 130_000,
                ..healthy_state()
            },
            OnChainState {
                vault_balance: 55_000,
                total_fees_withdrawn: 130_000,
                ..healthy_state()
            },
            OnChainState {
                vault_balance: 50_000,
                total_fees_withdrawn: 130_000,
                ..healthy_state()
            },
            // 8-10: recovery
            OnChainState {
                vault_balance: 55_000,
                total_fees_withdrawn: 140_000,
                ..healthy_state()
            },
            OnChainState {
                vault_balance: 60_000,
                total_fees_withdrawn: 150_000,
                ..healthy_state()
            },
            OnChainState {
                vault_balance: 65_000,
                total_fees_withdrawn: 160_000,
                ..healthy_state()
            },
        ];

        let treasury = MockTreasuryFetcher::new(states);
        let bridge = MockBridgeFetcher::constant(Some(good_bridge()));

        let results = orch.run_for_cycles(10, &treasury, &bridge);

        assert_eq!(results.len(), 10);

        // Verify lifecycle phases:
        // Cycle 4: consolidation
        assert_eq!(results[3].heartbeat_type, HeartbeatType::Consolidation);

        // Cycles 5-7: should eventually redirect
        let redirects: Vec<_> = results[4..7]
            .iter()
            .filter(|r| r.heartbeat_type == HeartbeatType::Redirect)
            .collect();
        assert!(
            !redirects.is_empty(),
            "Expected redirect during declining phase"
        );

        // Cycles 8-10: recovery
        assert!(results[9].tsi > 0.0);
        assert_eq!(results[9].recommended_action, RecommendedAction::Continue);

        // Memory should have entries.
        assert!(!orch.memory().working().is_empty());
        assert!(!orch.memory().project().is_empty());

        // Status should be clean.
        let status = orch.status();
        assert_eq!(status.consecutive_halts, 0);
        assert!(!status.is_running);
    }
}
