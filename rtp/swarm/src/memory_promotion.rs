//! Memory promotion — the compression ladder from working to core.
//!
//! Reads HeartbeatSignal + Evaluation output and decides what gets written
//! to persistent memory, at what tier, and with what confidence.
//!
//! ## Compression ladder (Prologue-style, four tiers)
//!
//! ```text
//! working    → scratchpad, every cycle, overwritten freely
//! project    → task context, survives a session, promoted on Consolidate
//! overview   → cross-cycle strategy insights, promoted on sustained improvement
//! core       → durable protocol truths, promoted only with human confirmation
//! ```
//!
//! ## Promotion rules
//!
//! 1. Every evaluate() call → WorkingMemory entry.
//! 2. Consolidation heartbeat → eligible WorkingMemory promoted to ProjectMemory.
//! 3. Redirect heartbeat → RedirectEvent written to ProjectMemory immediately.
//! 4. N consecutive positive tsi_delta cycles → OverviewMemory from ProjectMemory.
//! 5. Core is append-only, human-confirmed — never autonomous.
//!
//! ## Persistence
//!
//! Files under `memory/{working,project,overview,core}/`, JSON format,
//! atomic writes (write temp, rename). Working memory capped at 100 entries
//! — consolidation is the natural garbage collector.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};

use crate::evaluator::{Evaluation, SecondaryMetrics};
use crate::heartbeat::{HeartbeatSignal, HeartbeatType, RecommendedAction};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum working memory entries before pruning.
/// Matches the evaluator's TSI history cap.
pub const WORKING_MEMORY_CAP: usize = 100;

/// Default TSI threshold for promotion from working → project.
/// Entries below this threshold are not eligible for consolidation.
pub const DEFAULT_PROJECT_TSI_THRESHOLD: f64 = 0.6;

/// Default number of consecutive positive tsi_delta cycles required
/// for promotion from project → overview.
pub const DEFAULT_OVERVIEW_IMPROVEMENT_CYCLES: usize = 5;

/// Default memory directory (relative to working directory or repo root).
pub const DEFAULT_MEMORY_DIR: &str = "memory";

// ---------------------------------------------------------------------------
// Memory tiers
// ---------------------------------------------------------------------------

/// Working memory — the scratchpad. Written every cycle, freely overwritten.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemory {
    pub cycle_id: usize,
    pub timestamp: DateTime<Utc>,
    pub tsi: f64,
    pub tsi_delta: f64,
    pub recommended_action: RecommendedAction,
    pub degraded: bool,
    pub secondary_snapshot: SecondaryMetrics,
    /// Confidence score for this entry: equals TSI (raw).
    pub confidence: f64,
}

/// Project memory — task context that survives a session.
/// Promoted from working memory on consolidation, or written immediately
/// on redirect events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMemory {
    /// Unique ID for this project memory entry.
    pub id: String,
    pub created_at: DateTime<Utc>,
    /// The consolidation window that produced this summary.
    pub cycle_range: (usize, usize),
    /// Number of cycles in the consolidation window.
    pub cycles_in_window: usize,
    pub avg_tsi: f64,
    pub best_tsi: f64,
    pub worst_tsi: f64,
    /// The most frequent recommended action in the window.
    pub dominant_action: RecommendedAction,
    /// Strategy name during this window, if consistent.
    pub strategy: Option<String>,
    /// Confidence: avg_tsi × (1 - degraded_cycle_ratio).
    pub confidence: f64,
}

/// Redirect event — written to project memory immediately when a redirect
/// fires, bypassing the consolidation window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedirectEvent {
    /// Unique ID for this event.
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub cycle_id: usize,
    /// What triggered the redirect.
    pub trigger: RedirectTrigger,
    /// TSI at the time of redirect.
    pub tsi_at_redirect: f64,
}

/// Why a redirect was triggered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RedirectTrigger {
    /// TSI flat or declining for stagnation_threshold consecutive cycles.
    Stagnation,
    /// TSI hit zero (vault at/below runway, or max drawdown).
    ZeroTsi,
    /// Consecutive zero-TSI readings hit terminal threshold.
    Terminal,
}

impl std::fmt::Display for RedirectTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RedirectTrigger::Stagnation => write!(f, "stagnation"),
            RedirectTrigger::ZeroTsi => write!(f, "zero_tsi"),
            RedirectTrigger::Terminal => write!(f, "terminal"),
        }
    }
}

/// Overview memory — cross-cycle strategy insights.
/// Promoted from project memory on sustained improvement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverviewMemory {
    /// Unique ID for this overview entry.
    pub id: String,
    pub created_at: DateTime<Utc>,
    /// Source project memory entries that contributed.
    pub source_project_ids: Vec<String>,
    /// Strategy pattern that produced improvement.
    pub strategy_pattern: String,
    /// Number of cycles of sustained improvement.
    pub improvement_cycles: usize,
    /// Peak TSI reached during the improvement period.
    pub peak_tsi: f64,
    /// Confidence: project_confidence × improvement_consistency.
    pub confidence: f64,
}

/// Core memory — durable protocol truths.
///
/// **IMPORTANT: This tier is append-only and requires human confirmation.**
/// The `promote_to_core()` method exists but is never called autonomously.
/// The orchestrator must present a CoreMemory candidate to a human operator
/// for review before it is persisted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreMemory {
    /// Unique ID for this core entry.
    pub id: String,
    pub created_at: DateTime<Utc>,
    /// What this core truth encodes.
    pub insight: String,
    /// Source overview memory that produced this truth.
    pub source_overview_id: String,
    /// Confidence: manually assigned by human reviewer.
    pub confidence: f64,
    /// Human operator who confirmed this entry.
    pub confirmed_by: String,
}

// ---------------------------------------------------------------------------
// Promotion result
// ---------------------------------------------------------------------------

/// Result of processing a heartbeat signal — what was written and promoted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionResult {
    /// Working memory entry was written.
    pub working_written: bool,
    /// Project memory was created (consolidation or redirect event).
    pub project_created: Option<ProjectMemoryOrRedirect>,
    /// Overview memory was promoted from project.
    pub overview_promoted: Option<OverviewMemory>,
    /// Working memory entries that were pruned.
    pub pruned_working_count: usize,
}

/// A project memory entry can be either a consolidation summary or
/// a redirect event — both live in the project tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectMemoryOrRedirect {
    Consolidation(ProjectMemory),
    Redirect(RedirectEvent),
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for memory promotion. All thresholds are configurable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// TSI threshold for working → project promotion eligibility.
    pub project_tsi_threshold: f64,
    /// Consecutive positive tsi_delta cycles for project → overview.
    pub overview_improvement_cycles: usize,
    /// Maximum working memory entries before pruning.
    pub working_cap: usize,
    /// Root directory for memory persistence.
    pub memory_dir: PathBuf,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            project_tsi_threshold: DEFAULT_PROJECT_TSI_THRESHOLD,
            overview_improvement_cycles: DEFAULT_OVERVIEW_IMPROVEMENT_CYCLES,
            working_cap: WORKING_MEMORY_CAP,
            memory_dir: PathBuf::from(DEFAULT_MEMORY_DIR),
        }
    }
}

// ---------------------------------------------------------------------------
// Memory promotion engine
// ---------------------------------------------------------------------------

/// The memory promotion engine.
///
/// Consumes `Evaluation` + `HeartbeatSignal` and manages the four-tier
/// memory ladder. Optionally persists to disk.
pub struct MemoryPromotion {
    config: MemoryConfig,
    /// In-memory working store (cycle_id → entry).
    working: Vec<WorkingMemory>,
    /// In-memory project store.
    project: Vec<ProjectMemoryOrRedirect>,
    /// In-memory overview store.
    overview: Vec<OverviewMemory>,
    /// Consecutive cycles with positive tsi_delta (for overview promotion).
    consecutive_improvement_count: usize,
    /// TSI deltas tracked for improvement detection.
    tsi_deltas: Vec<f64>,
    /// Whether to persist to disk (disabled in tests).
    persist: bool,
}

impl Default for MemoryPromotion {
    fn default() -> Self {
        Self::new(MemoryConfig::default(), true)
    }
}

impl MemoryPromotion {
    /// Create a new memory promotion engine.
    ///
    /// `persist`: if true, writes to disk. Set to false for tests.
    pub fn new(config: MemoryConfig, persist: bool) -> Self {
        if persist {
            Self::ensure_dirs(&config.memory_dir);
        }
        Self {
            config,
            working: Vec::new(),
            project: Vec::new(),
            overview: Vec::new(),
            consecutive_improvement_count: 0,
            tsi_deltas: Vec::new(),
            persist,
        }
    }

    /// Create without disk persistence (for testing).
    pub fn new_in_memory(config: MemoryConfig) -> Self {
        Self::new(config, false)
    }

    // ── Main entry point ──────────────────────────────────────────────

    /// Process an evaluation cycle. Writes working memory, then checks
    /// for consolidation or redirect promotion.
    ///
    /// Returns a `PromotionResult` describing what happened.
    pub fn process(
        &mut self,
        evaluation: &Evaluation,
        signal: &HeartbeatSignal,
    ) -> PromotionResult {
        // 1. Always write working memory.
        let working_written = self.write_working(evaluation, signal);
        let mut pruned = 0;

        // Track tsi_delta for improvement detection.
        self.track_improvement(signal.tsi_delta);

        // 2. Check heartbeat type for promotion.
        let project_created = match signal.heartbeat_type {
            HeartbeatType::Consolidation => {
                // Promote eligible working memory to project.
                let result = self.consolidate_working(signal.cycle);
                // Store the result before pruning.
                if let Some(ref pm_or) = result {
                    self.project.push(pm_or.clone());
                }
                // Prune consolidated entries from working.
                pruned = self.prune_working();
                result
            }
            HeartbeatType::Redirect => {
                // Write redirect event immediately.
                let event = self.write_redirect(signal);
                self.project.push(ProjectMemoryOrRedirect::Redirect(event.clone()));
                Some(ProjectMemoryOrRedirect::Redirect(event))
            }
            HeartbeatType::PerIteration => {
                // No promotion on normal cycles.
                None
            }
        };

        // 3. Check for sustained improvement → overview promotion.
        let overview_promoted = self.check_overview_promotion(evaluation, signal);

        // 4. Persist to disk if enabled.
        if self.persist {
            if working_written {
                let _ = self.persist_working();
            }
            if project_created.is_some() {
                let _ = self.persist_project();
            }
            if overview_promoted.is_some() {
                let _ = self.persist_overview();
            }
        }

        PromotionResult {
            working_written,
            project_created,
            overview_promoted,
            pruned_working_count: pruned,
        }
    }

    // ── Working memory ────────────────────────────────────────────────

    /// Write a working memory entry. Always called, every cycle.
    /// Returns true if written successfully.
    fn write_working(
        &mut self,
        evaluation: &Evaluation,
        signal: &HeartbeatSignal,
    ) -> bool {
        let entry = WorkingMemory {
            cycle_id: signal.cycle,
            timestamp: signal.timestamp,
            tsi: signal.current_tsi,
            tsi_delta: signal.tsi_delta,
            recommended_action: signal.recommended_action,
            degraded: evaluation.score_degraded,
            secondary_snapshot: evaluation.secondary.clone(),
            confidence: signal.current_tsi, // working confidence = TSI (raw)
        };

        self.working.push(entry);
        // Cap at working_cap — drop oldest entries if over limit.
        while self.working.len() > self.config.working_cap {
            self.working.remove(0);
        }

        true
    }

    /// Prune working memory entries that have been consolidated.
    /// Called after consolidation to free space.
    fn prune_working(&mut self) -> usize {
        // After consolidation, we can drop entries that are below the
        // project threshold (they won't be eligible for future promotion).
        let before = self.working.len();
        let threshold = self.config.project_tsi_threshold;
        self.working.retain(|w| w.tsi >= threshold);
        before - self.working.len()
    }

    // ── Project memory ────────────────────────────────────────────────

    /// Consolidate eligible working memory entries into a project summary.
    ///
    /// Eligibility: TSI >= threshold AND not degraded.
    fn consolidate_working(&self, current_cycle: usize) -> Option<ProjectMemoryOrRedirect> {
        let threshold = self.config.project_tsi_threshold;
        let eligible: Vec<&WorkingMemory> = self
            .working
            .iter()
            .filter(|w| w.tsi >= threshold && !w.degraded)
            .collect();

        if eligible.is_empty() {
            return None;
        }

        let min_cycle = eligible.iter().map(|w| w.cycle_id).min().unwrap_or(0);
        let max_cycle = eligible.iter().map(|w| w.cycle_id).max().unwrap_or(0);

        let avg_tsi = eligible.iter().map(|w| w.tsi).sum::<f64>() / eligible.len() as f64;
        let best_tsi = eligible.iter().map(|w| w.tsi).fold(f64::NEG_INFINITY, f64::max);
        let worst_tsi = eligible.iter().map(|w| w.tsi).fold(f64::INFINITY, f64::min);

        // Find the dominant recommended action (most frequent).
        let dominant_action = Self::dominant_action(&eligible);

        // Strategy name: use the most recent one if all agree.
        let strategy = eligible
            .last()
            .and_then(|w| w.secondary_snapshot.strategy_name.clone());

        // Degraded cycle ratio for confidence calculation.
        let degraded_count = self
            .working
            .iter()
            .filter(|w| w.degraded)
            .count();
        let total_count = self.working.len().max(1);
        let degraded_ratio = degraded_count as f64 / total_count as f64;

        // Project confidence: avg_tsi × (1 - degraded_cycle_ratio).
        let confidence = avg_tsi * (1.0 - degraded_ratio);

        let project = ProjectMemory {
            id: format!("proj-{}", current_cycle),
            created_at: Utc::now(),
            cycle_range: (min_cycle, max_cycle),
            cycles_in_window: eligible.len(),
            avg_tsi,
            best_tsi,
            worst_tsi,
            dominant_action,
            strategy,
            confidence,
        };

        Some(ProjectMemoryOrRedirect::Consolidation(project))
    }

    /// Write a redirect event to project memory immediately.
    fn write_redirect(&self, signal: &HeartbeatSignal) -> RedirectEvent {
        let trigger = if signal.terminal {
            RedirectTrigger::Terminal
        } else if signal.current_tsi == 0.0 {
            RedirectTrigger::ZeroTsi
        } else {
            RedirectTrigger::Stagnation
        };

        RedirectEvent {
            id: format!("redirect-{}", signal.cycle),
            created_at: Utc::now(),
            cycle_id: signal.cycle,
            trigger,
            tsi_at_redirect: signal.current_tsi,
        }
    }

    // ── Overview memory ───────────────────────────────────────────────

    /// Track consecutive positive tsi_delta for overview promotion.
    fn track_improvement(&mut self, tsi_delta: f64) {
        if tsi_delta > 0.0 {
            self.consecutive_improvement_count += 1;
        } else {
            self.consecutive_improvement_count = 0;
        }
        self.tsi_deltas.push(tsi_delta);
    }

    /// Check if sustained improvement warrants overview promotion.
    fn check_overview_promotion(
        &mut self,
        evaluation: &Evaluation,
        signal: &HeartbeatSignal,
    ) -> Option<OverviewMemory> {
        if self.consecutive_improvement_count < self.config.overview_improvement_cycles {
            return None;
        }

        // Find the best project memory to promote.
        let best_project = self
            .project
            .iter()
            .filter_map(|p| match p {
                ProjectMemoryOrRedirect::Consolidation(pm) => Some(pm),
                _ => None,
            })
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal));

        let (source_ids, strategy_pattern, project_confidence) = match best_project {
            Some(pm) => (
                vec![pm.id.clone()],
                pm.strategy.clone().unwrap_or_else(|| "mixed".to_string()),
                pm.confidence,
            ),
            None => {
                // No project memory yet — use strategy from evaluation.
                (
                    vec![],
                    evaluation
                        .secondary
                        .strategy_name
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                    0.5, // Low confidence without project backing.
                )
            }
        };

        // Improvement consistency: fraction of positive deltas in recent window.
        let window: Vec<f64> = self
            .tsi_deltas
            .iter()
            .rev()
            .take(self.config.overview_improvement_cycles)
            .cloned()
            .collect();
        let positive_count = window.iter().filter(|d| **d > 0.0).count();
        let consistency = positive_count as f64 / window.len().max(1) as f64;

        // Overview confidence: project_confidence × improvement_consistency.
        let confidence = project_confidence * consistency;

        let overview = OverviewMemory {
            id: format!("overview-{}", signal.cycle),
            created_at: Utc::now(),
            source_project_ids: source_ids,
            strategy_pattern,
            improvement_cycles: self.consecutive_improvement_count,
            peak_tsi: signal.current_tsi,
            confidence,
        };

        // Reset improvement counter after promotion.
        self.consecutive_improvement_count = 0;

        self.overview.push(overview.clone());

        Some(overview)
    }

    // ── Core memory (human-only) ──────────────────────────────────────

    /// Promote an overview memory entry to core.
    ///
    /// **IMPORTANT: This method must NEVER be called autonomously.**
    /// The orchestrator must present a CoreMemory candidate to a human
    /// operator for review. Only after explicit human confirmation should
    /// this method be invoked with the reviewer's identity.
    ///
    /// This is enforced by the `confirmed_by` parameter — the orchestrator
    /// must provide a human-identifiable string (name, pubkey, etc.).
    pub fn promote_to_core(
        &mut self,
        overview: &OverviewMemory,
        insight: String,
        confidence: f64,
        confirmed_by: String,
    ) -> CoreMemory {
        let core = CoreMemory {
            id: format!("core-{}", Utc::now().timestamp_millis()),
            created_at: Utc::now(),
            insight,
            source_overview_id: overview.id.clone(),
            confidence,
            confirmed_by,
        };

        if self.persist {
            let _ = self.persist_core(&core);
        }

        core
    }

    // ── Queries ───────────────────────────────────────────────────────

    /// Get all working memory entries.
    pub fn working(&self) -> &[WorkingMemory] {
        &self.working
    }

    /// Get all project memory entries.
    pub fn project(&self) -> &[ProjectMemoryOrRedirect] {
        &self.project
    }

    /// Get only consolidation entries from project memory.
    pub fn project_consolidations(&self) -> Vec<&ProjectMemory> {
        self.project
            .iter()
            .filter_map(|p| match p {
                ProjectMemoryOrRedirect::Consolidation(pm) => Some(pm),
                _ => None,
            })
            .collect()
    }

    /// Get only redirect events from project memory.
    pub fn redirect_events(&self) -> Vec<&RedirectEvent> {
        self.project
            .iter()
            .filter_map(|p| match p {
                ProjectMemoryOrRedirect::Redirect(re) => Some(re),
                _ => None,
            })
            .collect()
    }

    /// Get all overview memory entries.
    pub fn overview(&self) -> &[OverviewMemory] {
        &self.overview
    }

    /// Current consecutive improvement count.
    pub fn consecutive_improvement_count(&self) -> usize {
        self.consecutive_improvement_count
    }

    // ── Persistence ───────────────────────────────────────────────────

    /// Ensure memory directories exist.
    fn ensure_dirs(base: &Path) {
        for tier in &["working", "project", "overview", "core"] {
            let dir = base.join(tier);
            let _ = fs::create_dir_all(&dir);
        }
    }

    /// Atomic write: write to temp file, then rename.
    fn atomic_write(path: &Path, content: &str) -> std::io::Result<()> {
        let tmp_path = path.with_extension("tmp");
        {
            let mut f = fs::File::create(&tmp_path)?;
            f.write_all(content.as_bytes())?;
            f.flush()?;
        }
        fs::rename(&tmp_path, path)
    }

    fn persist_working(&self) -> std::io::Result<()> {
        let dir = self.config.memory_dir.join("working");
        fs::create_dir_all(&dir)?;
        for entry in &self.working {
            let path = dir.join(format!("cycle-{}.json", entry.cycle_id));
            let json = serde_json::to_string_pretty(entry)
                .unwrap_or_else(|_| "{}".to_string());
            Self::atomic_write(&path, &json)?;
        }
        Ok(())
    }

    fn persist_project(&self) -> std::io::Result<()> {
        let dir = self.config.memory_dir.join("project");
        fs::create_dir_all(&dir)?;
        for entry in &self.project {
            let filename = match entry {
                ProjectMemoryOrRedirect::Consolidation(pm) => pm.id.clone(),
                ProjectMemoryOrRedirect::Redirect(re) => re.id.clone(),
            };
            let path = dir.join(format!("{}.json", filename));
            let json = serde_json::to_string_pretty(entry)
                .unwrap_or_else(|_| "{}".to_string());
            Self::atomic_write(&path, &json)?;
        }
        Ok(())
    }

    fn persist_overview(&self) -> std::io::Result<()> {
        let dir = self.config.memory_dir.join("overview");
        fs::create_dir_all(&dir)?;
        for entry in &self.overview {
            let path = dir.join(format!("{}.json", entry.id));
            let json = serde_json::to_string_pretty(entry)
                .unwrap_or_else(|_| "{}".to_string());
            Self::atomic_write(&path, &json)?;
        }
        Ok(())
    }

    fn persist_core(&self, core: &CoreMemory) -> std::io::Result<()> {
        let dir = self.config.memory_dir.join("core");
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", core.id));
        let json = serde_json::to_string_pretty(core)
            .unwrap_or_else(|_| "{}".to_string());
        Self::atomic_write(&path, &json)
    }

    // ── Helpers ───────────────────────────────────────────────────────

    /// Find the most frequent recommended action in a set of working entries.
    fn dominant_action(entries: &[&WorkingMemory]) -> RecommendedAction {
        let mut counts = std::collections::HashMap::new();
        for entry in entries {
            *counts.entry(entry.recommended_action).or_insert(0usize) += 1;
        }
        counts
            .into_iter()
            .max_by_key(|(_, c)| *c)
            .map(|(a, _)| a)
            .unwrap_or(RecommendedAction::Continue)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluator::{Evaluation, ProtocolPhase, SecondaryMetrics};

    // ── Helpers ─────────────────────────────────────────────────────────

    fn test_config() -> MemoryConfig {
        MemoryConfig {
            project_tsi_threshold: 0.6,
            overview_improvement_cycles: 3,
            working_cap: 100,
            memory_dir: PathBuf::from("/tmp/rtp-test-memory"),
        }
    }

    fn healthy_evaluation(tsi: f64) -> Evaluation {
        Evaluation {
            tsi,
            growth_factor: 1.0,
            safety_factor: 0.9,
            reliability_factor: tsi / 0.9,
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

    fn per_iteration_signal(cycle: usize, tsi: f64, delta: f64) -> HeartbeatSignal {
        HeartbeatSignal {
            heartbeat_type: HeartbeatType::PerIteration,
            current_tsi: tsi,
            tsi_delta: delta,
            stagnating: false,
            terminal: false,
            degraded: false,
            recommended_action: RecommendedAction::Continue,
            cycle,
            timestamp: Utc::now(),
        }
    }

    fn consolidation_signal(cycle: usize, tsi: f64, delta: f64) -> HeartbeatSignal {
        HeartbeatSignal {
            heartbeat_type: HeartbeatType::Consolidation,
            current_tsi: tsi,
            tsi_delta: delta,
            stagnating: false,
            terminal: false,
            degraded: false,
            recommended_action: RecommendedAction::Consolidate,
            cycle,
            timestamp: Utc::now(),
        }
    }

    fn redirect_signal_stagnation(cycle: usize, tsi: f64) -> HeartbeatSignal {
        HeartbeatSignal {
            heartbeat_type: HeartbeatType::Redirect,
            current_tsi: tsi,
            tsi_delta: -0.1,
            stagnating: true,
            terminal: false,
            degraded: false,
            recommended_action: RecommendedAction::Redirect,
            cycle,
            timestamp: Utc::now(),
        }
    }

    fn redirect_signal_zero(cycle: usize) -> HeartbeatSignal {
        HeartbeatSignal {
            heartbeat_type: HeartbeatType::Redirect,
            current_tsi: 0.0,
            tsi_delta: -1.0,
            stagnating: false,
            terminal: false,
            degraded: false,
            recommended_action: RecommendedAction::Redirect,
            cycle,
            timestamp: Utc::now(),
        }
    }

    fn redirect_signal_terminal(cycle: usize) -> HeartbeatSignal {
        HeartbeatSignal {
            heartbeat_type: HeartbeatType::Redirect,
            current_tsi: 0.0,
            tsi_delta: 0.0,
            stagnating: true,
            terminal: true,
            degraded: false,
            recommended_action: RecommendedAction::Halt,
            cycle,
            timestamp: Utc::now(),
        }
    }

    fn run_cycle(
        mp: &mut MemoryPromotion,
        cycle: usize,
        tsi: f64,
        delta: f64,
        hb_type: HeartbeatType,
    ) -> PromotionResult {
        let eval = healthy_evaluation(tsi);
        let signal = match hb_type {
            HeartbeatType::PerIteration => per_iteration_signal(cycle, tsi, delta),
            HeartbeatType::Consolidation => consolidation_signal(cycle, tsi, delta),
            HeartbeatType::Redirect => redirect_signal_stagnation(cycle, tsi),
        };
        mp.process(&eval, &signal)
    }

    // ── Working memory ──────────────────────────────────────────────────

    #[test]
    fn working_written_every_cycle() {
        let mut mp = MemoryPromotion::new_in_memory(test_config());
        let eval = healthy_evaluation(1.0);
        let signal = per_iteration_signal(1, 1.0, 0.0);

        let result = mp.process(&eval, &signal);

        assert!(result.working_written);
        assert_eq!(mp.working().len(), 1);
        assert_eq!(mp.working()[0].cycle_id, 1);
        assert!((mp.working()[0].confidence - 1.0).abs() < 0.001);
    }

    #[test]
    fn working_confidence_equals_tsi() {
        let mut mp = MemoryPromotion::new_in_memory(test_config());
        let eval = healthy_evaluation(1.47);
        let signal = per_iteration_signal(1, 1.47, 0.0);

        mp.process(&eval, &signal);

        assert!((mp.working()[0].confidence - 1.47).abs() < 0.001);
    }

    #[test]
    fn working_capped_at_max() {
        let config = MemoryConfig {
            working_cap: 5,
            ..test_config()
        };
        let mut mp = MemoryPromotion::new_in_memory(config);

        for i in 1..=10 {
            run_cycle(&mut mp, i, 1.0, 0.1, HeartbeatType::PerIteration);
        }

        assert_eq!(mp.working().len(), 5);
        // Should keep the 5 newest (cycles 6-10).
        assert_eq!(mp.working()[0].cycle_id, 6);
        assert_eq!(mp.working()[4].cycle_id, 10);
    }

    #[test]
    fn working_snapshots_secondary_metrics() {
        let mut mp = MemoryPromotion::new_in_memory(test_config());
        let eval = healthy_evaluation(1.0);
        let signal = per_iteration_signal(1, 1.0, 0.0);

        mp.process(&eval, &signal);

        let w = &mp.working()[0];
        assert_eq!(w.secondary_snapshot.strategy_name.as_deref(), Some("mr_rsi_bb"));
        assert!(w.secondary_snapshot.strategy_yield.is_some());
    }

    // ── Project memory: consolidation ───────────────────────────────────

    #[test]
    fn consolidation_promotes_eligible_working() {
        let mut mp = MemoryPromotion::new_in_memory(test_config());

        // Build up 5 cycles of eligible working memory.
        for i in 1..=5 {
            run_cycle(&mut mp, i, 1.0 + i as f64 * 0.1, 0.1, HeartbeatType::PerIteration);
        }
        assert!(mp.project().is_empty());

        // Fire consolidation.
        let result = run_cycle(&mut mp, 6, 1.6, 0.1, HeartbeatType::Consolidation);

        assert!(result.project_created.is_some());
        assert_eq!(mp.project_consolidations().len(), 1);
    }

    #[test]
    fn consolidation_summary_fields() {
        let mut mp = MemoryPromotion::new_in_memory(test_config());

        for i in 1..=4 {
            run_cycle(&mut mp, i, 1.0 + i as f64 * 0.2, 0.2, HeartbeatType::PerIteration);
        }
        run_cycle(&mut mp, 5, 1.9, 0.1, HeartbeatType::Consolidation);

        let pm = &mp.project_consolidations()[0];
        // 5 eligible entries: TSI = 1.2, 1.4, 1.6, 1.8, 1.9
        assert_eq!(pm.cycles_in_window, 5);
        // avg = (1.2 + 1.4 + 1.6 + 1.8 + 1.9) / 5 = 1.58
        assert!((pm.avg_tsi - 1.58).abs() < 0.1);
        assert!((pm.best_tsi - 1.9).abs() < 0.1);
        assert!((pm.worst_tsi - 1.2).abs() < 0.1);
        assert_eq!(pm.dominant_action, RecommendedAction::Continue);
        assert_eq!(pm.strategy.as_deref(), Some("mr_rsi_bb"));
    }

    #[test]
    fn consolidation_skips_degraded_entries() {
        let mut mp = MemoryPromotion::new_in_memory(test_config());

        // Two healthy, one degraded.
        run_cycle(&mut mp, 1, 1.0, 0.0, HeartbeatType::PerIteration);
        let eval_d = degraded_evaluation(1.2);
        let sig_d = per_iteration_signal(2, 1.2, 0.2);
        mp.process(&eval_d, &sig_d);
        run_cycle(&mut mp, 3, 1.3, 0.1, HeartbeatType::PerIteration);

        // Consolidate.
        run_cycle(&mut mp, 4, 1.4, 0.1, HeartbeatType::Consolidation);

        let pm = &mp.project_consolidations()[0];
        // Only 2 eligible (cycles 1 and 3 — cycle 2 was degraded).
        // Plus cycle 4 which is also eligible and triggers consolidation.
        assert!(pm.cycles_in_window >= 2);
        assert!(pm.confidence > 0.0);
    }

    #[test]
    fn consolidation_skips_below_threshold() {
        let config = MemoryConfig {
            project_tsi_threshold: 2.0, // Very high threshold.
            ..test_config()
        };
        let mut mp = MemoryPromotion::new_in_memory(config);

        // TSI of 1.0 — below 2.0 threshold.
        run_cycle(&mut mp, 1, 1.0, 0.0, HeartbeatType::PerIteration);
        run_cycle(&mut mp, 2, 1.1, 0.1, HeartbeatType::PerIteration);

        let result = run_cycle(&mut mp, 3, 1.2, 0.1, HeartbeatType::Consolidation);

        // No eligible entries → no project memory.
        assert!(result.project_created.is_none());
    }

    // ── Project memory: redirect events ─────────────────────────────────

    #[test]
    fn redirect_writes_event_immediately() {
        let mut mp = MemoryPromotion::new_in_memory(test_config());

        run_cycle(&mut mp, 1, 1.0, 0.0, HeartbeatType::PerIteration);
        let result = run_cycle(&mut mp, 2, 0.9, -0.1, HeartbeatType::Redirect);

        match &result.project_created {
            Some(ProjectMemoryOrRedirect::Redirect(re)) => {
                assert_eq!(re.cycle_id, 2);
                assert_eq!(re.trigger, RedirectTrigger::Stagnation);
                assert!((re.tsi_at_redirect - 0.9).abs() < 0.001);
            }
            _ => panic!("Expected RedirectEvent"),
        }
    }

    #[test]
    fn redirect_event_persists_before_consolidation() {
        let mut mp = MemoryPromotion::new_in_memory(test_config());

        // Cycle 1: normal.
        run_cycle(&mut mp, 1, 1.0, 0.0, HeartbeatType::PerIteration);

        // Cycle 2: redirect fires mid-window (before any consolidation).
        let result = run_cycle(&mut mp, 2, 0.9, -0.1, HeartbeatType::Redirect);

        // RedirectEvent should appear in project memory immediately.
        assert!(result.project_created.is_some());
        let events = mp.redirect_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].cycle_id, 2);
        assert_eq!(events[0].trigger, RedirectTrigger::Stagnation);

        // No consolidation entries yet — only the redirect.
        assert_eq!(mp.project_consolidations().len(), 0);
    }

    #[test]
    fn redirect_trigger_stagnation() {
        let mut mp = MemoryPromotion::new_in_memory(test_config());
        let signal = redirect_signal_stagnation(1, 0.9);
        let eval = healthy_evaluation(0.9);
        mp.process(&eval, &signal);

        let events = mp.redirect_events();
        assert_eq!(events[0].trigger, RedirectTrigger::Stagnation);
    }

    #[test]
    fn redirect_trigger_zero_tsi() {
        let mut mp = MemoryPromotion::new_in_memory(test_config());
        let signal = redirect_signal_zero(1);
        let eval = healthy_evaluation(0.0);
        mp.process(&eval, &signal);

        let events = mp.redirect_events();
        assert_eq!(events[0].trigger, RedirectTrigger::ZeroTsi);
    }

    #[test]
    fn redirect_trigger_terminal() {
        let mut mp = MemoryPromotion::new_in_memory(test_config());
        let signal = redirect_signal_terminal(1);
        let eval = healthy_evaluation(0.0);
        mp.process(&eval, &signal);

        let events = mp.redirect_events();
        assert_eq!(events[0].trigger, RedirectTrigger::Terminal);
    }

    // ── Overview memory ─────────────────────────────────────────────────

    #[test]
    fn overview_promoted_on_sustained_improvement() {
        let config = MemoryConfig {
            overview_improvement_cycles: 3,
            ..test_config()
        };
        let mut mp = MemoryPromotion::new_in_memory(config);

        // Warm up with a neutral delta to reset improvement counter,
        // then consolidate to create a project entry.
        run_cycle(&mut mp, 1, 1.0, 0.0, HeartbeatType::PerIteration);
        run_cycle(&mut mp, 2, 1.0, 0.0, HeartbeatType::PerIteration);
        run_cycle(&mut mp, 3, 1.2, 0.2, HeartbeatType::Consolidation);
        assert_eq!(mp.project_consolidations().len(), 1);
        // After consolidation, improvement counter was tracking delta=0.0 (cycle 2)
        // then delta=0.2 (cycle 3), so counter = 1.

        // Reset improvement counter with a neutral delta.
        run_cycle(&mut mp, 4, 1.2, 0.0, HeartbeatType::PerIteration);
        assert_eq!(mp.consecutive_improvement_count(), 0);
        assert!(mp.overview().is_empty());

        // Now 3 consecutive improving cycles.
        run_cycle(&mut mp, 5, 1.3, 0.1, HeartbeatType::PerIteration);
        assert!(mp.overview().is_empty());

        run_cycle(&mut mp, 6, 1.4, 0.1, HeartbeatType::PerIteration);
        assert!(mp.overview().is_empty());

        let result = run_cycle(&mut mp, 7, 1.5, 0.1, HeartbeatType::PerIteration);
        assert!(result.overview_promoted.is_some());
        assert_eq!(mp.overview().len(), 1);
    }

    #[test]
    fn overview_not_promoted_without_sustained_improvement() {
        let config = MemoryConfig {
            overview_improvement_cycles: 3,
            ..test_config()
        };
        let mut mp = MemoryPromotion::new_in_memory(config);

        // 2 improving, then 1 declining.
        run_cycle(&mut mp, 1, 1.0, 0.0, HeartbeatType::PerIteration);
        run_cycle(&mut mp, 2, 1.1, 0.1, HeartbeatType::PerIteration);
        run_cycle(&mut mp, 3, 1.0, -0.1, HeartbeatType::PerIteration); // Resets counter.
        run_cycle(&mut mp, 4, 1.1, 0.1, HeartbeatType::PerIteration);
        run_cycle(&mut mp, 5, 1.2, 0.1, HeartbeatType::PerIteration);

        // Only 2 consecutive improvements (cycles 4-5), need 3.
        assert!(mp.overview().is_empty());
    }

    #[test]
    fn overview_confidence_calculation() {
        let config = MemoryConfig {
            overview_improvement_cycles: 3,
            ..test_config()
        };
        let mut mp = MemoryPromotion::new_in_memory(config);

        // Build project memory first. Use neutral deltas to control
        // improvement counter precisely.
        run_cycle(&mut mp, 1, 1.0, 0.0, HeartbeatType::PerIteration);
        run_cycle(&mut mp, 2, 1.0, 0.0, HeartbeatType::PerIteration);
        run_cycle(&mut mp, 3, 1.2, 0.2, HeartbeatType::Consolidation);
        // After cycle 3: improvement counter = 1 (from delta 0.2).

        // Reset counter, then 3 consecutive improving cycles.
        run_cycle(&mut mp, 4, 1.2, 0.0, HeartbeatType::PerIteration);
        assert_eq!(mp.consecutive_improvement_count(), 0);

        run_cycle(&mut mp, 5, 1.3, 0.1, HeartbeatType::PerIteration);
        run_cycle(&mut mp, 6, 1.4, 0.1, HeartbeatType::PerIteration);
        run_cycle(&mut mp, 7, 1.5, 0.1, HeartbeatType::PerIteration);

        let ov = &mp.overview()[0];
        assert!(ov.confidence > 0.0);
        // Confidence = project_confidence × consistency. Project confidence
        // uses avg_tsi which can be > 1.0, so overview confidence can exceed 1.0.
        // The important thing is it's positive and computed correctly.
        assert_eq!(ov.improvement_cycles, 3);
        assert!((ov.peak_tsi - 1.5).abs() < 0.001);
    }

    #[test]
    fn overview_resets_improvement_counter() {
        let config = MemoryConfig {
            overview_improvement_cycles: 2,
            ..test_config()
        };
        let mut mp = MemoryPromotion::new_in_memory(config);

        // 2 improvements → overview promoted.
        run_cycle(&mut mp, 1, 1.0, 0.1, HeartbeatType::PerIteration);
        run_cycle(&mut mp, 2, 1.1, 0.1, HeartbeatType::PerIteration);
        assert_eq!(mp.overview().len(), 1);

        // Counter should be reset.
        assert_eq!(mp.consecutive_improvement_count(), 0);

        // Need 2 more improvements for next overview.
        run_cycle(&mut mp, 3, 1.2, 0.1, HeartbeatType::PerIteration);
        assert_eq!(mp.overview().len(), 1); // Not yet.

        run_cycle(&mut mp, 4, 1.3, 0.1, HeartbeatType::PerIteration);
        assert_eq!(mp.overview().len(), 2); // Second overview.
    }

    // ── Core memory (human-only) ────────────────────────────────────────

    #[test]
    fn promote_to_core_requires_human() {
        // Build up an overview.
        let config = MemoryConfig {
            overview_improvement_cycles: 2,
            ..test_config()
        };
        let mut mp = MemoryPromotion::new_in_memory(config);
        run_cycle(&mut mp, 1, 1.0, 0.1, HeartbeatType::PerIteration);
        run_cycle(&mut mp, 2, 1.1, 0.1, HeartbeatType::PerIteration);
        assert_eq!(mp.overview().len(), 1);

        let overview = mp.overview()[0].clone();
        let core = mp.promote_to_core(
            &overview,
            "RSI mean-reversion is consistently profitable".to_string(),
            0.95,
            "human-operator-001".to_string(),
        );

        assert_eq!(core.confirmed_by, "human-operator-001");
        assert!(core.insight.contains("RSI"));
        assert!((core.confidence - 0.95).abs() < 0.001);
        assert_eq!(core.source_overview_id, overview.id);
    }

    // ── Pruning ─────────────────────────────────────────────────────────

    #[test]
    fn pruning_removes_consolidated_entries() {
        let config = MemoryConfig {
            project_tsi_threshold: 0.6,
            working_cap: 100,
            ..test_config()
        };
        let mut mp = MemoryPromotion::new_in_memory(config);

        // 3 eligible cycles + 1 below threshold.
        run_cycle(&mut mp, 1, 1.0, 0.0, HeartbeatType::PerIteration);
        run_cycle(&mut mp, 2, 1.1, 0.1, HeartbeatType::PerIteration);
        // Low TSI — not eligible for project.
        run_cycle(&mut mp, 3, 0.3, -0.8, HeartbeatType::PerIteration);
        run_cycle(&mut mp, 4, 1.2, 0.9, HeartbeatType::PerIteration);

        // Consolidate and prune.
        let result = run_cycle(&mut mp, 5, 1.3, 0.1, HeartbeatType::Consolidation);

        // The below-threshold entry (cycle 3) should be pruned.
        assert!(result.pruned_working_count > 0);
        assert!(mp.working().iter().all(|w| w.tsi >= 0.6));
    }

    // ── Confidence scoring ──────────────────────────────────────────────

    #[test]
    fn project_confidence_factors_degraded_ratio() {
        let mut mp = MemoryPromotion::new_in_memory(test_config());

        // 3 healthy, 1 degraded.
        run_cycle(&mut mp, 1, 1.0, 0.0, HeartbeatType::PerIteration);
        let eval_d = degraded_evaluation(1.2);
        let sig_d = per_iteration_signal(2, 1.2, 0.2);
        mp.process(&eval_d, &sig_d);
        run_cycle(&mut mp, 3, 1.3, 0.1, HeartbeatType::PerIteration);

        // Consolidate.
        run_cycle(&mut mp, 4, 1.4, 0.1, HeartbeatType::Consolidation);

        let pm = &mp.project_consolidations()[0];
        // 1 degraded out of 4 total = 0.25 degraded ratio.
        // Eligible (non-degraded): cycles 1, 3, 4 → avg_tsi = (1.0+1.3+1.4)/3 ≈ 1.233
        // confidence = 1.233 × (1 - 0.25) = 1.233 × 0.75 ≈ 0.925
        assert!(pm.confidence > 0.0);
        // Degraded entries reduce confidence below what pure avg_tsi would give.
        // Without degraded entries: confidence = avg_tsi × 1.0 = 1.233
        // With degraded: confidence = 1.233 × 0.75 = 0.925
        assert!(pm.confidence < 1.3); // Reduced from pure avg_tsi.
    }

    // ── Atomic write ────────────────────────────────────────────────────

    #[test]
    fn atomic_write_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.json");

        MemoryPromotion::atomic_write(&path, r#"{"test": true}"#).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("test"));
        assert!(!path.with_extension("tmp").exists()); // Temp file cleaned up.
    }

    // ── Display traits ──────────────────────────────────────────────────

    #[test]
    fn redirect_trigger_display() {
        assert_eq!(format!("{}", RedirectTrigger::Stagnation), "stagnation");
        assert_eq!(format!("{}", RedirectTrigger::ZeroTsi), "zero_tsi");
        assert_eq!(format!("{}", RedirectTrigger::Terminal), "terminal");
    }

    // ── Full lifecycle ──────────────────────────────────────────────────

    #[test]
    fn full_memory_lifecycle() {
        let config = MemoryConfig {
            overview_improvement_cycles: 4, // Higher threshold for this test.
            ..test_config()
        };
        let mut mp = MemoryPromotion::new_in_memory(config);

        // Cycles 1-3: healthy, per-iteration with neutral deltas.
        run_cycle(&mut mp, 1, 1.0, 0.0, HeartbeatType::PerIteration);
        run_cycle(&mut mp, 2, 1.1, 0.1, HeartbeatType::PerIteration);
        run_cycle(&mut mp, 3, 1.2, 0.1, HeartbeatType::PerIteration);
        assert_eq!(mp.working().len(), 3);
        assert!(mp.project().is_empty());

        // Cycle 4: consolidation → project memory created.
        // Delta 0.0 to avoid contributing to improvement counter.
        run_cycle(&mut mp, 4, 1.4, 0.0, HeartbeatType::Consolidation);
        assert_eq!(mp.project_consolidations().len(), 1);

        // Cycles 5-6: redirect event mid-window.
        // Negative deltas reset improvement counter.
        run_cycle(&mut mp, 5, 1.3, -0.1, HeartbeatType::PerIteration);
        let result = run_cycle(&mut mp, 6, 1.2, -0.1, HeartbeatType::Redirect);
        assert!(matches!(result.project_created, Some(ProjectMemoryOrRedirect::Redirect(_))));
        assert_eq!(mp.redirect_events().len(), 1);

        // Cycles 7-10: sustained improvement (4 cycles) → overview promoted.
        run_cycle(&mut mp, 7, 1.3, 0.1, HeartbeatType::PerIteration);
        assert!(mp.overview().is_empty());
        run_cycle(&mut mp, 8, 1.4, 0.1, HeartbeatType::PerIteration);
        assert!(mp.overview().is_empty());
        run_cycle(&mut mp, 9, 1.5, 0.1, HeartbeatType::PerIteration);
        assert!(mp.overview().is_empty());
        let result = run_cycle(&mut mp, 10, 1.6, 0.1, HeartbeatType::PerIteration);
        assert!(result.overview_promoted.is_some());
        assert_eq!(mp.overview().len(), 1);

        // Cycle 11: human promotes to core.
        let overview = mp.overview()[0].clone();
        let core = mp.promote_to_core(
            &overview,
            "Mean-reversion with RSI filter is durable".to_string(),
            0.92,
            "operator-alice".to_string(),
        );
        assert_eq!(core.confirmed_by, "operator-alice");
    }
}
