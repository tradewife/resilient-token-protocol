//! Lifecycle — wing spawn, health-check, and retire.
//! Registers new wings, monitors health via heartbeats, retires unhealthy wings.
//! - Tracks wing uptime and performance metrics

use crate::types::{HealthStatus, WingId, WingRegistration};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

/// Health check configuration.
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// How often to check wing health.
    pub check_interval: Duration,
    /// How long before a missed heartbeat is considered degraded.
    pub degraded_after: Duration,
    /// How long before a degraded wing is considered unhealthy.
    pub unhealthy_after: Duration,
    /// How long before an unhealthy wing is retired.
    pub retire_after: Duration,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            degraded_after: Duration::from_secs(60),
            unhealthy_after: Duration::from_secs(120),
            retire_after: Duration::from_secs(300),
        }
    }
}

/// A wing's runtime state managed by the lifecycle.
pub struct WingState {
    pub registration: WingRegistration,
    /// Whether the wing has been retired.
    pub retired: bool,
    pub retired_at: Option<DateTime<Utc>>,
}

/// The lifecycle manager for the swarm.
///
/// Spawns wings, monitors health, and retires failed ones.
/// All wing registration and deregistration flows through here.
pub struct LifecycleManager {
    /// All registered wings keyed by WingId.
    wings: Arc<DashMap<WingId, WingState>>,
    /// Health check configuration.
    health_config: HealthConfig,
    /// Callback when a wing is retired.
    #[allow(clippy::type_complexity)]
    on_retire: Arc<dyn Fn(WingId, &str) + Send + Sync>,
}

impl LifecycleManager {
    pub fn new(health_config: HealthConfig) -> Self {
        Self {
            wings: Arc::new(DashMap::new()),
            health_config,
            on_retire: Arc::new(|wing, reason| {
                warn!(wing = %wing, reason, "Wing retired");
            }),
        }
    }

    /// Create with default health config.
    pub fn with_defaults() -> Self {
        Self::new(HealthConfig::default())
    }

    /// Spawn (register) a new wing with the swarm.
    pub fn spawn(&self, wing_id: WingId) -> Result<WingRegistration, String> {
        if self.wings.contains_key(&wing_id) {
            return Err(format!("Wing {} is already registered", wing_id));
        }

        let now = Utc::now();
        let registration = WingRegistration {
            id: wing_id,
            registered_at: now,
            last_heartbeat: now,
            status: HealthStatus::Healthy,
        };

        let state = WingState {
            registration: registration.clone(),
            retired: false,
            retired_at: None,
        };

        self.wings.insert(wing_id, state);
        info!(wing = %wing_id, "Wing spawned");
        Ok(registration)
    }

    /// Record a heartbeat from a wing.
    pub fn heartbeat(
        &self,
        wing_id: WingId,
        status: HealthStatus,
        _metrics: serde_json::Value,
    ) -> Result<(), String> {
        let mut wing = self
            .wings
            .get_mut(&wing_id)
            .ok_or_else(|| format!("Wing {} not registered", wing_id))?;

        if wing.retired {
            return Err(format!("Wing {} is retired", wing_id));
        }

        wing.registration.last_heartbeat = Utc::now();
        wing.registration.status = status.clone();
        Ok(())
    }

    /// Check health of all registered wings and update statuses.
    pub fn check_health(&self) -> Vec<(WingId, HealthStatus)> {
        let now = Utc::now();
        let mut results = Vec::new();

        for mut entry in self.wings.iter_mut() {
            let wing_id = *entry.key();
            let state = entry.value_mut();

            if state.retired {
                continue;
            }

            let since_last = now
                .signed_duration_since(state.registration.last_heartbeat)
                .to_std()
                .unwrap_or(Duration::ZERO);

            let new_status = if since_last >= self.health_config.retire_after {
                // Retire the wing.
                state.retired = true;
                state.retired_at = Some(now);
                (self.on_retire)(wing_id, "Heartbeat timeout — retired");
                HealthStatus::Offline
            } else if since_last >= self.health_config.unhealthy_after {
                HealthStatus::Unhealthy {
                    reason: format!("No heartbeat for {} seconds", since_last.as_secs()),
                }
            } else if since_last >= self.health_config.degraded_after {
                HealthStatus::Degraded {
                    reason: format!("No heartbeat for {} seconds", since_last.as_secs()),
                }
            } else {
                HealthStatus::Healthy
            };

            state.registration.status = new_status.clone();
            results.push((wing_id, new_status));
        }

        results
    }

    /// Gracefully retire a wing.
    pub fn retire(&self, wing_id: &WingId, reason: &str) -> Result<(), String> {
        let mut wing = self
            .wings
            .get_mut(wing_id)
            .ok_or_else(|| format!("Wing {} not registered", wing_id))?;

        wing.retired = true;
        wing.retired_at = Some(Utc::now());
        wing.registration.status = HealthStatus::Offline;
        (self.on_retire)(*wing_id, reason);
        info!(wing = %wing_id, reason, "Wing retired by request");
        Ok(())
    }

    /// Get registration info for a wing.
    pub fn get_wing(&self, wing_id: &WingId) -> Option<WingRegistration> {
        self.wings.get(wing_id).map(|w| w.registration.clone())
    }

    /// List all registered (non-retired) wings.
    pub fn active_wings(&self) -> Vec<WingId> {
        self.wings
            .iter()
            .filter(|entry| !entry.value().retired)
            .map(|entry| *entry.key())
            .collect()
    }

    /// List all wings including retired ones.
    pub fn all_wings(&self) -> Vec<WingRegistration> {
        self.wings
            .iter()
            .map(|entry| entry.value().registration.clone())
            .collect()
    }

    /// Check if a wing is active (registered and not retired).
    pub fn is_active(&self, wing_id: &WingId) -> bool {
        self.wings.get(wing_id).map(|w| !w.retired).unwrap_or(false)
    }

    /// Count of active wings.
    pub fn active_count(&self) -> usize {
        self.wings.iter().filter(|e| !e.value().retired).count()
    }

    /// Set a custom retirement callback (for testing or integration).
    pub fn set_on_retire<F>(&mut self, f: F)
    where
        F: Fn(WingId, &str) + Send + Sync + 'static,
    {
        self.on_retire = Arc::new(f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_wing() {
        let lc = LifecycleManager::with_defaults();
        let reg = lc.spawn(WingId::Trading).unwrap();
        assert_eq!(reg.id, WingId::Trading);
        assert_eq!(reg.status, HealthStatus::Healthy);
    }

    #[test]
    fn duplicate_spawn_fails() {
        let lc = LifecycleManager::with_defaults();
        lc.spawn(WingId::Trading).unwrap();
        assert!(lc.spawn(WingId::Trading).is_err());
    }

    #[test]
    fn heartbeat_updates_status() {
        let lc = LifecycleManager::with_defaults();
        lc.spawn(WingId::Security).unwrap();

        lc.heartbeat(
            WingId::Security,
            HealthStatus::Degraded {
                reason: "high latency".to_string(),
            },
            serde_json::json!({}),
        )
        .unwrap();

        let wing = lc.get_wing(&WingId::Security).unwrap();
        assert!(matches!(wing.status, HealthStatus::Degraded { .. }));
    }

    #[test]
    fn heartbeat_on_unknown_wing_fails() {
        let lc = LifecycleManager::with_defaults();
        assert!(
            lc.heartbeat(
                WingId::Trading,
                HealthStatus::Healthy,
                serde_json::json!({})
            )
            .is_err()
        );
    }

    #[test]
    fn retire_wing() {
        let lc = LifecycleManager::with_defaults();
        lc.spawn(WingId::Evolve).unwrap();
        assert!(lc.is_active(&WingId::Evolve));

        lc.retire(&WingId::Evolve, "test retirement").unwrap();
        assert!(!lc.is_active(&WingId::Evolve));

        let wing = lc.get_wing(&WingId::Evolve).unwrap();
        assert!(matches!(wing.status, HealthStatus::Offline));
        assert!(!lc.is_active(&WingId::Evolve));
    }

    #[test]
    fn active_wings_list() {
        let lc = LifecycleManager::with_defaults();
        lc.spawn(WingId::Trading).unwrap();
        lc.spawn(WingId::Security).unwrap();
        lc.spawn(WingId::Audit).unwrap();

        assert_eq!(lc.active_count(), 3);

        lc.retire(&WingId::Security, "test").unwrap();
        assert_eq!(lc.active_count(), 2);
        assert_eq!(lc.active_wings().len(), 2);
    }

    #[test]
    fn health_check_detects_stale() {
        // Use very short timeouts for testing.
        let lc = LifecycleManager::new(HealthConfig {
            check_interval: Duration::from_millis(10),
            degraded_after: Duration::from_millis(50),
            unhealthy_after: Duration::from_millis(100),
            retire_after: Duration::from_secs(3600), // Don't auto-retire in this test.
        });

        lc.spawn(WingId::Trading).unwrap();

        // Immediately after spawn, wing is healthy.
        let results = lc.check_health();
        assert!(results.len() == 1);
        assert!(matches!(results[0].1, HealthStatus::Healthy));

        // Sleep past degraded threshold.
        std::thread::sleep(Duration::from_millis(80));
        let results = lc.check_health();
        assert!(matches!(results[0].1, HealthStatus::Degraded { .. }));
    }

    #[test]
    fn retire_nonexistent_fails() {
        let lc = LifecycleManager::with_defaults();
        assert!(lc.retire(&WingId::Trading, "nope").is_err());
    }
}
