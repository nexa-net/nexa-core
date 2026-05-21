use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::domain::models::RestartPolicy;

/// Maximum backoff delay between restart attempts.
pub const MAX_BACKOFF: Duration = Duration::from_secs(300);

/// Base delay for the first restart (before exponential growth).
pub const BASE_DELAY: Duration = Duration::from_secs(1);

/// Duration a pod must stay healthy before its restart count resets.
pub const HEALTHY_RESET_WINDOW: Duration = Duration::from_secs(600);

/// Number of restarts that triggers crash-loop detection.
pub const CRASH_LOOP_THRESHOLD: u32 = 10;

/// Returns `true` when the given policy permits a restart for the given exit code.
pub fn should_restart(policy: &RestartPolicy, exit_code: i64) -> bool {
    match policy {
        RestartPolicy::Always => true,
        RestartPolicy::OnFailure => exit_code != 0,
        RestartPolicy::Never => false,
    }
}

/// Computes the backoff delay for a given restart count using capped
/// exponential backoff: `min(MAX_BACKOFF, BASE_DELAY * 2^count)`.
pub fn backoff_delay(restart_count: u32) -> Duration {
    let delay = BASE_DELAY.saturating_mul(2u32.saturating_pow(restart_count));
    if delay > MAX_BACKOFF {
        MAX_BACKOFF
    } else {
        delay
    }
}

/// Per-pod restart tracking state.
#[derive(Debug, Clone)]
pub struct RestartState {
    pub count: u32,
    pub last_restart: Option<DateTime<Utc>>,
    pub last_healthy_since: Option<DateTime<Utc>>,
}

impl RestartState {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_restart: None,
            last_healthy_since: None,
        }
    }

    /// Returns `true` when the pod has been continuously healthy long enough
    /// to reset its restart counter.
    pub fn should_reset_count(&self, now: DateTime<Utc>) -> bool {
        if let Some(since) = self.last_healthy_since {
            let elapsed = (now - since).to_std().unwrap_or(Duration::ZERO);
            elapsed >= HEALTHY_RESET_WINDOW
        } else {
            false
        }
    }

    /// Resets the restart counter if the pod has been healthy long enough.
    pub fn reset_if_healthy(&mut self, now: DateTime<Utc>) {
        if self.should_reset_count(now) {
            self.count = 0;
            self.last_restart = None;
        }
    }

    /// Records that the pod is now healthy.
    pub fn mark_healthy(&mut self, now: DateTime<Utc>) {
        if self.last_healthy_since.is_none() {
            self.last_healthy_since = Some(now);
        }
    }

    /// Records that the pod is now unhealthy.
    pub fn mark_unhealthy(&mut self) {
        self.last_healthy_since = None;
    }

    /// Returns `true` when the restart count has reached the crash-loop threshold.
    pub fn is_crash_loop(&self) -> bool {
        self.count >= CRASH_LOOP_THRESHOLD
    }

    /// Increments the restart counter and records the timestamp.
    pub fn record_restart(&mut self, now: DateTime<Utc>) {
        self.count += 1;
        self.last_restart = Some(now);
        self.last_healthy_since = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta;

    #[test]
    fn should_restart_always_policy() {
        assert!(should_restart(&RestartPolicy::Always, 0));
        assert!(should_restart(&RestartPolicy::Always, 1));
        assert!(should_restart(&RestartPolicy::Always, 137));
    }

    #[test]
    fn should_restart_never_policy() {
        assert!(!should_restart(&RestartPolicy::Never, 0));
        assert!(!should_restart(&RestartPolicy::Never, 1));
    }

    #[test]
    fn should_restart_on_failure_policy() {
        assert!(!should_restart(&RestartPolicy::OnFailure, 0));
        assert!(should_restart(&RestartPolicy::OnFailure, 1));
        assert!(should_restart(&RestartPolicy::OnFailure, 137));
    }

    #[test]
    fn backoff_delay_first_restart() {
        assert_eq!(backoff_delay(0), Duration::from_secs(1));
    }

    #[test]
    fn backoff_delay_grows_exponentially() {
        assert_eq!(backoff_delay(1), Duration::from_secs(2));
        assert_eq!(backoff_delay(2), Duration::from_secs(4));
        assert_eq!(backoff_delay(3), Duration::from_secs(8));
    }

    #[test]
    fn backoff_delay_caps_at_max() {
        assert_eq!(backoff_delay(20), MAX_BACKOFF);
    }

    #[test]
    fn restart_state_new_has_zero_count() {
        let state = RestartState::new();
        assert_eq!(state.count, 0);
        assert!(state.last_restart.is_none());
        assert!(state.last_healthy_since.is_none());
    }

    #[test]
    fn record_restart_increments_count() {
        let mut state = RestartState::new();
        let now = Utc::now();
        state.record_restart(now);
        assert_eq!(state.count, 1);
        assert_eq!(state.last_restart, Some(now));
    }

    #[test]
    fn crash_loop_detection() {
        let mut state = RestartState::new();
        let now = Utc::now();
        for _ in 0..CRASH_LOOP_THRESHOLD {
            state.record_restart(now);
        }
        assert!(state.is_crash_loop());
    }

    #[test]
    fn not_crash_loop_below_threshold() {
        let mut state = RestartState::new();
        let now = Utc::now();
        for _ in 0..(CRASH_LOOP_THRESHOLD - 1) {
            state.record_restart(now);
        }
        assert!(!state.is_crash_loop());
    }

    #[test]
    fn healthy_reset_window() {
        let mut state = RestartState::new();
        let now = Utc::now();
        state.record_restart(now);
        state.mark_healthy(now);

        // Not enough time has passed
        assert!(!state.should_reset_count(now));

        // After the window
        let future = now + TimeDelta::seconds(HEALTHY_RESET_WINDOW.as_secs() as i64 + 1);
        assert!(state.should_reset_count(future));
    }

    #[test]
    fn reset_if_healthy_clears_count() {
        let mut state = RestartState::new();
        let now = Utc::now();
        state.record_restart(now);
        state.record_restart(now);
        assert_eq!(state.count, 2);

        state.mark_healthy(now);
        let future = now + TimeDelta::seconds(HEALTHY_RESET_WINDOW.as_secs() as i64 + 1);
        state.reset_if_healthy(future);
        assert_eq!(state.count, 0);
        assert!(state.last_restart.is_none());
    }

    #[test]
    fn mark_unhealthy_clears_healthy_since() {
        let mut state = RestartState::new();
        let now = Utc::now();
        state.mark_healthy(now);
        assert!(state.last_healthy_since.is_some());
        state.mark_unhealthy();
        assert!(state.last_healthy_since.is_none());
    }
}
