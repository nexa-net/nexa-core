use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum HealthState {
    Healthy,
    Failing { consecutive_failures: u32 },
    Unhealthy,
}

#[derive(Debug, Clone)]
pub struct PodHealthConfig {
    pub pod_id: Uuid,
    pub container_ip: String,
    pub port: u16,
    pub path: String,
    pub interval: Duration,
    pub timeout: Duration,
    pub retries: u32,
}

#[derive(Debug)]
pub struct HealthTracker {
    states: HashMap<Uuid, HealthState>,
    configs: HashMap<Uuid, PodHealthConfig>,
    last_probe: HashMap<Uuid, Instant>,
}

impl HealthTracker {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            configs: HashMap::new(),
            last_probe: HashMap::new(),
        }
    }

    pub fn register(&mut self, config: PodHealthConfig) {
        let id = config.pod_id;
        self.configs.insert(id, config);
        self.states.insert(id, HealthState::Healthy);
        self.last_probe.insert(id, Instant::now());
    }

    pub fn unregister(&mut self, pod_id: &Uuid) {
        self.states.remove(pod_id);
        self.configs.remove(pod_id);
        self.last_probe.remove(pod_id);
    }

    pub fn state(&self, pod_id: &Uuid) -> Option<&HealthState> {
        self.states.get(pod_id)
    }

    pub fn config(&self, pod_id: &Uuid) -> Option<&PodHealthConfig> {
        self.configs.get(pod_id)
    }

    pub fn pods_due_for_probe(&self) -> Vec<Uuid> {
        self.configs
            .iter()
            .filter(|(id, config)| {
                let state = self.states.get(id);
                if matches!(state, Some(HealthState::Unhealthy)) {
                    return false;
                }
                let last = self.last_probe.get(id);
                match last {
                    Some(t) => t.elapsed() > config.interval,
                    None => true,
                }
            })
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn mark_probed(&mut self, pod_id: &Uuid) {
        self.last_probe.insert(*pod_id, Instant::now());
    }

    pub fn record_result(&mut self, pod_id: &Uuid, healthy: bool) -> Option<(HealthState, bool)> {
        let retries = self.configs.get(pod_id)?.retries;
        let current = self.states.get(pod_id)?.clone();

        let (new_state, trigger_restart) = match (current, healthy) {
            (HealthState::Healthy, true) => (HealthState::Healthy, false),
            (HealthState::Healthy, false) => (
                HealthState::Failing {
                    consecutive_failures: 1,
                },
                false,
            ),
            (HealthState::Failing { .. }, true) => (HealthState::Healthy, false),
            (
                HealthState::Failing {
                    consecutive_failures: n,
                },
                false,
            ) => {
                if n + 1 >= retries {
                    (HealthState::Unhealthy, true)
                } else {
                    (
                        HealthState::Failing {
                            consecutive_failures: n + 1,
                        },
                        false,
                    )
                }
            }
            (HealthState::Unhealthy, _) => (HealthState::Unhealthy, false),
        };

        self.states.insert(*pod_id, new_state.clone());
        Some((new_state, trigger_restart))
    }

    pub fn reset(&mut self, pod_id: &Uuid) {
        if self.states.contains_key(pod_id) {
            self.states.insert(*pod_id, HealthState::Healthy);
        }
    }

    pub fn tracked_pods(&self) -> Vec<Uuid> {
        self.configs.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    fn make_config(pod_id: Uuid, retries: u32, interval: Duration) -> PodHealthConfig {
        PodHealthConfig {
            pod_id,
            container_ip: "127.0.0.1".into(),
            port: 8080,
            path: "/health".into(),
            interval,
            timeout: Duration::from_secs(1),
            retries,
        }
    }

    #[test]
    fn register_starts_healthy() {
        let mut tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.register(make_config(id, 3, Duration::from_secs(10)));
        assert_eq!(tracker.state(&id), Some(&HealthState::Healthy));
    }

    #[test]
    fn success_keeps_healthy() {
        let mut tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.register(make_config(id, 3, Duration::from_secs(10)));
        let (state, restart) = tracker.record_result(&id, true).unwrap();
        assert_eq!(state, HealthState::Healthy);
        assert!(!restart);
    }

    #[test]
    fn single_failure_transitions_to_failing() {
        let mut tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.register(make_config(id, 3, Duration::from_secs(10)));
        let (state, restart) = tracker.record_result(&id, false).unwrap();
        assert_eq!(
            state,
            HealthState::Failing {
                consecutive_failures: 1
            }
        );
        assert!(!restart);
    }

    #[test]
    fn consecutive_failures_increment() {
        let mut tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.register(make_config(id, 5, Duration::from_secs(10)));
        tracker.record_result(&id, false).unwrap();
        let (state, restart) = tracker.record_result(&id, false).unwrap();
        assert_eq!(
            state,
            HealthState::Failing {
                consecutive_failures: 2
            }
        );
        assert!(!restart);
    }

    #[test]
    fn reaching_retries_transitions_to_unhealthy() {
        let mut tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.register(make_config(id, 3, Duration::from_secs(10)));
        tracker.record_result(&id, false).unwrap();
        tracker.record_result(&id, false).unwrap();
        let (state, restart) = tracker.record_result(&id, false).unwrap();
        assert_eq!(state, HealthState::Unhealthy);
        assert!(restart);
    }

    #[test]
    fn success_resets_from_failing_to_healthy() {
        let mut tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.register(make_config(id, 3, Duration::from_secs(10)));
        tracker.record_result(&id, false).unwrap();
        let (state, restart) = tracker.record_result(&id, true).unwrap();
        assert_eq!(state, HealthState::Healthy);
        assert!(!restart);
    }

    #[test]
    fn unhealthy_does_not_re_trigger_restart() {
        let mut tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.register(make_config(id, 2, Duration::from_secs(10)));
        tracker.record_result(&id, false).unwrap();
        tracker.record_result(&id, false).unwrap();
        // Now Unhealthy — another failure should not trigger restart
        let (state, restart) = tracker.record_result(&id, false).unwrap();
        assert_eq!(state, HealthState::Unhealthy);
        assert!(!restart);
    }

    #[test]
    fn reset_restores_healthy() {
        let mut tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.register(make_config(id, 2, Duration::from_secs(10)));
        tracker.record_result(&id, false).unwrap();
        tracker.record_result(&id, false).unwrap();
        assert_eq!(tracker.state(&id), Some(&HealthState::Unhealthy));
        tracker.reset(&id);
        assert_eq!(tracker.state(&id), Some(&HealthState::Healthy));
    }

    #[test]
    fn unregister_removes_all_tracking() {
        let mut tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.register(make_config(id, 3, Duration::from_secs(10)));
        tracker.unregister(&id);
        assert!(tracker.state(&id).is_none());
        assert!(tracker.config(&id).is_none());
        assert!(tracker.tracked_pods().is_empty());
    }

    #[test]
    fn pods_due_for_probe_respects_interval() {
        let mut tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.register(make_config(id, 3, Duration::from_millis(50)));
        // Just registered — not yet due
        assert!(tracker.pods_due_for_probe().is_empty());
        thread::sleep(Duration::from_millis(60));
        assert!(tracker.pods_due_for_probe().contains(&id));
        tracker.mark_probed(&id);
        assert!(tracker.pods_due_for_probe().is_empty());
    }

    #[test]
    fn unhealthy_pods_are_not_probed() {
        let mut tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.register(make_config(id, 2, Duration::from_millis(50)));
        tracker.record_result(&id, false).unwrap();
        tracker.record_result(&id, false).unwrap();
        assert_eq!(tracker.state(&id), Some(&HealthState::Unhealthy));
        thread::sleep(Duration::from_millis(60));
        assert!(!tracker.pods_due_for_probe().contains(&id));
    }

    #[test]
    fn record_result_for_unknown_pod_returns_none() {
        let mut tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        assert!(tracker.record_result(&id, true).is_none());
    }
}
