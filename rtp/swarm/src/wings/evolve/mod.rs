//! Evolve Wing — the only wing that can modify how other wings work.
//!
//! Every change is a diff that goes through the Coordinator and must pass
//! the Audit Wing. The Evolve Wing follows the Darwinian loop:
//!   Propose -> Assess baseline -> Apply -> Monitor -> Keep or Rollback
//!
//! Reference: https://github.com/chrisworsey55/atlas-gic (Darwinian loop)
//!
//! LLM PROVIDER NOTE:
//! Currently: Zai API (OpenAI-compatible, glm-5-turbo)
//! Near-term: GLM-5.1 open weights on same interface
//! Production: Self-hosted on decentralised compute (Akash/Hyperbolic)
//! Zero code change required to swap — only LLM_API_BASE_URL + LLM_MODEL
//! in configs/.env.swarm. This is intentional: the swarm does not trust
//! any single AI provider.

pub mod assessor;
pub mod proposer;
pub mod rollback;

use crate::types::WingId;
use assessor::{Assessor, PerformanceMetrics};
use proposer::Proposer;
use rollback::RollbackManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ── LLM-powered strategy proposer ──────────────────────────────────────

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
}

/// Build the system prompt for strategy mutation proposals.
fn build_mutation_prompt() -> String {
    "You are the Evolve Wing of RTP, an autonomous treasury management \
     swarm on Solana. The current strategy is SOL/USDT Survivor 2.69 \
     with params: signal_threshold=0.3, tp_atr=3.0, sl_atr=1.5, \
     max_hold=36h, trailing_stop_atr=0.5. \
     Last cycle: yield=0.175 USDC, sharpe=3.96, 47 trades. \
     The heartbeat has detected stagnation. Propose exactly 3 parameter \
     mutations as JSON. Each mutation must change exactly one param. \
     Stay within these bounds: signal_threshold [0.1-0.5], \
     tp_atr [1.5-5.0], sl_atr [0.5-3.0], max_hold [12h-72h], \
     trailing_stop_atr [0.2-1.5]. \
     Respond ONLY with valid JSON array, no explanation: \
     [{\"param\": \"signal_threshold\", \"value\": 0.28, \"rationale\": \"...\"}, \
     {\"param\": \"tp_atr\", \"value\": 3.5, \"rationale\": \"...\"}, \
     {\"param\": \"trailing_stop_atr\", \"value\": 0.4, \"rationale\": \"...\"}]"
        .to_string()
}

/// Deterministic fallback mutations used when the LLM is unavailable.
///
/// These are the same three mutations the LLM would typically propose:
/// tighter signal filter, wider take-profit, tighter trailing stop.
pub fn deterministic_fallback_mutations() -> Vec<StrategyMutation> {
    vec![
        StrategyMutation {
            param: "signal_threshold".to_string(),
            value: 0.28,
            rationale: "reduce noise sensitivity".to_string(),
        },
        StrategyMutation {
            param: "tp_atr".to_string(),
            value: 3.5,
            rationale: "extend profit target in ranging market".to_string(),
        },
        StrategyMutation {
            param: "trailing_stop_atr".to_string(),
            value: 0.4,
            rationale: "tighten drawdown protection".to_string(),
        },
    ]
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
pub async fn propose_strategy_mutation(config: Option<LlmProposerConfig>) -> ProposeResult {
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
                        "content": build_mutation_prompt()
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
                                    tracing::info!(
                                        "[EVOLVE] LLM proposed {} mutations (model: {})",
                                        mutations.len(),
                                        cfg.model
                                    );
                                    ProposeResult {
                                        mutations,
                                        used_llm: true,
                                        model_label: cfg.model.clone(),
                                    }
                                }
                                Ok(_) | Err(_) => {
                                    tracing::warn!(
                                        "[EVOLVE] LLM response unparseable — using deterministic fallback"
                                    );
                                    ProposeResult {
                                        mutations: deterministic_fallback_mutations(),
                                        used_llm: false,
                                        model_label: "deterministic-fallback".to_string(),
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
                                mutations: deterministic_fallback_mutations(),
                                used_llm: false,
                                model_label: "deterministic-fallback".to_string(),
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
                        mutations: deterministic_fallback_mutations(),
                        used_llm: false,
                        model_label: "deterministic-fallback".to_string(),
                    }
                }
            }
        }
        None => {
            tracing::info!(
                "[EVOLVE] LLM unavailable — using deterministic fallback proposer"
            );
            ProposeResult {
                mutations: deterministic_fallback_mutations(),
                used_llm: false,
                model_label: "deterministic-fallback".to_string(),
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
        self.proposer.get(&proposal_id).unwrap()
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

    // ── LLM proposer tests ────────────────────────────────────────────

    #[test]
    fn deterministic_fallback_returns_three_mutations() {
        let mutations = deterministic_fallback_mutations();
        assert_eq!(mutations.len(), 3);
        assert_eq!(mutations[0].param, "signal_threshold");
        assert!((mutations[0].value - 0.28).abs() < f64::EPSILON);
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
        let result = propose_strategy_mutation(None).await;
        assert!(!result.used_llm);
        assert_eq!(result.model_label, "deterministic-fallback");
        assert_eq!(result.mutations.len(), 3);
    }

    #[tokio::test]
    async fn propose_with_bad_url_uses_fallback() {
        let cfg = LlmProposerConfig {
            api_base_url: "http://127.0.0.1:1".to_string(), // unreachable
            api_key: "test-key".to_string(),
            model: "test-model".to_string(),
        };
        let result = propose_strategy_mutation(Some(cfg)).await;
        assert!(!result.used_llm);
        assert_eq!(result.mutations.len(), 3);
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
        let wrapped = "```json\n[{\"param\": \"tp_atr\", \"value\": 3.5, \"rationale\": \"test\"}]\n```";
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
}
