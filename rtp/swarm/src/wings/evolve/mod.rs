//! Evolve Wing — the only wing that can modify how other wings work.
//!
//! Darwinian loop: Propose → Assess → Apply → Monitor → Keep or Rollback.
//! All changes are diffs routed through Coordinator, audited by Audit Wing.
//! LLM provider is swappable via LLM_API_BASE_URL + LLM_MODEL in configs/.env.swarm.

pub mod assessor;
pub mod proposer;
pub mod rollback;

use crate::types::WingId;
use crate::wings::trading::StrategyConfig;
use assessor::{Assessor, PerformanceMetrics};
use proposer::Proposer;
use rollback::RollbackManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// LLM-powered strategy proposer

/// A single parameter mutation proposed by the LLM proposer.
///
/// Each mutation targets exactly one strategy parameter, keeping its value
/// within the bounds enforced by the SoulContract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyMutation {
    /// Parameter name (e.g. "signal_threshold", "tp_atr").
    pub param: String,
    /// Proposed new value for the parameter.
    pub value: f64,
    /// Short rationale for why this mutation improves the strategy.
    pub rationale: String,
}

/// Configuration for the LLM strategy proposer.
#[derive(Debug, Clone)]
pub struct LlmProposerConfig {
    pub api_base_url: String,
    pub api_key: String,
    pub model: String,
}

impl LlmProposerConfig {
    /// Load from environment variables.
    /// Returns `None` if any required variable is missing (triggers fallback).
    pub fn from_env() -> Option<Self> {
        let api_base_url = std::env::var("LLM_API_BASE_URL").ok()?;
        let api_key = std::env::var("LLM_API_KEY").ok()?;
        let model = std::env::var("LLM_MODEL").ok()?;
        if api_base_url.is_empty() || api_key.is_empty() || model.is_empty() {
            return None;
        }
        Some(Self {
            api_base_url,
            api_key,
            model,
        })
    }
}

/// Result of an LLM proposer invocation.
#[derive(Debug)]
pub struct ProposeResult {
    /// The mutations proposed (either from LLM or deterministic fallback).
    pub mutations: Vec<StrategyMutation>,
    /// Whether the LLM was actually called (false = fallback path).
    pub used_llm: bool,
    /// The model name used (or "deterministic-fallback").
    pub model_label: String,
    /// Raw LLM response content (for audit trail). None if fallback used.
    pub raw_llm_response: Option<String>,
}

/// Live performance context injected into the LLM prompt.
///
/// Replaces hardcoded metrics with real data from the trading system.
/// Every field is Optional — if data is unavailable, the prompt omits
/// that section rather than fabricating numbers.
#[derive(Debug, Clone, Default)]
pub struct MutationContext {
    /// Current strategy parameters (not the hardcoded ones).
    pub current_params: Option<StrategyConfig>,
    /// Total realized PnL in SOL from live trading.
    pub total_pnl_sol: Option<f64>,
    /// Number of live trades completed.
    pub total_trades: Option<usize>,
    /// Number of winning trades.
    pub winning_trades: Option<usize>,
    /// Whether a position is currently open.
    pub has_open_position: bool,
    /// Hours since last trade (if any).
    pub hours_since_last_trade: Option<f64>,
    /// Current SOL price (from Flash Trade oracle).
    pub sol_price: Option<f64>,
    /// Recent volatility (ATR-based, annualized).
    pub recent_volatility: Option<f64>,
    /// Whether the market regime appears trending or ranging.
    pub regime_hint: Option<String>,
    /// Previous cycle's mutation results (for learning).
    pub prev_mutations_applied: Option<Vec<StrategyMutation>>,
    /// Previous cycle's PnL change after mutation (for feedback).
    pub prev_pnl_delta: Option<f64>,
}

/// Build the mutation prompt using real performance context.
///
/// If context is sparse (e.g. first cycle, no live data), the prompt
/// explicitly says so — the LLM should not assume fictional performance.
fn build_mutation_prompt(ctx: &MutationContext) -> String {
    let defaults = StrategyConfig::default();

    let signal = ctx.current_params.as_ref()
        .map(|p| p.signal_threshold)
        .unwrap_or(defaults.signal_threshold);
    let tp = ctx.current_params.as_ref()
        .map(|p| p.tp_atr)
        .unwrap_or(defaults.tp_atr);
    let sl = ctx.current_params.as_ref()
        .map(|p| p.sl_atr)
        .unwrap_or(defaults.sl_atr);
    let hold = ctx.current_params.as_ref()
        .map(|p| p.max_hold_hours)
        .unwrap_or(defaults.max_hold_hours);
    let trail = ctx.current_params.as_ref()
        .map(|p| p.trailing_stop_atr)
        .unwrap_or(defaults.trailing_stop_atr);

    let perf_section = match (ctx.total_pnl_sol, ctx.total_trades, ctx.winning_trades) {
        (Some(pnl), Some(n), Some(wins)) => {
            let win_rate = if n > 0 { wins as f64 / n as f64 * 100.0 } else { 0.0 };
            format!(
                "LIVE PERFORMANCE: total_pnl={:.6} SOL, trades={}, win_rate={:.0}%, open_position={}",
                pnl, n, win_rate, ctx.has_open_position,
            )
        }
        _ => "LIVE PERFORMANCE: no live trading data yet (starting from validated backtest baseline).".to_string(),
    };

    let regime_section = match &ctx.regime_hint {
        Some(r) => format!("MARKET REGIME: {}", r),
        None => "MARKET REGIME: unknown (treat as uncertain).".to_string(),
    };

    let volatility_section = match ctx.recent_volatility {
        Some(v) => format!("RECENT_VOLATILITY: {:.1}% annualized", v * 100.0),
        None => "RECENT_VOLATILITY: unknown.".to_string(),
    };

    let feedback_section = match (&ctx.prev_mutations_applied, ctx.prev_pnl_delta) {
        (Some(prev), Some(delta)) if !prev.is_empty() => {
            let prev_desc: Vec<String> = prev.iter()
                .map(|m| format!("{}={:.2}", m.param, m.value))
                .collect();
            format!(
                "PREVIOUS MUTATIONS: applied [{}]. Resulting PnL delta: {:.6} SOL. {}",
                prev_desc.join(", "),
                delta,
                if delta > 0.0 { "Mutations helped — continue in this direction." }
                else if delta < 0.0 { "Mutations hurt — consider reversing or trying a different direction." }
                else { "No measurable impact — try more aggressive changes." },
            )
        }
        _ => "PREVIOUS MUTATIONS: none (first mutation cycle).".to_string(),
    };

    format!(
        "You are the Evolve Wing of RTP, an autonomous treasury management \
         swarm on Solana. Your job is to propose parameter mutations that \
         improve REALIZED PnL, not backtest aesthetics.

CURRENT STRATEGY: SOL/USDT Survivor 2.69 (MultiTF trend-following)
Current params: signal_threshold={}, tp_atr={}, sl_atr={}, max_hold={}h, trailing_stop_atr={}. 9x leverage.

{}
{}
{}
{}

SOULCONTRACT BOUNDS (never exceed these):
- signal_threshold: [0.1, 0.5]
- tp_atr: [1.5, 5.0]
- sl_atr: [0.5, 3.0]
- max_hold: [12, 72] hours
- trailing_stop_atr: [0.2, 1.5]

CONSTRAINTS:
- Propose exactly 3 parameter mutations. Each must change exactly one param.
- Small changes only: max ±20% from current value. Large jumps cause regime mismatch.
- If performance is good, propose conservative tweaks. If bad, propose more aggressive changes.
- NEVER propose the same change twice if previous mutations hurt (see feedback above).
- Prioritize reducing drawdown over increasing return. Survival > profit.

Respond ONLY with valid JSON array, no explanation:
[{{\"param\": \"signal_threshold\", \"value\": 0.28, \"rationale\": \"...\"}}, \
 {{\"param\": \"tp_atr\", \"value\": 3.5, \"rationale\": \"...\"}}, \
 {{\"param\": \"trailing_stop_atr\", \"value\": 0.4, \"rationale\": \"...\"}}]",
        signal, tp, sl, hold, trail,
        perf_section,
        regime_section,
        volatility_section,
        feedback_section,
    )
}

/// Soulcontract-enforced bounds for strategy parameters.
///
/// These bounds are specified in the LLM prompt and MUST be validated
/// after the LLM responds, because an LLM can hallucinate out-of-range
/// values. This is the guardrail between "LLM says" and "code accepts".
const SOULCONTRACT_BOUNDS: &[(&str, f64, f64)] = &[
    ("signal_threshold", 0.1, 0.5),
    ("tp_atr", 1.5, 5.0),
    ("sl_atr", 0.5, 3.0),
    ("max_hold", 12.0, 72.0),
    ("trailing_stop_atr", 0.2, 1.5),
];

/// Validate that a proposed mutation falls within soulcontract bounds.
///
/// Returns `Ok(())` if the mutation is within bounds, `Err` with a
/// description if out of bounds. Unknown parameters are rejected.
pub fn validate_mutation_bounds(mutation: &StrategyMutation) -> Result<(), String> {
    let bound = SOULCONTRACT_BOUNDS
        .iter()
        .find(|(name, _, _)| *name == mutation.param);

    match bound {
        Some((_, min, max)) => {
            if mutation.value < *min || mutation.value > *max {
                Err(format!(
                    "mutation out of bounds: {}={} not in [{}, {}]",
                    mutation.param, mutation.value, min, max
                ))
            } else {
                Ok(())
            }
        }
        None => Err(format!(
            "unknown parameter: {} (not in soulcontract bounds)",
            mutation.param
        )),
    }
}

/// Validate all mutations, filtering out-of-bounds ones.
///
/// Returns only mutations that pass the bounds check. Logs rejections.
pub fn validate_all_mutations(mutations: Vec<StrategyMutation>) -> Vec<StrategyMutation> {
    mutations
        .into_iter()
        .filter(|m| match validate_mutation_bounds(m) {
            Ok(()) => true,
            Err(e) => {
                tracing::warn!("[EVOLVE] rejected mutation (bounds): {}", e);
                false
            }
        })
        .collect()
}

/// Maximum allowed delta (fractional change) from current value.
/// Prevents the LLM from proposing wild swings that would be
/// equivalent to a completely new (untested) strategy.
const MAX_MUTATION_DELTA: f64 = 0.20; // 20%

/// Validate mutations against a maximum delta from the current config.
///
/// This is the second gate after bounds checking. It rejects mutations
/// that change a parameter by more than 20% — per the PDF's guidance
/// that large parameter changes are equivalent to untested new strategies
/// and should go through full walk-forward validation, not LLM mutation.
///
/// Returns only mutations within the delta threshold.
pub fn validate_mutation_deltas(
    mutations: Vec<StrategyMutation>,
    current: &StrategyConfig,
) -> Vec<StrategyMutation> {
    mutations
        .into_iter()
        .filter(|m| {
            let current_val = match m.param.as_str() {
                "signal_threshold" => current.signal_threshold,
                "tp_atr" => current.tp_atr,
                "sl_atr" => current.sl_atr,
                "max_hold" => current.max_hold_hours,
                "trailing_stop_atr" => current.trailing_stop_atr,
                _ => return true, // unknown params already filtered by bounds check
            };
            if current_val == 0.0 {
                return true; // avoid division by zero
            }
            let delta = ((m.value - current_val) / current_val).abs();
            if delta > MAX_MUTATION_DELTA {
                tracing::warn!(
                    "[EVOLVE] rejected mutation (delta {:.0}% > {}% cap): {} {} → {}",
                    delta * 100.0,
                    MAX_MUTATION_DELTA * 100.0,
                    m.param,
                    current_val,
                    m.value,
                );
                false
            } else {
                tracing::info!(
                    "[EVOLVE] delta check passed: {} {} → {} ({:.1}%)",
                    m.param, current_val, m.value, delta * 100.0,
                );
                true
            }
        })
        .collect()
}

/// Deterministic fallback mutations used when the LLM is unavailable.
///
/// When the system has no performance data (first cycle, no live trades),
/// returns an empty vec — no mutations is safer than random mutations.
/// When performance data exists and is negative, returns conservative
/// defensive tweaks. When positive, returns small exploratory tweaks.
pub fn deterministic_fallback_mutations(ctx: &MutationContext) -> Vec<StrategyMutation> {
    let defaults = StrategyConfig::default();
    let signal = ctx.current_params.as_ref()
        .map(|p| p.signal_threshold).unwrap_or(defaults.signal_threshold);
    let tp = ctx.current_params.as_ref()
        .map(|p| p.tp_atr).unwrap_or(defaults.tp_atr);
    let trail = ctx.current_params.as_ref()
        .map(|p| p.trailing_stop_atr).unwrap_or(defaults.trailing_stop_atr);

    // If we have positive PnL, don't mess with it — small explorations only.
    // If negative, make defensive adjustments. If no data, stay flat.
    match (ctx.total_pnl_sol, ctx.total_trades) {
        (Some(pnl), Some(n)) if n > 0 && pnl >= 0.0 => {
            // Winning — tiny exploratory tweaks
            vec![
                StrategyMutation {
                    param: "signal_threshold".to_string(),
                    value: (signal * 1.04).clamp(0.1, 0.5),
                    rationale: "exploratory: slightly tighter entry filter while profitable".to_string(),
                },
                StrategyMutation {
                    param: "tp_atr".to_string(),
                    value: (tp * 1.03).clamp(1.5, 5.0),
                    rationale: "exploratory: let winners run slightly longer".to_string(),
                },
            ]
        }
        (Some(pnl), Some(n)) if n > 0 && pnl < 0.0 => {
            // Losing — defensive adjustments
            vec![
                StrategyMutation {
                    param: "trailing_stop_atr".to_string(),
                    value: (trail * 0.85).clamp(0.2, 1.5),
                    rationale: "defensive: tighter trailing stop to protect capital".to_string(),
                },
                StrategyMutation {
                    param: "sl_atr".to_string(),
                    value: (defaults.sl_atr * 0.9).clamp(0.5, 3.0),
                    rationale: "defensive: tighter stop-loss to limit drawdown".to_string(),
                },
            ]
        }
        _ => {
            // No live data — do NOT mutate. The validated backtest baseline
            // is better than uninformed parameter changes.
            tracing::info!("[EVOLVE] no live performance data — skipping deterministic mutations (safer to stay flat)");
            vec![]
        }
    }
}

/// Parse the LLM response content into a Vec<StrategyMutation>.
///
/// The LLM may wrap the JSON array in markdown code fences or add
/// extra whitespace. This function strips common wrapping patterns
/// before deserialising.
fn parse_mutation_response(content: &str) -> Result<Vec<StrategyMutation>, String> {
    let trimmed = content.trim();

    // Strip markdown code fences if present.
    let json_str = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .map(|s| s.strip_suffix("```").unwrap_or(s))
        .unwrap_or(trimmed)
        .trim();

    serde_json::from_str(json_str).map_err(|e| format!("Failed to parse LLM response: {}", e))
}

/// Call an OpenAI-compatible LLM API to propose strategy mutations.
///
/// If `LLM_API_KEY` is not set or the call fails, returns the
/// deterministic fallback mutations instead. Tests use this fallback
/// path (no API key in CI) so they always pass.
pub async fn propose_strategy_mutation(
    config: Option<LlmProposerConfig>,
    ctx: &MutationContext,
) -> ProposeResult {
    match config {
        Some(cfg) => {
            let client = reqwest::Client::new();
            let url = format!("{}/chat/completions", cfg.api_base_url);

            let body = serde_json::json!({
                "model": cfg.model,
                "messages": [
                    {
                        "role": "system",
                        "content": "You are a strategy parameter optimizer. Respond ONLY with a valid JSON array, no other text. Example format: [{\"param\": \"signal_threshold\", \"value\": 0.28, \"rationale\": \"reduce noise\"}]"
                    },
                    {
                        "role": "user",
                        "content": build_mutation_prompt(ctx)
                    }
                ],
                "temperature": 0.3,
                "max_tokens": 2048
            });

            match client
                .post(&url)
                .header("Authorization", format!("Bearer {}", cfg.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
            {
                Ok(resp) => {
                    match resp.json::<serde_json::Value>().await {
                        Ok(json) => {
                            // Extract content from OpenAI-compatible response.
                            // Some "thinking" models (e.g. glm-5-turbo) put the
                            // actual output in `reasoning_content` when
                            // `content` is empty or max_tokens is too low.
                            let content = json
                                .get("choices")
                                .and_then(|c| c.get(0))
                                .and_then(|c| c.get("message"))
                                .and_then(|m| {
                                    let c = m.get("content").and_then(|v| v.as_str()).unwrap_or("");
                                    if !c.is_empty() {
                                        Some(c.to_string())
                                    } else {
                                        // Fallback: try reasoning_content for thinking models.
                                        m.get("reasoning_content")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string())
                                    }
                                })
                                .unwrap_or_default();

                            match parse_mutation_response(&content) {
                                Ok(mutations) if !mutations.is_empty() => {
                                    let validated = validate_all_mutations(mutations);
                                    tracing::info!(
                                        "[EVOLVE] LLM proposed mutations, {} within soulcontract bounds (model: {})",
                                        validated.len(),
                                        cfg.model
                                    );
                                    ProposeResult {
                                        mutations: validated,
                                        used_llm: true,
                                        model_label: cfg.model.clone(),
                                        raw_llm_response: Some(content),
                                    }
                                }
                                Ok(_) | Err(_) => {
                                    tracing::warn!(
                                        "[EVOLVE] LLM response unparseable — using deterministic fallback"
                                    );
                                    ProposeResult {
                                        mutations: deterministic_fallback_mutations(ctx),
                                        used_llm: false,
                                        model_label: "deterministic-fallback".to_string(),
                                        raw_llm_response: Some(content),
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "[EVOLVE] LLM response decode failed: {} — using deterministic fallback",
                                e
                            );
                            ProposeResult {
                                mutations: deterministic_fallback_mutations(ctx),
                                used_llm: false,
                                model_label: "deterministic-fallback".to_string(),
                                raw_llm_response: None,
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "[EVOLVE] LLM unavailable: {} — using deterministic fallback",
                        e
                    );
                    ProposeResult {
                        mutations: deterministic_fallback_mutations(ctx),
                        used_llm: false,
                        model_label: "deterministic-fallback".to_string(),
                        raw_llm_response: None,
                    }
                }
            }
        }
        None => {
            tracing::info!("[EVOLVE] LLM unavailable — using deterministic fallback proposer");
            ProposeResult {
                mutations: deterministic_fallback_mutations(ctx),
                used_llm: false,
                model_label: "deterministic-fallback".to_string(),
                raw_llm_response: None,
            }
        }
    }
}

/// The Evolve Wing — self-modification and adaptation.
///
/// Combines:
/// - **Assessor**: benchmark wing performance, detect regressions
/// - **Proposer**: architecture change proposals (SPARC methodology)
/// - **Rollback**: revert changes that degrade performance >5%
pub struct EvolveWing {
    pub assessor: Arc<Assessor>,
    pub proposer: Arc<Proposer>,
    pub rollback: Arc<RollbackManager>,
}

impl EvolveWing {
    pub fn new() -> Self {
        let proposer = Arc::new(Proposer::new());
        let assessor = Arc::new(Assessor::new());
        let rollback = Arc::new(RollbackManager::with_defaults(proposer.clone()));

        Self {
            assessor,
            proposer,
            rollback,
        }
    }

    /// Record performance metrics for a wing and assess it.
    pub fn record_and_assess(&self, metrics: PerformanceMetrics) -> assessor::Assessment {
        let wing = metrics.wing;
        self.assessor.record_metrics(metrics);
        self.assessor.assess(wing)
    }

    /// Propose a change, assess baseline, and submit for audit.
    pub fn propose_change(
        &self,
        target_wing: WingId,
        description: String,
        diff: String,
        rationale: String,
        expected_impact: String,
    ) -> proposer::ChangeProposal {
        // Assess baseline before proposing.
        let baseline = self.assessor.assess(target_wing);
        let proposal_id = uuid::Uuid::new_v4();
        self.proposer.propose_with_id(
            proposal_id,
            target_wing,
            description,
            diff,
            rationale,
            expected_impact,
        );

        // Record baseline score on the proposal.
        let _ = self
            .proposer
            .set_baseline_score(&proposal_id, baseline.score);

        // Return the proposal with baseline set.
        match self.proposer.get(&proposal_id) {
            Some(p) => p,
            None => panic!("proposal not found after creation: {}", proposal_id),
        }
    }

    /// Check if any applied changes need rollback.
    pub fn check_rollbacks(&self) -> Vec<rollback::RollbackOperation> {
        let changes = self.rollback.tracked_changes();
        let mut triggered = Vec::new();

        for change in changes {
            if change.rolled_back {
                continue;
            }
            let current = self.assessor.assess(change.target_wing);
            if let Some(op) = self.rollback.evaluate(change.id, current.score) {
                triggered.push(op);
            }
        }

        triggered
    }

    /// Execute a rollback for a specific change.
    pub fn execute_rollback(
        &self,
        change_id: uuid::Uuid,
    ) -> Result<rollback::RollbackOperation, String> {
        self.rollback.execute_rollback(&change_id)
    }
}

impl Default for EvolveWing {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::WingId;

    fn trading_metrics(usdc_yield: f64, consistency: f64) -> PerformanceMetrics {
        PerformanceMetrics {
            wing: WingId::Trading,
            usdc_yield,
            sol_reserves: 50000.0,
            max_drawdown: 0.05,
            consistency,
            captured_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn evolve_wing_full_lifecycle() {
        let evolve = EvolveWing::new();

        // Step 1: Record baseline metrics.
        evolve.record_and_assess(trading_metrics(5000.0, 0.85));

        // Step 2: Propose a change.
        let proposal = evolve.propose_change(
            WingId::Trading,
            "Optimize entry logic".to_string(),
            "diff --git a/entry.rs".to_string(),
            "Better signals needed".to_string(),
            "+5% Sharpe".to_string(),
        );

        assert_eq!(proposal.status, proposer::ProposalStatus::Proposed);
        assert!(proposal.baseline_score.is_some());

        // Step 3: Track the change.
        let baseline = proposal.baseline_score.unwrap();
        evolve.rollback.track_change(crate::types::TrackedChange {
            id: proposal.id,
            target_wing: WingId::Trading,
            diff: "entry logic change".to_string(),
            baseline_score: baseline,
            applied_at: chrono::Utc::now(),
            rolled_back: false,
            rollback_reason: None,
        });

        // Step 4: Simulate improvement — no rollback.
        evolve.record_and_assess(trading_metrics(6000.0, 0.90));
        let rollbacks = evolve.check_rollbacks();
        assert!(rollbacks.is_empty());

        // Step 5: Simulate degradation — rollback triggered.
        evolve.record_and_assess(trading_metrics(500.0, 0.20));
        let rollbacks = evolve.check_rollbacks();
        assert_eq!(rollbacks.len(), 1);
        assert_eq!(rollbacks[0].status, rollback::RollbackStatus::Queued);

        // Step 6: Execute rollback.
        let result = evolve.execute_rollback(proposal.id).unwrap();
        assert_eq!(result.status, rollback::RollbackStatus::Completed);
    }

    #[test]
    fn propose_change_records_baseline() {
        let evolve = EvolveWing::new();
        evolve.record_and_assess(trading_metrics(5000.0, 0.85));

        let proposal = evolve.propose_change(
            WingId::Trading,
            "Test change".to_string(),
            "diff".to_string(),
            "reason".to_string(),
            "impact".to_string(),
        );

        assert!(proposal.baseline_score.unwrap() > 0.0);
    }

    #[test]
    fn evolve_wing_id() {
        let proposal = proposer::ChangeProposal::new(
            WingId::Security,
            "test".to_string(),
            "diff".to_string(),
            "r".to_string(),
            "i".to_string(),
        );
        let msg = proposal.to_message();
        assert_eq!(msg.from, WingId::Evolve);
    }

    // LLM proposer tests

    fn empty_ctx() -> MutationContext {
        MutationContext::default()
    }

    fn winning_ctx() -> MutationContext {
        MutationContext {
            total_pnl_sol: Some(0.5),
            total_trades: Some(10),
            winning_trades: Some(7),
            ..Default::default()
        }
    }

    fn losing_ctx() -> MutationContext {
        MutationContext {
            total_pnl_sol: Some(-0.3),
            total_trades: Some(8),
            winning_trades: Some(2),
            ..Default::default()
        }
    }

    #[test]
    fn deterministic_fallback_no_data_returns_empty() {
        let mutations = deterministic_fallback_mutations(&empty_ctx());
        assert!(mutations.is_empty(), "no data = no mutations (safer)");
    }

    #[test]
    fn deterministic_fallback_winning_returns_exploratory() {
        let mutations = deterministic_fallback_mutations(&winning_ctx());
        assert!(!mutations.is_empty());
        // All within bounds
        for m in &mutations {
            assert!(validate_mutation_bounds(m).is_ok());
        }
    }

    #[test]
    fn deterministic_fallback_losing_returns_defensive() {
        let mutations = deterministic_fallback_mutations(&losing_ctx());
        assert!(!mutations.is_empty());
        // Should include defensive adjustments
        assert!(mutations.iter().any(|m| m.param == "trailing_stop_atr" || m.param == "sl_atr"));
    }

    #[test]
    fn strategy_mutation_serde_roundtrip() {
        let m = StrategyMutation {
            param: "signal_threshold".to_string(),
            value: 0.28,
            rationale: "reduce noise".to_string(),
        };
        let json = serde_json::to_string(&m).unwrap();
        let deserialized: StrategyMutation = serde_json::from_str(&json).unwrap();
        assert_eq!(m, deserialized);
    }

    #[tokio::test]
    async fn propose_with_no_config_uses_fallback() {
        let result = propose_strategy_mutation(None, &empty_ctx()).await;
        assert!(!result.used_llm);
        assert_eq!(result.model_label, "deterministic-fallback");
        assert!(result.mutations.is_empty()); // empty ctx = no mutations
        assert!(result.raw_llm_response.is_none());
    }

    #[tokio::test]
    async fn propose_with_bad_url_uses_fallback() {
        let cfg = LlmProposerConfig {
            api_base_url: "http://127.0.0.1:1".to_string(), // unreachable
            api_key: "test-key".to_string(),
            model: "test-model".to_string(),
        };
        let result = propose_strategy_mutation(Some(cfg), &empty_ctx()).await;
        assert!(!result.used_llm);
        assert!(result.raw_llm_response.is_none());
    }

    #[test]
    fn parse_mutation_response_clean_json() {
        let json = r#"[
            {"param": "signal_threshold", "value": 0.28, "rationale": "reduce noise"},
            {"param": "tp_atr", "value": 3.5, "rationale": "wider target"},
            {"param": "trailing_stop_atr", "value": 0.4, "rationale": "tighter stop"}
        ]"#;
        let mutations = parse_mutation_response(json).unwrap();
        assert_eq!(mutations.len(), 3);
        assert_eq!(mutations[0].param, "signal_threshold");
    }

    #[test]
    fn parse_mutation_response_markdown_wrapped() {
        let wrapped =
            "```json\n[{\"param\": \"tp_atr\", \"value\": 3.5, \"rationale\": \"test\"}]\n```";
        let mutations = parse_mutation_response(wrapped).unwrap();
        assert_eq!(mutations.len(), 1);
        assert_eq!(mutations[0].param, "tp_atr");
    }

    #[test]
    fn parse_mutation_response_invalid_json_fails() {
        let result = parse_mutation_response("not json at all");
        assert!(result.is_err());
    }

    #[test]
    fn llm_config_from_env_missing_returns_none() {
        // Clear any existing env vars for this test.
        // Safety: removing env vars is safe in single-threaded test context.
        unsafe {
            std::env::remove_var("LLM_API_BASE_URL");
            std::env::remove_var("LLM_API_KEY");
            std::env::remove_var("LLM_MODEL");
        }
        assert!(LlmProposerConfig::from_env().is_none());
    }

    #[test]
    fn validate_mutation_within_bounds() {
        let m = StrategyMutation {
            param: "signal_threshold".to_string(),
            value: 0.3,
            rationale: "test".to_string(),
        };
        assert!(validate_mutation_bounds(&m).is_ok());
    }

    #[test]
    fn validate_mutation_below_bounds_rejected() {
        let m = StrategyMutation {
            param: "signal_threshold".to_string(),
            value: 0.05, // below min 0.1
            rationale: "test".to_string(),
        };
        let err = validate_mutation_bounds(&m).unwrap_err();
        assert!(err.contains("out of bounds"));
        assert!(err.contains("0.05"));
    }

    #[test]
    fn validate_mutation_above_bounds_rejected() {
        let m = StrategyMutation {
            param: "tp_atr".to_string(),
            value: 6.0, // above max 5.0
            rationale: "test".to_string(),
        };
        let err = validate_mutation_bounds(&m).unwrap_err();
        assert!(err.contains("out of bounds"));
    }

    #[test]
    fn validate_mutation_unknown_param_rejected() {
        let m = StrategyMutation {
            param: "evil_param".to_string(),
            value: 999.0,
            rationale: "malicious".to_string(),
        };
        let err = validate_mutation_bounds(&m).unwrap_err();
        assert!(err.contains("unknown parameter"));
    }

    #[test]
    fn validate_all_filters_out_of_bounds() {
        let mutations = vec![
            StrategyMutation {
                param: "signal_threshold".to_string(),
                value: 0.3, // valid
                rationale: "good".to_string(),
            },
            StrategyMutation {
                param: "tp_atr".to_string(),
                value: 99.0, // invalid
                rationale: "bad".to_string(),
            },
            StrategyMutation {
                param: "unknown".to_string(),
                value: 1.0, // invalid
                rationale: "ugly".to_string(),
            },
        ];
        let validated = validate_all_mutations(mutations);
        assert_eq!(validated.len(), 1);
        assert_eq!(validated[0].param, "signal_threshold");
    }

    #[test]
    fn deterministic_fallback_all_within_bounds() {
        // Verify all deterministic fallback mutations pass validation.
        let mutations = deterministic_fallback_mutations(&winning_ctx());
        for m in &mutations {
            assert!(
                validate_mutation_bounds(m).is_ok(),
                "fallback mutation {}={} should be within bounds",
                m.param,
                m.value
            );
        }
    }

    // Delta gate tests

    #[test]
    fn delta_gate_allows_small_change() {
        let current = StrategyConfig::default();
        let mutations = vec![
            StrategyMutation {
                param: "signal_threshold".to_string(),
                value: 0.27, // 0.25 → 0.27 = 8% change
                rationale: "test".to_string(),
            },
        ];
        let accepted = validate_mutation_deltas(mutations, &current);
        assert_eq!(accepted.len(), 1);
    }

    #[test]
    fn delta_gate_rejects_large_change() {
        let current = StrategyConfig::default();
        let mutations = vec![
            StrategyMutation {
                param: "tp_atr".to_string(),
                value: 2.0, // 5.0 → 2.0 = 60% change — should be rejected
                rationale: "test".to_string(),
            },
        ];
        let accepted = validate_mutation_deltas(mutations, &current);
        assert!(accepted.is_empty(), "60% change should be rejected");
    }

    #[test]
    fn delta_gate_boundary_exactly_20pct() {
        let current = StrategyConfig::default();
        let mutations = vec![
            StrategyMutation {
                param: "sl_atr".to_string(),
                value: 2.7 * 1.20, // exactly 20% up
                rationale: "test".to_string(),
            },
        ];
        let accepted = validate_mutation_deltas(mutations, &current);
        assert_eq!(accepted.len(), 1, "exactly 20% should pass (<=)");
    }

    #[test]
    fn delta_gate_mix_accepted_rejected() {
        let current = StrategyConfig::default();
        let mutations = vec![
            StrategyMutation {
                param: "signal_threshold".to_string(),
                value: 0.28, // 12% — ok
                rationale: "test".to_string(),
            },
            StrategyMutation {
                param: "tp_atr".to_string(),
                value: 1.5, // 70% — rejected
                rationale: "test".to_string(),
            },
            StrategyMutation {
                param: "trailing_stop_atr".to_string(),
                value: 0.15, // 7% — ok
                rationale: "test".to_string(),
            },
        ];
        let accepted = validate_mutation_deltas(mutations, &current);
        assert_eq!(accepted.len(), 2);
        assert!(accepted.iter().any(|m| m.param == "signal_threshold"));
        assert!(accepted.iter().any(|m| m.param == "trailing_stop_atr"));
    }
}
