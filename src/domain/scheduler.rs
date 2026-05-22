use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{NexaError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchedulerWeights {
    pub cpu: f64,
    pub memory: f64,
    pub load: f64,
    pub failure: f64,
}

impl SchedulerWeights {
    pub fn spread() -> Self {
        Self { cpu: 0.35, memory: 0.35, load: 0.15, failure: 0.15 }
    }

    pub fn binpack() -> Self {
        Self { cpu: -0.30, memory: -0.30, load: -0.10, failure: 0.15 }
    }
}

impl Default for SchedulerWeights {
    fn default() -> Self {
        Self::spread()
    }
}

#[derive(Debug, Clone)]
pub struct NodeSnapshot {
    pub node_id: Uuid,
    pub cpu_available: f64,
    pub cpu_total: f64,
    pub memory_available: u64,
    pub memory_total: u64,
    pub running_pods: u32,
    pub max_pods: u32,
    pub recent_failures: Vec<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct PodRequest {
    pub cpu_request: f64,
    pub memory_request: u64,
}

impl Default for PodRequest {
    fn default() -> Self {
        Self { cpu_request: 0.0, memory_request: 0 }
    }
}

pub fn failure_penalty(failures: &[DateTime<Utc>], now: DateTime<Utc>) -> f64 {
    failures
        .iter()
        .map(|t| {
            let age_minutes = (now - *t).num_minutes() as f64;
            (-age_minutes / 10.0).exp()
        })
        .sum::<f64>()
        .min(1.0)
}

pub struct WeightedScheduler {
    weights: SchedulerWeights,
}

impl WeightedScheduler {
    pub fn new(weights: SchedulerWeights) -> Self {
        Self { weights }
    }

    pub fn weights(&self) -> &SchedulerWeights {
        &self.weights
    }

    pub fn score_node(&self, request: &PodRequest, node: &NodeSnapshot) -> f64 {
        if node.running_pods >= node.max_pods {
            return f64::NEG_INFINITY;
        }
        if request.cpu_request > 0.0 && node.cpu_available < request.cpu_request {
            return f64::NEG_INFINITY;
        }
        if request.memory_request > 0 && node.memory_available < request.memory_request {
            return f64::NEG_INFINITY;
        }

        let cpu_after = node.cpu_available - request.cpu_request;
        let mem_after = node.memory_available.saturating_sub(request.memory_request);

        let cpu_ratio = if node.cpu_total > 0.0 { cpu_after / node.cpu_total } else { 0.0 };
        let mem_ratio = if node.memory_total > 0 { mem_after as f64 / node.memory_total as f64 } else { 0.0 };
        let load_ratio = if node.max_pods > 0 { node.running_pods as f64 / node.max_pods as f64 } else { 1.0 };

        let fail_penalty = failure_penalty(&node.recent_failures, Utc::now());

        self.weights.cpu * cpu_ratio
            + self.weights.memory * mem_ratio
            - self.weights.load * load_ratio
            - self.weights.failure * fail_penalty
    }

    pub fn select_node(&self, request: &PodRequest, nodes: &[NodeSnapshot]) -> Result<Uuid> {
        if nodes.is_empty() {
            return Err(NexaError::SchedulingFailed("no candidate nodes available".into()));
        }

        let mut best_id: Option<Uuid> = None;
        let mut best_score = f64::NEG_INFINITY;

        for node in nodes {
            let score = self.score_node(request, node);
            if score > best_score {
                best_score = score;
                best_id = Some(node.node_id);
            }
        }

        match best_id {
            Some(id) if best_score > f64::NEG_INFINITY => Ok(id),
            _ => Err(NexaError::SchedulingFailed("no node has sufficient resources".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Task 1 tests: types ──

    #[test]
    fn spread_weights_sum_to_one() {
        let w = SchedulerWeights::spread();
        let sum = w.cpu + w.memory + w.load + w.failure;
        assert!((sum - 1.0).abs() < 1e-9, "spread weights sum = {sum}");
    }

    #[test]
    fn binpack_has_negative_resource_weights() {
        let w = SchedulerWeights::binpack();
        assert!(w.cpu < 0.0);
        assert!(w.memory < 0.0);
        assert!(w.load < 0.0);
        assert!(w.failure > 0.0);
    }

    #[test]
    fn default_is_spread() {
        assert_eq!(SchedulerWeights::default(), SchedulerWeights::spread());
    }

    #[test]
    fn pod_request_default_is_best_effort() {
        let req = PodRequest::default();
        assert_eq!(req.cpu_request, 0.0);
        assert_eq!(req.memory_request, 0);
    }

    // ── Task 2 tests: failure_penalty ──

    #[test]
    fn failure_penalty_no_failures_is_zero() {
        let now = Utc::now();
        assert_eq!(failure_penalty(&[], now), 0.0);
    }

    #[test]
    fn failure_penalty_recent_failure_is_high() {
        let now = Utc::now();
        let penalty = failure_penalty(&[now], now);
        assert!((penalty - 1.0).abs() < 1e-9, "penalty = {penalty}");
    }

    #[test]
    fn failure_penalty_old_failure_decays() {
        let now = Utc::now();
        let old = now - chrono::Duration::minutes(30);
        let penalty = failure_penalty(&[old], now);
        assert!(penalty < 0.06, "penalty = {penalty}");
        assert!(penalty > 0.04, "penalty = {penalty}");
    }

    #[test]
    fn failure_penalty_capped_at_one() {
        let now = Utc::now();
        let failures: Vec<DateTime<Utc>> = (0..10).map(|_| now).collect();
        let penalty = failure_penalty(&failures, now);
        assert!((penalty - 1.0).abs() < 1e-9, "penalty = {penalty}");
    }

    #[test]
    fn failure_penalty_multiple_mixed_ages() {
        let now = Utc::now();
        let f1 = now;
        let f2 = now - chrono::Duration::minutes(10);
        let penalty = failure_penalty(&[f1, f2], now);
        assert!((penalty - 1.0).abs() < 1e-9, "penalty = {penalty}");
    }

    #[test]
    fn failure_penalty_single_10min_ago() {
        let now = Utc::now();
        let failure = now - chrono::Duration::minutes(10);
        let penalty = failure_penalty(&[failure], now);
        assert!((penalty - (-1.0_f64).exp()).abs() < 1e-4, "penalty = {penalty}");
    }

    // ── Task 3 tests: score_node ──

    fn make_node(
        cpu_available: f64,
        cpu_total: f64,
        mem_available: u64,
        mem_total: u64,
        running_pods: u32,
        max_pods: u32,
    ) -> NodeSnapshot {
        NodeSnapshot {
            node_id: Uuid::new_v4(),
            cpu_available,
            cpu_total,
            memory_available: mem_available,
            memory_total: mem_total,
            running_pods,
            max_pods,
            recent_failures: vec![],
        }
    }

    #[test]
    fn score_node_fully_idle_spread() {
        let scheduler = WeightedScheduler::new(SchedulerWeights::spread());
        let node = make_node(4.0, 4.0, 8_000_000_000, 8_000_000_000, 0, 100);
        let req = PodRequest { cpu_request: 0.5, memory_request: 512_000_000 };
        let score = scheduler.score_node(&req, &node);
        assert!((score - 0.63385).abs() < 1e-6, "score = {score}");
    }

    #[test]
    fn score_node_half_loaded_spread() {
        let scheduler = WeightedScheduler::new(SchedulerWeights::spread());
        let node = make_node(2.0, 4.0, 4_000_000_000, 8_000_000_000, 50, 100);
        let req = PodRequest { cpu_request: 0.0, memory_request: 0 };
        let score = scheduler.score_node(&req, &node);
        assert!((score - 0.275).abs() < 1e-6, "score = {score}");
    }

    #[test]
    fn score_node_insufficient_cpu_returns_negative_infinity() {
        let scheduler = WeightedScheduler::new(SchedulerWeights::spread());
        let node = make_node(0.2, 4.0, 8_000_000_000, 8_000_000_000, 0, 100);
        let req = PodRequest { cpu_request: 1.0, memory_request: 0 };
        let score = scheduler.score_node(&req, &node);
        assert!(score == f64::NEG_INFINITY, "score = {score}");
    }

    #[test]
    fn score_node_insufficient_memory_returns_negative_infinity() {
        let scheduler = WeightedScheduler::new(SchedulerWeights::spread());
        let node = make_node(4.0, 4.0, 100_000_000, 8_000_000_000, 0, 100);
        let req = PodRequest { cpu_request: 0.0, memory_request: 512_000_000 };
        let score = scheduler.score_node(&req, &node);
        assert!(score == f64::NEG_INFINITY, "score = {score}");
    }

    #[test]
    fn score_node_at_max_pods_returns_negative_infinity() {
        let scheduler = WeightedScheduler::new(SchedulerWeights::spread());
        let node = make_node(4.0, 4.0, 8_000_000_000, 8_000_000_000, 100, 100);
        let req = PodRequest::default();
        let score = scheduler.score_node(&req, &node);
        assert!(score == f64::NEG_INFINITY, "score = {score}");
    }

    #[test]
    fn score_node_with_failures_penalized() {
        let scheduler = WeightedScheduler::new(SchedulerWeights::spread());
        let now = Utc::now();
        let mut node = make_node(4.0, 4.0, 8_000_000_000, 8_000_000_000, 0, 100);
        node.recent_failures = vec![now];
        let req = PodRequest::default();
        let score = scheduler.score_node(&req, &node);
        assert!((score - 0.55).abs() < 1e-6, "score = {score}");
    }

    #[test]
    fn score_node_best_effort_pod_uses_full_capacity() {
        let scheduler = WeightedScheduler::new(SchedulerWeights::spread());
        let node = make_node(4.0, 4.0, 8_000_000_000, 8_000_000_000, 0, 100);
        let req = PodRequest::default();
        let score = scheduler.score_node(&req, &node);
        assert!((score - 0.70).abs() < 1e-6, "score = {score}");
    }

    // ── Task 4 tests: select_node ──

    #[test]
    fn select_node_picks_highest_score() {
        let scheduler = WeightedScheduler::new(SchedulerWeights::spread());
        let req = PodRequest { cpu_request: 0.5, memory_request: 512_000_000 };
        let idle_node = make_node(4.0, 4.0, 8_000_000_000, 8_000_000_000, 0, 100);
        let busy_node = make_node(1.0, 4.0, 2_000_000_000, 8_000_000_000, 80, 100);
        let idle_id = idle_node.node_id;
        let nodes = vec![busy_node, idle_node];
        let selected = scheduler.select_node(&req, &nodes).unwrap();
        assert_eq!(selected, idle_id);
    }

    #[test]
    fn select_node_skips_insufficient_nodes() {
        let scheduler = WeightedScheduler::new(SchedulerWeights::spread());
        let req = PodRequest { cpu_request: 2.0, memory_request: 0 };
        let small = make_node(1.0, 2.0, 8_000_000_000, 8_000_000_000, 0, 100);
        let big = make_node(4.0, 8.0, 8_000_000_000, 8_000_000_000, 0, 100);
        let big_id = big.node_id;
        let nodes = vec![small, big];
        let selected = scheduler.select_node(&req, &nodes).unwrap();
        assert_eq!(selected, big_id);
    }

    #[test]
    fn select_node_empty_list_returns_error() {
        let scheduler = WeightedScheduler::new(SchedulerWeights::spread());
        let req = PodRequest::default();
        let result = scheduler.select_node(&req, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn select_node_all_nodes_insufficient_returns_error() {
        let scheduler = WeightedScheduler::new(SchedulerWeights::spread());
        let req = PodRequest { cpu_request: 8.0, memory_request: 0 };
        let n1 = make_node(2.0, 4.0, 8_000_000_000, 8_000_000_000, 0, 100);
        let n2 = make_node(1.0, 4.0, 8_000_000_000, 8_000_000_000, 0, 100);
        let result = scheduler.select_node(&req, &[n1, n2]);
        assert!(result.is_err());
    }

    #[test]
    fn select_node_prefers_node_without_failures() {
        let scheduler = WeightedScheduler::new(SchedulerWeights::spread());
        let req = PodRequest::default();
        let clean = make_node(4.0, 4.0, 8_000_000_000, 8_000_000_000, 0, 100);
        let clean_id = clean.node_id;
        let mut failed = make_node(4.0, 4.0, 8_000_000_000, 8_000_000_000, 0, 100);
        failed.recent_failures = vec![Utc::now()];
        let nodes = vec![failed, clean];
        let selected = scheduler.select_node(&req, &nodes).unwrap();
        assert_eq!(selected, clean_id);
    }

    #[test]
    fn select_node_single_node_returns_it() {
        let scheduler = WeightedScheduler::new(SchedulerWeights::spread());
        let req = PodRequest::default();
        let node = make_node(4.0, 4.0, 8_000_000_000, 8_000_000_000, 0, 100);
        let node_id = node.node_id;
        let selected = scheduler.select_node(&req, &[node]).unwrap();
        assert_eq!(selected, node_id);
    }

    #[test]
    fn select_node_three_nodes_picks_best() {
        let scheduler = WeightedScheduler::new(SchedulerWeights::spread());
        let req = PodRequest { cpu_request: 1.0, memory_request: 1_000_000_000 };
        let n1 = make_node(2.0, 4.0, 4_000_000_000, 8_000_000_000, 50, 100);
        let n2 = make_node(4.0, 4.0, 8_000_000_000, 8_000_000_000, 10, 100);
        let n3 = make_node(3.0, 4.0, 6_000_000_000, 8_000_000_000, 30, 100);
        let best_id = n2.node_id;
        let nodes = vec![n1, n2, n3];
        let selected = scheduler.select_node(&req, &nodes).unwrap();
        assert_eq!(selected, best_id);
    }

    // ── Task 5 tests: binpack mode ──

    #[test]
    fn binpack_prefers_busy_node() {
        let scheduler = WeightedScheduler::new(SchedulerWeights::binpack());
        let req = PodRequest { cpu_request: 0.5, memory_request: 512_000_000 };
        let idle = make_node(4.0, 4.0, 8_000_000_000, 8_000_000_000, 5, 100);
        let busy = make_node(2.0, 4.0, 3_000_000_000, 8_000_000_000, 60, 100);
        let busy_id = busy.node_id;
        let nodes = vec![idle, busy];
        let selected = scheduler.select_node(&req, &nodes).unwrap();
        assert_eq!(selected, busy_id);
    }

    #[test]
    fn binpack_still_rejects_insufficient_resources() {
        let scheduler = WeightedScheduler::new(SchedulerWeights::binpack());
        let req = PodRequest { cpu_request: 3.0, memory_request: 0 };
        let busy = make_node(1.0, 4.0, 8_000_000_000, 8_000_000_000, 60, 100);
        let idle = make_node(4.0, 4.0, 8_000_000_000, 8_000_000_000, 5, 100);
        let idle_id = idle.node_id;
        let nodes = vec![busy, idle];
        let selected = scheduler.select_node(&req, &nodes).unwrap();
        assert_eq!(selected, idle_id);
    }

    #[test]
    fn binpack_score_lower_for_idle_node() {
        let scheduler = WeightedScheduler::new(SchedulerWeights::binpack());
        let req = PodRequest::default();
        let idle = make_node(4.0, 4.0, 8_000_000_000, 8_000_000_000, 0, 100);
        let busy = make_node(1.0, 4.0, 2_000_000_000, 8_000_000_000, 70, 100);
        let idle_score = scheduler.score_node(&req, &idle);
        let busy_score = scheduler.score_node(&req, &busy);
        assert!(busy_score > idle_score, "binpack should prefer busy: idle={idle_score}, busy={busy_score}");
    }

    #[test]
    fn binpack_still_penalizes_failures() {
        let scheduler = WeightedScheduler::new(SchedulerWeights::binpack());
        let req = PodRequest::default();
        let clean = make_node(1.0, 4.0, 2_000_000_000, 8_000_000_000, 70, 100);
        let clean_id = clean.node_id;
        let mut failed = make_node(1.0, 4.0, 2_000_000_000, 8_000_000_000, 70, 100);
        failed.recent_failures = vec![Utc::now()];
        let nodes = vec![failed, clean];
        let selected = scheduler.select_node(&req, &nodes).unwrap();
        assert_eq!(selected, clean_id);
    }

    #[test]
    fn spread_and_binpack_pick_opposite_nodes() {
        let req = PodRequest { cpu_request: 0.5, memory_request: 512_000_000 };
        let idle = make_node(4.0, 4.0, 8_000_000_000, 8_000_000_000, 5, 100);
        let busy = make_node(2.0, 4.0, 3_000_000_000, 8_000_000_000, 60, 100);
        let idle_id = idle.node_id;
        let busy_id = busy.node_id;
        let spread = WeightedScheduler::new(SchedulerWeights::spread());
        let binpack = WeightedScheduler::new(SchedulerWeights::binpack());
        let nodes = vec![idle.clone(), busy.clone()];
        let spread_pick = spread.select_node(&req, &nodes).unwrap();
        let binpack_pick = binpack.select_node(&req, &nodes).unwrap();
        assert_eq!(spread_pick, idle_id, "spread should pick idle");
        assert_eq!(binpack_pick, busy_id, "binpack should pick busy");
    }
}
