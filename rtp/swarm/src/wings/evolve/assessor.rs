//! Assessor — benchmark wing performance, identify bottlenecks and regressions.
//! Treasury-native metric: (USDC yield / SOL reserves) × (1 - max drawdown) × wing_consistency.
//! Flags wings that degrade beyond the rollback threshold (5%).

use crate::types::WingId;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// An assessment result for a wing — returned by `assess()`.
#[derive(Debug, Clone)]
pub struct Assessment {
    pub wing: WingId,
    pub score: f64,
    pub bottlenecks: Vec<String>,
    pub recommendations: Vec<String>,
}

/// Treasury-native performance metric inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub wing: WingId,
    pub usdc_yield: f64,
    pub sol_reserves: f64,
    pub max_drawdown: f64,
    /// Consistency: ratio of successful operations (0.0 to 1.0).
    pub consistency: f64,
    /// Timestamp of when these metrics were captured.
    pub captured_at: DateTime<Utc>,
}

impl PerformanceMetrics {
    /// Calculate the treasury-native performance score.
    ///
    /// Formula: (USDC yield / SOL reserves) * (1 - max_drawdown) * consistency
    pub fn treasury_score(&self) -> f64 {
        if self.sol_reserves <= 0.0 {
            return 0.0;
        }
        let yield_ratio = self.usdc_yield / self.sol_reserves;
        let drawdown_factor = (1.0 - self.max_drawdown).max(0.0);
        yield_ratio * drawdown_factor * self.consistency
    }
}

/// A stored assessment for a wing.
#[derive(Debug, Clone)]
pub struct StoredAssessment {
    pub wing: WingId,
    pub score: f64,
    pub metrics: PerformanceMetrics,
    pub bottlenecks: Vec<String>,
    pub recommendations: Vec<String>,
    pub assessed_at: DateTime<Utc>,
}

/// The Assessor — evaluates wing performance and identifies regressions.
pub struct Assessor {
    /// Latest metrics per wing.
    metrics: Arc<DashMap<WingId, PerformanceMetrics>>,
    /// Historical assessments.
    assessments: Arc<DashMap<WingId, StoredAssessment>>,
    /// Baseline scores (set after first assessment).
    baselines: Arc<DashMap<WingId, f64>>,
    /// Rollback threshold (default 5%).
    rollback_threshold: f64,
}

impl Default for Assessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Assessor {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(DashMap::new()),
            assessments: Arc::new(DashMap::new()),
            baselines: Arc::new(DashMap::new()),
            rollback_threshold: 0.05,
        }
    }

    /// Record performance metrics for a wing.
    pub fn record_metrics(&self, metrics: PerformanceMetrics) {
        self.metrics.insert(metrics.wing, metrics);
    }

    /// Record a clone of performance metrics for a wing (does not take ownership).
    pub fn record_metrics_ref(&self, metrics: &PerformanceMetrics) {
        self.metrics.insert(metrics.wing, metrics.clone());
    }

    /// Assess a wing: compute score, detect regressions, identify bottlenecks.
    pub fn assess(&self, wing_id: WingId) -> Assessment {
        let metrics = self.metrics.get(&wing_id).map(|m| m.clone());

        let (score, bottlenecks, recommendations) = match &metrics {
            Some(m) => {
                let score = m.treasury_score();
                let mut bottlenecks = Vec::new();
                let mut recommendations = Vec::new();

                // Check consistency.
                if m.consistency < 0.5 {
                    bottlenecks.push(format!("Low consistency: {:.1}%", m.consistency * 100.0));
                    recommendations
                        .push("Investigate failure modes and add retry logic".to_string());
                }

                // Check drawdown.
                if m.max_drawdown > 0.2 {
                    bottlenecks.push(format!("High drawdown: {:.1}%", m.max_drawdown * 100.0));
                    recommendations.push(
                        "Review risk parameters and consider reducing position sizing".to_string(),
                    );
                }

                // Check yield relative to reserves.
                if m.sol_reserves > 0.0 {
                    let yield_pct = m.usdc_yield / m.sol_reserves;
                    if yield_pct < 0.01 {
                        bottlenecks.push("Yield below 1% of reserves".to_string());
                        recommendations.push(
                            "Consider strategy rebalancing or venue diversification".to_string(),
                        );
                    }
                }

                (score, bottlenecks, recommendations)
            }
            None => {
                let mut bottlenecks = vec!["No metrics recorded".to_string()];
                if !self.metrics.contains_key(&wing_id) {
                    bottlenecks.push("Wing has not reported metrics".to_string());
                }
                (
                    0.0,
                    bottlenecks,
                    vec!["Begin metric collection immediately".to_string()],
                )
            }
        };

        // Store assessment.
        let assessment = StoredAssessment {
            wing: wing_id,
            score,
            metrics: metrics.unwrap_or(PerformanceMetrics {
                wing: wing_id,
                usdc_yield: 0.0,
                sol_reserves: 0.0,
                max_drawdown: 0.0,
                consistency: 0.0,
                captured_at: Utc::now(),
            }),
            bottlenecks: bottlenecks.clone(),
            recommendations: recommendations.clone(),
            assessed_at: Utc::now(),
        };
        self.assessments.insert(wing_id, assessment);

        // Set baseline if this is the first assessment.
        if !self.baselines.contains_key(&wing_id) && score > 0.0 {
            self.baselines.insert(wing_id, score);
        }

        Assessment {
            wing: wing_id,
            score,
            bottlenecks,
            recommendations,
        }
    }

    /// Check if a wing has degraded beyond the rollback threshold
    /// relative to its baseline score.
    pub fn exceeds_rollback_threshold(&self, wing_id: WingId) -> bool {
        let current = self
            .assessments
            .get(&wing_id)
            .map(|a| a.score)
            .unwrap_or(0.0);

        if let Some(baseline_ref) = self.baselines.get(&wing_id) {
            let baseline = *baseline_ref;
            if baseline <= 0.0 {
                return false;
            }
            let degradation = (baseline - current) / baseline;
            degradation > self.rollback_threshold
        } else {
            false
        }
    }

    /// Get the latest assessment for a wing.
    pub fn latest_assessment(&self, wing_id: WingId) -> Option<StoredAssessment> {
        self.assessments.get(&wing_id).map(|a| a.clone())
    }

    /// Assess all wings that have reported metrics.
    pub fn assess_all(&self) -> Vec<Assessment> {
        let wings: Vec<WingId> = self.metrics.iter().map(|r| *r.key()).collect();
        wings.into_iter().map(|w| self.assess(w)).collect()
    }

    /// Get all wings that exceed the rollback threshold.
    pub fn wings_needing_rollback(&self) -> Vec<WingId> {
        let wings: Vec<WingId> = self.assessments.iter().map(|r| *r.key()).collect();
        wings
            .into_iter()
            .filter(|w| self.exceeds_rollback_threshold(*w))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_metrics() -> PerformanceMetrics {
        PerformanceMetrics {
            wing: WingId::Trading,
            usdc_yield: 5000.0,
            sol_reserves: 50000.0,
            max_drawdown: 0.05,
            consistency: 0.85,
            captured_at: Utc::now(),
        }
    }

    fn degraded_metrics() -> PerformanceMetrics {
        PerformanceMetrics {
            wing: WingId::Trading,
            usdc_yield: 500.0,
            sol_reserves: 50000.0,
            max_drawdown: 0.30,
            consistency: 0.30,
            captured_at: Utc::now(),
        }
    }

    #[test]
    fn treasury_score_calculation() {
        let m = good_metrics();
        // (5000/50000) * (1 - 0.05) * 0.85 = 0.1 * 0.95 * 0.85 = 0.08075
        let score = m.treasury_score();
        assert!((score - 0.08075).abs() < 0.0001);
    }

    #[test]
    fn zero_reserves_returns_zero() {
        let m = PerformanceMetrics {
            wing: WingId::Trading,
            usdc_yield: 1000.0,
            sol_reserves: 0.0,
            max_drawdown: 0.0,
            consistency: 1.0,
            captured_at: Utc::now(),
        };
        assert_eq!(m.treasury_score(), 0.0);
    }

    #[test]
    fn assess_healthy_wing() {
        let assessor = Assessor::new();
        assessor.record_metrics(good_metrics());
        let assessment = assessor.assess(WingId::Trading);
        assert!(assessment.score > 0.0);
        assert!(assessment.bottlenecks.is_empty());
    }

    #[test]
    fn assess_detects_low_consistency() {
        let assessor = Assessor::new();
        let mut m = good_metrics();
        m.consistency = 0.3;
        assessor.record_metrics(m);
        let assessment = assessor.assess(WingId::Trading);
        assert!(
            assessment
                .bottlenecks
                .iter()
                .any(|b| b.contains("Low consistency"))
        );
    }

    #[test]
    fn assess_detects_high_drawdown() {
        let assessor = Assessor::new();
        let mut m = good_metrics();
        m.max_drawdown = 0.25;
        assessor.record_metrics(m);
        let assessment = assessor.assess(WingId::Trading);
        assert!(
            assessment
                .bottlenecks
                .iter()
                .any(|b| b.contains("High drawdown"))
        );
    }

    #[test]
    fn baseline_and_rollback_detection() {
        let assessor = Assessor::new();

        // First assessment sets baseline.
        assessor.record_metrics(good_metrics());
        let _ = assessor.assess(WingId::Trading);
        assert!(!assessor.exceeds_rollback_threshold(WingId::Trading));

        // Second assessment with degraded metrics.
        assessor.record_metrics(degraded_metrics());
        let _ = assessor.assess(WingId::Trading);
        assert!(assessor.exceeds_rollback_threshold(WingId::Trading));
    }

    #[test]
    fn assess_no_metrics() {
        let assessor = Assessor::new();
        let assessment = assessor.assess(WingId::Knowledge);
        assert_eq!(assessment.score, 0.0);
        assert!(!assessment.bottlenecks.is_empty());
    }

    #[test]
    fn assess_all_wings() {
        let assessor = Assessor::new();
        assessor.record_metrics(good_metrics());

        let mut sec_metrics = good_metrics();
        sec_metrics.wing = WingId::Security;
        assessor.record_metrics(sec_metrics);

        let results = assessor.assess_all();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn wings_needing_rollback() {
        let assessor = Assessor::new();
        assessor.record_metrics(good_metrics());
        let _ = assessor.assess(WingId::Trading);

        assert!(assessor.wings_needing_rollback().is_empty());

        assessor.record_metrics(degraded_metrics());
        let _ = assessor.assess(WingId::Trading);

        let rollback_wings = assessor.wings_needing_rollback();
        assert!(rollback_wings.contains(&WingId::Trading));
    }
}
