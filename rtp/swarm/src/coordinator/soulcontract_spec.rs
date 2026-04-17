//! SoulcontractSpec — machine-readable representation of soulcontract.md.
//! Drift detection compares parsed spec against Soulguard's active rules.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// A single soulcontract constraint, parsed from the markdown.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Constraint {
    /// Human-readable name (e.g. "pda_ownership", "human_sovereign_control").
    pub name: String,
    /// The section it came from ("What Cannot Evolve" or "Core Values").
    pub section: String,
    /// The raw markdown text describing this constraint.
    pub raw_text: String,
    /// Whether this constraint requires human consent to modify.
    pub requires_human_consent: bool,
}

/// A phase evolution rule, parsed from soulcontract.md.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhaseRule {
    pub name: String,
    pub threshold_usd: f64,
    pub description: String,
}

/// The full parsed soulcontract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulcontractSpec {
    /// All constraints from "What Cannot Evolve".
    pub immutable_constraints: Vec<Constraint>,
    /// All items from "What Can Evolve".
    pub evolvable_items: Vec<String>,
    /// Phase evolution rules.
    pub phases: Vec<PhaseRule>,
    /// The core values (1-5).
    pub core_values: Vec<String>,
    /// Rollback threshold (e.g. 0.05 for 5%).
    pub rollback_threshold: f64,
    /// Raw markdown content for drift detection.
    pub raw_content: String,
}

impl SoulcontractSpec {
    /// Parse soulcontract.md from a file path.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read soulcontract.md: {}", e))?;
        Self::from_str(&content)
    }

    /// Parse soulcontract.md from a string (for testing).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(content: &str) -> Result<Self, String> {
        let mut immutable_constraints = Vec::new();
        let mut evolvable_items = Vec::new();
        let mut phases = Vec::new();
        let mut core_values = Vec::new();
        let mut rollback_threshold = 0.05;

        let mut current_section = String::new();
        let mut in_table = false;

        for line in content.lines() {
            let trimmed = line.trim();

            // Detect sections.
            if trimmed == "## What Cannot Evolve" {
                current_section = "What Cannot Evolve".to_string();
                continue;
            } else if trimmed == "## What Can Evolve" {
                current_section = "What Can Evolve".to_string();
                continue;
            } else if trimmed.starts_with("## Core Values") {
                current_section = "Core Values".to_string();
                continue;
            } else if trimmed.starts_with("## Phase Evolution") {
                current_section = "Phase Evolution".to_string();
                in_table = false;
                continue;
            } else if trimmed == "## Amendment Protocol" {
                current_section = "Amendment Protocol".to_string();
                continue;
            }

            // Track table state.
            if trimmed.starts_with('|') && trimmed.contains("---") {
                in_table = true;
                continue;
            }
            if !trimmed.starts_with('|') {
                in_table = false;
            }

            // Parse rollback threshold from amendment protocol.
            if trimmed.contains("degrades > 5%") {
                rollback_threshold = 0.05;
            }

            match current_section.as_str() {
                "Core Values" => {
                    // Parse "N. The protocol..." lines.
                    if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit())
                        && let Some(value) = rest.trim_start().strip_prefix(". ")
                    {
                        core_values.push(value.trim_end_matches('.').to_string());
                    }
                }
                "What Cannot Evolve" => {
                    // Parse bullet lines like "- **PDA ownership** — ..."
                    if let Some(rest) = trimmed.strip_prefix("- **")
                        && let Some((name, desc)) = rest.split_once("**")
                    {
                        let name = name.trim();
                        let desc = desc.trim_start_matches(['-', '\u{2014}', '\u{2013}']);
                        let desc = desc.trim();
                        immutable_constraints.push(Constraint {
                            name: name.to_lowercase().replace([' ', '-'], "_"),
                            section: "What Cannot Evolve".to_string(),
                            raw_text: format!("{}: {}", name, desc),
                            requires_human_consent: true,
                        });
                    }
                }
                "What Can Evolve" => {
                    if let Some(item) = trimmed.strip_prefix("- ") {
                        evolvable_items.push(item.to_string());
                    }
                }
                "Phase Evolution" => {
                    if in_table && trimmed.starts_with('|') {
                        let cells: Vec<&str> = trimmed
                            .split('|')
                            .map(|c| c.trim())
                            .filter(|c| !c.is_empty() && !c.starts_with("---"))
                            .collect();
                        if cells.len() >= 3 {
                            let threshold = cells[1]
                                .replace(['$', 'k', ',', '<', '>'], "")
                                .trim()
                                .parse::<f64>()
                                .unwrap_or(0.0);
                            phases.push(PhaseRule {
                                name: cells[0].to_string(),
                                threshold_usd: threshold
                                    * if cells[1].contains("k") { 1_000.0 } else { 1.0 },
                                description: cells.get(2).unwrap_or(&"").to_string(),
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(Self {
            immutable_constraints,
            evolvable_items,
            phases,
            core_values,
            rollback_threshold,
            raw_content: content.to_string(),
        })
    }

    /// Get the set of immutable constraint names.
    pub fn constraint_names(&self) -> HashSet<String> {
        self.immutable_constraints
            .iter()
            .map(|c| c.name.clone())
            .collect()
    }

    /// Check if a given name is an immutable constraint.
    pub fn is_immutable(&self, name: &str) -> bool {
        self.immutable_constraints.iter().any(|c| c.name == name)
    }

    /// Get the rollback threshold.
    pub fn rollback_threshold(&self) -> f64 {
        self.rollback_threshold
    }

    /// Detect drift between this spec and a set of active constraint names.
    /// Returns (in_spec_but_not_active, active_but_not_in_spec).
    pub fn detect_drift(&self, active_constraints: &HashSet<String>) -> DriftReport {
        let spec_names = self.constraint_names();
        let missing_from_active: Vec<String> = spec_names
            .iter()
            .filter(|n| !active_constraints.contains(*n))
            .cloned()
            .collect();
        let extra_in_active: Vec<String> = active_constraints
            .iter()
            .filter(|n| !spec_names.contains(n.as_str()))
            .cloned()
            .collect();

        DriftReport {
            in_sync: missing_from_active.is_empty() && extra_in_active.is_empty(),
            missing_from_active,
            extra_in_active,
        }
    }
}

/// Result of drift detection between parsed spec and active constraints.
#[derive(Debug, Clone)]
pub struct DriftReport {
    /// True if spec and active constraints are perfectly in sync.
    pub in_sync: bool,
    /// Constraints in the spec but not enforced by Soulguard.
    pub missing_from_active: Vec<String>,
    /// Constraints enforced by Soulguard but not in the spec.
    pub extra_in_active: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_SOULCONTRACT: &str = r#"
# soulcontract.md

## Core Values

1. The protocol exists to generate sustainable yield
2. No single entity controls the treasury

## What Can Evolve

- Strategy parameters
- Risk thresholds

## What Cannot Evolve

- **Core values** — the five statements above are immutable
- **PDA ownership** — treasury must always be PDA-owned
- **Fee immutability** — SPL TransferFeeConfig must remain immutable

## Phase Evolution

| Phase | Threshold | New Capabilities |
|-------|-----------|-----------------|
| Sustenance | < $50k | Reinvest all yield |
| Ecosystem | $50k–$1M | Auto-provide LP |
| Humanity | > $1M | USDC grants |

## Amendment Protocol

Auto-rollback if system performance degrades > 5% post-amendment
"#;

    #[test]
    fn parse_core_values() {
        let spec = SoulcontractSpec::from_str(MINIMAL_SOULCONTRACT).unwrap();
        assert_eq!(spec.core_values.len(), 2);
        assert!(spec.core_values[0].contains("sustainable yield"));
    }

    #[test]
    fn parse_immutable_constraints() {
        let spec = SoulcontractSpec::from_str(MINIMAL_SOULCONTRACT).unwrap();
        assert!(spec.is_immutable("core_values"));
        assert!(spec.is_immutable("pda_ownership"));
        assert!(spec.is_immutable("fee_immutability"));
        assert!(!spec.is_immutable("risk_thresholds"));
    }

    #[test]
    fn parse_evolvable_items() {
        let spec = SoulcontractSpec::from_str(MINIMAL_SOULCONTRACT).unwrap();
        assert!(spec.evolvable_items.iter().any(|i| i.contains("Strategy")));
        assert!(spec.evolvable_items.iter().any(|i| i.contains("Risk")));
    }

    #[test]
    fn parse_phases() {
        let spec = SoulcontractSpec::from_str(MINIMAL_SOULCONTRACT).unwrap();
        assert_eq!(spec.phases.len(), 3);
        assert_eq!(spec.phases[0].name, "Sustenance");
        assert!((spec.phases[0].threshold_usd - 50_000.0).abs() < 1.0);
    }

    #[test]
    fn parse_rollback_threshold() {
        let spec = SoulcontractSpec::from_str(MINIMAL_SOULCONTRACT).unwrap();
        assert_eq!(spec.rollback_threshold, 0.05);
    }

    #[test]
    fn drift_detection_in_sync() {
        let spec = SoulcontractSpec::from_str(MINIMAL_SOULCONTRACT).unwrap();
        let active = spec.constraint_names();
        let report = spec.detect_drift(&active);
        assert!(report.in_sync);
    }

    #[test]
    fn drift_detection_missing_constraint() {
        let spec = SoulcontractSpec::from_str(MINIMAL_SOULCONTRACT).unwrap();
        let mut active = spec.constraint_names();
        active.remove("pda_ownership");
        let report = spec.detect_drift(&active);
        assert!(!report.in_sync);
        assert!(
            report
                .missing_from_active
                .contains(&"pda_ownership".to_string())
        );
    }

    #[test]
    fn drift_detection_extra_constraint() {
        let spec = SoulcontractSpec::from_str(MINIMAL_SOULCONTRACT).unwrap();
        let mut active = spec.constraint_names();
        active.insert("made_up_constraint".to_string());
        let report = spec.detect_drift(&active);
        assert!(!report.in_sync);
        assert!(
            report
                .extra_in_active
                .contains(&"made_up_constraint".to_string())
        );
    }

    #[test]
    fn parse_full_soulcontract() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap_or(Path::new("."))
            .join("soulcontract.md");
        if path.exists() {
            let spec = SoulcontractSpec::from_file(&path).unwrap();
            assert!(spec.immutable_constraints.len() >= 5);
            assert!(spec.core_values.len() >= 5);
            assert_eq!(spec.phases.len(), 3);
        }
    }
}
