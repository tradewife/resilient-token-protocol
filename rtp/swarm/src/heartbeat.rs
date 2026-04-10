//! Heartbeat — coordination primitive between evaluator and swarm.
//!
//! The heartbeat is the CORAL-style trigger that translates evaluator
//! output into actionable signals for the rest of the swarm.
//!
//! ## Three heartbeat types (from CORAL §3.3)
//!
//! - **PerIteration**: fires after every `evaluate()` call. Default rhythm.
//! - **Consolidation**: fires every N iterations. Triggers periodic memory
//!   compression and cross-cycle insight extraction.
//! - **Redirect**: fires when the evaluator detects stagnation or a terminal
//!   state. Triggers strategy pivot, knowledge surfacing, or halt.
//!
//! ## Key constraint
//!
//! The heartbeat does NOT make strategy decisions. It only signals.
//! The orchestrator decides what to do with the signal.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::evaluator::{Evaluation, HealthCheck};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default number of iterations between consolidation heartbeats.
/// CORAL recommends periodic (not per-iteration) consolidation.
/// With a 30-second heartbeat interval, this means consolidation every 5 minutes.
pub const DEFAULT_CONSOLIDATION_INTERVAL: usize = 10;

// ---------------------------------------------------------------------------
// Heartbeat type
// ---------------------------------------------------------------------------

/// Why this heartbeat fired. Downstream consumers pattern-match on this
/// without needing to know evaluator internals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeartbeatType {
    /// Fired after every evaluate() call. Normal operating rhythm.
    PerIteration,
    /// Fired every N iterations (configurable). Periodic memory consolidation.
    Consolidation,
    /// Fired on stagnation or terminal state. Strategy pivot or halt required.
    Redirect,
}

impl std::fmt::Display for HeartbeatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeartbeatType::PerIteration => write!(f, "per-iteration"),
            HeartbeatType::Consolidation => write!(f, "consolidation"),
            HeartbeatType::Redirect => write!(f, "redirect"),
        }
    }
}

// ---------------------------------------------------------------------------
// Recommended action
// ---------------------------------------------------------------------------

/// What the heartbeat recommends the orchestrator do next.
///
/// The heartbeat does NOT execute these — it only signals.
/// The orchestrator interprets them in the context of the full system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecommendedAction {
    /// TSI healthy, no stagnation. Proceed with normal operations.
    Continue,
    /// Periodic consolidation due. Knowledge Wing should compress
    /// working memory into project-level insights.
    Consolidate,
    /// Strategy stagnant or TSI at zero (not yet terminal).
    /// Swarm should pivot: surface prior insights, propose parameter changes.
    Redirect,
    /// Terminal state detected. Escalate to human, halt new proposals.
    /// Do not execute any autonomous actions until human acknowledges.
    Halt,
}

impl std::fmt::Display for RecommendedAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecommendedAction::Continue => write!(f, "continue"),
            RecommendedAction::Consolidate => write!(f, "consolidate"),
            RecommendedAction::Redirect => write!(f, "redirect"),
            RecommendedAction::Halt => write!(f, "halt"),
        }
    }
}

// ---------------------------------------------------------------------------
// Heartbeat signal
// ---------------------------------------------------------------------------

/// The signal produced by every heartbeat. Contains everything downstream
/// consumers (memory_promotion, orchestrator, dashboard) need without
/// coupling to evaluator internals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatSignal {
    /// Why this heartbeat fired.
    pub heartbeat_type: HeartbeatType,
    /// Current Treasury Survival Index.
    pub current_tsi: f64,
    /// Change in TSI since the prior cycle. Zero on the first cycle.
    pub tsi_delta: f64,
    /// Evaluator detected stagnation (flat/declining TSI).
    pub stagnating: bool,
    /// Evaluator detected terminal state (consecutive zero TSI).
    pub terminal: bool,
    /// Evaluator operating in degraded mode (no oracle or no bridge).
    pub degraded: bool,
    /// What the heartbeat recommends the orchestrator do.
    pub recommended_action: RecommendedAction,
    /// Which evaluation cycle this is (monotonically increasing).
    pub cycle: usize,
    /// Timestamp of this heartbeat.
    pub timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the heartbeat engine. All parameters are configurable
/// at construction. Defaults are sensible for a 30-second heartbeat interval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    /// Number of iterations between consolidation heartbeats.
    /// Set to 1 to consolidate every cycle (not recommended).
    /// Set higher for longer intervals between memory compression.
    pub consolidation_interval: usize,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            consolidation_interval: DEFAULT_CONSOLIDATION_INTERVAL,
        }
    }
}

// ---------------------------------------------------------------------------
// Heartbeat engine
// ---------------------------------------------------------------------------

/// The heartbeat engine — translates evaluator output into signals.
///
/// Usage:
/// ```ignore
/// let evaluation = evaluator.evaluate(&state, bridge.as_ref());
/// let health = evaluator.health_check();
/// let signal = heartbeat.process(&evaluation, &health);
/// // Pass signal to orchestrator, memory promotion, dashboard.
/// ```
pub struct HeartbeatEngine {
    config: HeartbeatConfig,
    /// Monotonically increasing cycle counter.
    cycle: usize,
    /// TSI from the previous cycle (for delta computation).
    prev_tsi: Option<f64>,
}

impl Default for HeartbeatEngine {
    fn default() -> Self {
        Self::new(HeartbeatConfig::default())
    }
}

impl HeartbeatEngine {
    /// Create a new heartbeat engine with the given configuration.
    pub fn new(config: HeartbeatConfig) -> Self {
        Self {
            config,
            cycle: 0,
            prev_tsi: None,
        }
    }

    /// Create with a specific consolidation interval (convenience).
    pub fn with_consolidation_interval(interval: usize) -> Self {
        Self::new(HeartbeatConfig {
            consolidation_interval: interval,
        })
    }

    /// Process an evaluation result and produce a heartbeat signal.
    ///
    /// This is the single entry point. It:
    /// 1. Computes the TSI delta from the prior cycle.
    /// 2. Determines the heartbeat type based on health state and cycle count.
    /// 3. Determines the recommended action based on TSI and health.
    /// 4. Returns a `HeartbeatSignal` for downstream consumption.
    ///
    /// ## Safety guarantee
    ///
    /// If TSI = 0 (runway breach, max drawdown, etc.), the signal will
    /// always carry `Redirect` or `Halt` — never `Continue`.
    pub fn process(&mut self, evaluation: &Evaluation, health: &HealthCheck) -> HeartbeatSignal {
        self.cycle += 1;

        let current_tsi = evaluation.tsi;
        let tsi_delta = match self.prev_tsi {
            Some(prev) => current_tsi - prev,
            None => 0.0, // First cycle: no delta.
        };

        let stagnating = health.stagnant;
        let terminal = health.terminal;
        let degraded = evaluation.score_degraded;

        // Determine heartbeat type and recommended action.
        //
        // Decision tree (evaluated in priority order):
        //
        //   1. Terminal → Redirect + Halt
        //   2. TSI = 0 (not yet terminal) → Redirect + Redirect
        //   3. Stagnating → Redirect + Redirect
        //   4. Consolidation cycle → Consolidation + Consolidate
        //   5. Default → PerIteration + Continue
        //
        // Safety: TSI = 0 can NEVER produce Continue or Consolidate.
        let (heartbeat_type, recommended_action) = if terminal {
            // Terminal state: system is dying. Escalate immediately.
            (HeartbeatType::Redirect, RecommendedAction::Halt)
        } else if current_tsi == 0.0 {
            // Zero TSI but not yet terminal (first zero, or terminal
            // threshold hasn't been reached). Signal redirect so the
            // swarm can attempt recovery.
            (HeartbeatType::Redirect, RecommendedAction::Redirect)
        } else if stagnating {
            // Strategy is stagnant. Pivot.
            (HeartbeatType::Redirect, RecommendedAction::Redirect)
        } else if self.is_consolidation_cycle() {
            // Periodic consolidation cycle.
            (HeartbeatType::Consolidation, RecommendedAction::Consolidate)
        } else {
            // Normal operation.
            (HeartbeatType::PerIteration, RecommendedAction::Continue)
        };

        let signal = HeartbeatSignal {
            heartbeat_type,
            current_tsi,
            tsi_delta,
            stagnating,
            terminal,
            degraded,
            recommended_action,
            cycle: self.cycle,
            timestamp: Utc::now(),
        };

        // Store TSI for next cycle's delta computation.
        self.prev_tsi = Some(current_tsi);

        signal
    }

    /// Current cycle number.
    pub fn cycle(&self) -> usize {
        self.cycle
    }

    /// Previous TSI value (for external delta computation if needed).
    pub fn prev_tsi(&self) -> Option<f64> {
        self.prev_tsi
    }

    /// Consolidation interval from config.
    pub fn consolidation_interval(&self) -> usize {
        self.config.consolidation_interval
    }

    // ── Internal ─────────────────────────────────────────────────────

    /// True if this cycle is a consolidation cycle.
    ///
    /// Fires when `cycle % consolidation_interval == 0`, but NOT on the
    /// first cycle (cycle 1 should be PerIteration to establish baseline).
    fn is_consolidation_cycle(&self) -> bool {
        self.cycle > 1
            && self.config.consolidation_interval > 0
            && self
                .cycle
                .is_multiple_of(self.config.consolidation_interval)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluator::{Evaluation, HealthCheck, ProtocolPhase, SecondaryMetrics};

    // ── Helpers ─────────────────────────────────────────────────────────

    fn healthy_evaluation(tsi: f64) -> Evaluation {
        Evaluation {
            tsi,
            growth_factor: 1.0,
            safety_factor: 0.9,
            reliability_factor: tsi / 0.9, // Back-calculate for consistency.
            score_degraded: false,
            oracle_degraded: false,
            bridge_degraded: false,
            secondary: SecondaryMetrics {
                treasury_nav: 50_000.0,
                fee_rate: 1.0,
                distribution_efficiency: 0.70,
                hydration_ratio: 0.10,
                price_floor_distance: 5.0,
                phase: ProtocolPhase::Sustenance,
                strategy_yield: Some(118.3),
                strategy_name: Some("mr_rsi_bb".to_string()),
                folds_validated: Some(9),
            },
            evaluated_at: Utc::now(),
        }
    }

    fn degraded_evaluation(tsi: f64) -> Evaluation {
        Evaluation {
            tsi,
            growth_factor: 1.0,
            safety_factor: 0.9,
            reliability_factor: tsi / 0.9,
            score_degraded: true,
            oracle_degraded: true,
            bridge_degraded: false,
            secondary: SecondaryMetrics {
                treasury_nav: 50_000.0,
                fee_rate: 1.0,
                distribution_efficiency: 0.70,
                hydration_ratio: 0.10,
                price_floor_distance: 5.0,
                phase: ProtocolPhase::Sustenance,
                strategy_yield: None,
                strategy_name: None,
                folds_validated: None,
            },
            evaluated_at: Utc::now(),
        }
    }

    fn zero_evaluation() -> Evaluation {
        Evaluation {
            tsi: 0.0,
            growth_factor: 0.0,
            safety_factor: 0.0,
            reliability_factor: 0.0,
            score_degraded: false,
            oracle_degraded: false,
            bridge_degraded: false,
            secondary: SecondaryMetrics {
                treasury_nav: 5_000.0,
                fee_rate: 0.0,
                distribution_efficiency: 0.0,
                hydration_ratio: 0.0,
                price_floor_distance: 0.5,
                phase: ProtocolPhase::Sustenance,
                strategy_yield: None,
                strategy_name: None,
                folds_validated: None,
            },
            evaluated_at: Utc::now(),
        }
    }

    fn healthy_check() -> HealthCheck {
        HealthCheck {
            stagnant: false,
            terminal: false,
            bridge_failed: false,
            tsi_history: vec![1.0, 1.1, 1.2],
            consecutive_zeros: 0,
            consecutive_bridge_failures: 0,
        }
    }

    fn stagnant_check() -> HealthCheck {
        HealthCheck {
            stagnant: true,
            terminal: false,
            bridge_failed: false,
            tsi_history: vec![1.2, 1.1, 1.0],
            consecutive_zeros: 0,
            consecutive_bridge_failures: 0,
        }
    }

    fn terminal_check() -> HealthCheck {
        HealthCheck {
            stagnant: true,
            terminal: true,
            bridge_failed: false,
            tsi_history: vec![0.0, 0.0],
            consecutive_zeros: 2,
            consecutive_bridge_failures: 0,
        }
    }

    // ── PerIteration ─────────────────────────────────────────────────────

    #[test]
    fn first_cycle_is_per_iteration() {
        let mut engine = HeartbeatEngine::default();
        let signal = engine.process(&healthy_evaluation(1.0), &healthy_check());

        assert_eq!(signal.heartbeat_type, HeartbeatType::PerIteration);
        assert_eq!(signal.recommended_action, RecommendedAction::Continue);
        assert_eq!(signal.cycle, 1);
        assert_eq!(signal.tsi_delta, 0.0); // No prior cycle.
    }

    #[test]
    fn second_healthy_cycle_is_per_iteration() {
        let mut engine = HeartbeatEngine::default();
        engine.process(&healthy_evaluation(1.0), &healthy_check());
        let signal = engine.process(&healthy_evaluation(1.1), &healthy_check());

        assert_eq!(signal.heartbeat_type, HeartbeatType::PerIteration);
        assert_eq!(signal.recommended_action, RecommendedAction::Continue);
        assert_eq!(signal.cycle, 2);
        assert!((signal.tsi_delta - 0.1).abs() < 0.001);
    }

    #[test]
    fn per_iteration_carries_correct_fields() {
        let mut engine = HeartbeatEngine::default();
        let signal = engine.process(&healthy_evaluation(1.47), &healthy_check());

        assert!((signal.current_tsi - 1.47).abs() < 0.001);
        assert!(!signal.stagnating);
        assert!(!signal.terminal);
        assert!(!signal.degraded);
        assert_eq!(signal.recommended_action, RecommendedAction::Continue);
    }

    // ── Consolidation ────────────────────────────────────────────────────

    #[test]
    fn consolidation_fires_at_interval() {
        let mut engine = HeartbeatEngine::with_consolidation_interval(3);

        // Cycle 1: PerIteration (first cycle is always PerIteration).
        let s1 = engine.process(&healthy_evaluation(1.0), &healthy_check());
        assert_eq!(s1.heartbeat_type, HeartbeatType::PerIteration);

        // Cycle 2: PerIteration.
        let s2 = engine.process(&healthy_evaluation(1.1), &healthy_check());
        assert_eq!(s2.heartbeat_type, HeartbeatType::PerIteration);

        // Cycle 3: Consolidation (3 % 3 == 0).
        let s3 = engine.process(&healthy_evaluation(1.2), &healthy_check());
        assert_eq!(s3.heartbeat_type, HeartbeatType::Consolidation);
        assert_eq!(s3.recommended_action, RecommendedAction::Consolidate);

        // Cycle 4: PerIteration again.
        let s4 = engine.process(&healthy_evaluation(1.3), &healthy_check());
        assert_eq!(s4.heartbeat_type, HeartbeatType::PerIteration);

        // Cycle 6: Consolidation again.
        let _ = engine.process(&healthy_evaluation(1.4), &healthy_check());
        let s6 = engine.process(&healthy_evaluation(1.5), &healthy_check());
        assert_eq!(s6.heartbeat_type, HeartbeatType::Consolidation);
    }

    #[test]
    fn consolidation_does_not_fire_on_first_cycle() {
        let mut engine = HeartbeatEngine::with_consolidation_interval(1);
        let signal = engine.process(&healthy_evaluation(1.0), &healthy_check());

        // Even with interval=1, cycle 1 is PerIteration (baseline).
        assert_eq!(signal.heartbeat_type, HeartbeatType::PerIteration);

        // Cycle 2 with interval=1 should consolidate.
        let s2 = engine.process(&healthy_evaluation(1.1), &healthy_check());
        assert_eq!(s2.heartbeat_type, HeartbeatType::Consolidation);
    }

    #[test]
    fn consolidation_interval_configurable() {
        let mut engine = HeartbeatEngine::with_consolidation_interval(5);

        for i in 1..=5 {
            let signal = engine.process(&healthy_evaluation(i as f64), &healthy_check());
            if i == 5 {
                assert_eq!(signal.heartbeat_type, HeartbeatType::Consolidation);
            } else {
                assert_eq!(signal.heartbeat_type, HeartbeatType::PerIteration);
            }
        }
    }

    // ── Redirect: stagnation ─────────────────────────────────────────────

    #[test]
    fn redirect_on_stagnation() {
        let mut engine = HeartbeatEngine::default();
        let signal = engine.process(&healthy_evaluation(1.0), &stagnant_check());

        assert_eq!(signal.heartbeat_type, HeartbeatType::Redirect);
        assert_eq!(signal.recommended_action, RecommendedAction::Redirect);
        assert!(signal.stagnating);
        assert!(!signal.terminal);
    }

    #[test]
    fn redirect_takes_priority_over_consolidation() {
        let mut engine = HeartbeatEngine::with_consolidation_interval(2);

        // Cycle 1: PerIteration.
        let _ = engine.process(&healthy_evaluation(1.0), &healthy_check());

        // Cycle 2: Would be consolidation, but stagnation overrides.
        let signal = engine.process(&healthy_evaluation(1.0), &stagnant_check());
        assert_eq!(signal.heartbeat_type, HeartbeatType::Redirect);
        assert_eq!(signal.recommended_action, RecommendedAction::Redirect);
    }

    #[test]
    fn redirect_carries_positive_tsi() {
        let mut engine = HeartbeatEngine::default();
        let signal = engine.process(&healthy_evaluation(0.95), &stagnant_check());

        assert!(signal.current_tsi > 0.0);
        assert_eq!(signal.heartbeat_type, HeartbeatType::Redirect);
    }

    // ── Redirect/Halt: TSI = 0 (safety short-circuit) ───────────────────

    #[test]
    fn zero_tsi_produces_redirect_not_continue() {
        let mut engine = HeartbeatEngine::default();
        let mut health = healthy_check();
        health.stagnant = false;
        health.terminal = false;
        // Note: consecutive_zeros = 1 (not yet terminal).

        let signal = engine.process(&zero_evaluation(), &health);

        // Safety guarantee: zero TSI → Redirect, never Continue.
        assert_eq!(signal.heartbeat_type, HeartbeatType::Redirect);
        assert_eq!(signal.recommended_action, RecommendedAction::Redirect);
        assert_eq!(signal.current_tsi, 0.0);
    }

    #[test]
    fn zero_tsi_takes_priority_over_consolidation() {
        let mut engine = HeartbeatEngine::with_consolidation_interval(1);
        // Establish a baseline cycle.
        let _ = engine.process(&healthy_evaluation(1.0), &healthy_check());

        // Cycle 2: Would consolidate, but zero TSI overrides.
        let signal = engine.process(&zero_evaluation(), &healthy_check());
        assert_eq!(signal.heartbeat_type, HeartbeatType::Redirect);
    }

    // ── Halt: terminal state ─────────────────────────────────────────────

    #[test]
    fn terminal_state_produces_halt() {
        let mut engine = HeartbeatEngine::default();
        let signal = engine.process(&zero_evaluation(), &terminal_check());

        assert_eq!(signal.heartbeat_type, HeartbeatType::Redirect);
        assert_eq!(signal.recommended_action, RecommendedAction::Halt);
        assert!(signal.terminal);
        assert_eq!(signal.current_tsi, 0.0);
    }

    #[test]
    fn halt_takes_priority_over_everything() {
        let mut engine = HeartbeatEngine::with_consolidation_interval(1);
        let _ = engine.process(&healthy_evaluation(1.0), &healthy_check());

        // Cycle 2: Would consolidate, but terminal overrides.
        let signal = engine.process(&zero_evaluation(), &terminal_check());
        assert_eq!(signal.recommended_action, RecommendedAction::Halt);
    }

    #[test]
    fn halt_signal_carries_terminal_flag() {
        let mut engine = HeartbeatEngine::default();
        let signal = engine.process(&zero_evaluation(), &terminal_check());

        assert!(signal.terminal);
        assert!(signal.stagnating); // Terminal implies stagnant in our health check.
    }

    // ── Degraded mode ────────────────────────────────────────────────────

    #[test]
    fn degraded_flag_propagates() {
        let mut engine = HeartbeatEngine::default();
        let signal = engine.process(&degraded_evaluation(1.0), &healthy_check());

        assert!(signal.degraded);
        assert_eq!(signal.heartbeat_type, HeartbeatType::PerIteration);
        // Degraded alone doesn't change the action — still Continue.
        assert_eq!(signal.recommended_action, RecommendedAction::Continue);
    }

    #[test]
    fn degraded_with_zero_tsi_is_redirect() {
        let mut engine = HeartbeatEngine::default();
        let eval = Evaluation {
            tsi: 0.0,
            score_degraded: true,
            ..zero_evaluation()
        };
        let signal = engine.process(&eval, &healthy_check());

        assert!(signal.degraded);
        assert_eq!(signal.heartbeat_type, HeartbeatType::Redirect);
    }

    // ── TSI delta tracking ───────────────────────────────────────────────

    #[test]
    fn tsi_delta_computed_across_cycles() {
        let mut engine = HeartbeatEngine::default();

        let s1 = engine.process(&healthy_evaluation(1.0), &healthy_check());
        assert_eq!(s1.tsi_delta, 0.0); // First cycle: no delta.

        let s2 = engine.process(&healthy_evaluation(1.2), &healthy_check());
        assert!((s2.tsi_delta - 0.2).abs() < 0.001);

        let s3 = engine.process(&healthy_evaluation(0.9), &healthy_check());
        assert!((s3.tsi_delta - (-0.3)).abs() < 0.001); // Dropped.
    }

    #[test]
    fn tsi_delta_with_zero_recovery() {
        let mut engine = HeartbeatEngine::default();

        let _ = engine.process(&healthy_evaluation(1.0), &healthy_check());
        let s2 = engine.process(&zero_evaluation(), &healthy_check());
        assert!((s2.tsi_delta - (-1.0)).abs() < 0.001);

        let s3 = engine.process(&healthy_evaluation(0.8), &healthy_check());
        assert!((s3.tsi_delta - 0.8).abs() < 0.001);
    }

    // ── Cycle tracking ───────────────────────────────────────────────────

    #[test]
    fn cycle_monotonically_increases() {
        let mut engine = HeartbeatEngine::default();

        for i in 1..=20 {
            let signal = engine.process(&healthy_evaluation(i as f64), &healthy_check());
            assert_eq!(signal.cycle, i);
        }
    }

    // ── Recovery transitions ─────────────────────────────────────────────

    #[test]
    fn recovery_from_redirect_to_continue() {
        let mut engine = HeartbeatEngine::default();

        // Stagnant → Redirect.
        let s1 = engine.process(&healthy_evaluation(1.0), &stagnant_check());
        assert_eq!(s1.heartbeat_type, HeartbeatType::Redirect);

        // Recovered → Continue.
        let s2 = engine.process(&healthy_evaluation(1.5), &healthy_check());
        assert_eq!(s2.heartbeat_type, HeartbeatType::PerIteration);
        assert_eq!(s2.recommended_action, RecommendedAction::Continue);
    }

    #[test]
    fn recovery_from_zero_tsi_to_continue() {
        let mut engine = HeartbeatEngine::default();

        // Zero TSI → Redirect.
        let s1 = engine.process(&zero_evaluation(), &healthy_check());
        assert_eq!(s1.heartbeat_type, HeartbeatType::Redirect);

        // Recovered → Continue (TSI positive, health clear).
        let s2 = engine.process(&healthy_evaluation(1.0), &healthy_check());
        assert_eq!(s2.recommended_action, RecommendedAction::Continue);
    }

    // ── Display traits ───────────────────────────────────────────────────

    #[test]
    fn heartbeat_type_display() {
        assert_eq!(format!("{}", HeartbeatType::PerIteration), "per-iteration");
        assert_eq!(format!("{}", HeartbeatType::Consolidation), "consolidation");
        assert_eq!(format!("{}", HeartbeatType::Redirect), "redirect");
    }

    #[test]
    fn recommended_action_display() {
        assert_eq!(format!("{}", RecommendedAction::Continue), "continue");
        assert_eq!(format!("{}", RecommendedAction::Consolidate), "consolidate");
        assert_eq!(format!("{}", RecommendedAction::Redirect), "redirect");
        assert_eq!(format!("{}", RecommendedAction::Halt), "halt");
    }

    // ── Edge cases ───────────────────────────────────────────────────────

    #[test]
    fn consolidation_interval_zero_never_consolidates() {
        let mut engine = HeartbeatEngine::with_consolidation_interval(0);

        for i in 1..=100 {
            let signal = engine.process(&healthy_evaluation(i as f64), &healthy_check());
            assert_eq!(signal.heartbeat_type, HeartbeatType::PerIteration);
        }
    }

    #[test]
    fn negative_tsi_delta_on_decline() {
        let mut engine = HeartbeatEngine::default();
        let _ = engine.process(&healthy_evaluation(2.0), &healthy_check());
        let signal = engine.process(&healthy_evaluation(1.5), &healthy_check());

        assert!((signal.tsi_delta - (-0.5)).abs() < 0.001);
        // Still Continue if not stagnant.
        assert_eq!(signal.recommended_action, RecommendedAction::Continue);
    }

    // ── Integration-like: full lifecycle sequence ────────────────────────

    #[test]
    fn full_lifecycle_sequence() {
        let mut engine = HeartbeatEngine::with_consolidation_interval(5);

        // Cycles 1-3: healthy, PerIteration.
        for i in 1..=3 {
            let s = engine.process(&healthy_evaluation(i as f64), &healthy_check());
            assert_eq!(s.heartbeat_type, HeartbeatType::PerIteration);
            assert_eq!(s.recommended_action, RecommendedAction::Continue);
        }

        // Cycle 4: stagnant → Redirect.
        let s4 = engine.process(&healthy_evaluation(3.0), &stagnant_check());
        assert_eq!(s4.heartbeat_type, HeartbeatType::Redirect);
        assert_eq!(s4.recommended_action, RecommendedAction::Redirect);

        // Cycle 5: recovered → would consolidate (5 % 5 == 0), but still healthy.
        let s5 = engine.process(&healthy_evaluation(3.5), &healthy_check());
        assert_eq!(s5.heartbeat_type, HeartbeatType::Consolidation);
        assert_eq!(s5.recommended_action, RecommendedAction::Consolidate);

        // Cycle 6-7: healthy.
        for i in 6..=7 {
            let s = engine.process(
                &healthy_evaluation(3.5 + (i as f64 - 5.0) * 0.1),
                &healthy_check(),
            );
            assert_eq!(s.heartbeat_type, HeartbeatType::PerIteration);
        }

        // Cycle 8: zero TSI → Redirect.
        let s8 = engine.process(&zero_evaluation(), &healthy_check());
        assert_eq!(s8.heartbeat_type, HeartbeatType::Redirect);
        assert_eq!(s8.recommended_action, RecommendedAction::Redirect);

        // Cycle 9: still zero → terminal → Halt.
        let s9 = engine.process(&zero_evaluation(), &terminal_check());
        assert_eq!(s9.heartbeat_type, HeartbeatType::Redirect);
        assert_eq!(s9.recommended_action, RecommendedAction::Halt);
    }
}
