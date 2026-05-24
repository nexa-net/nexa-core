use std::any::Any;

pub trait MetricsPort: Send + Sync {
    fn record_http_request(&self, method: &str, path: &str, status: u16, duration_secs: f64);
    fn record_container_event(&self, event: &str);
    fn record_schedule_decision(&self, strategy: &str, duration_secs: f64);
    fn record_deployment_op(&self, op: &str);
    fn set_node_count(&self, count: usize);
    fn set_pod_count(&self, count: usize);
    fn set_deployment_count(&self, count: usize);
    fn record_proxy_request(&self, domain: &str, status: u16, duration_secs: f64);
    fn record_proxy_error(&self, domain: &str, error_type: &str);
    fn as_any(&self) -> &dyn Any;
}

pub struct NoOpMetrics;

impl MetricsPort for NoOpMetrics {
    fn record_http_request(&self, _method: &str, _path: &str, _status: u16, _duration_secs: f64) {}
    fn record_container_event(&self, _event: &str) {}
    fn record_schedule_decision(&self, _strategy: &str, _duration_secs: f64) {}
    fn record_deployment_op(&self, _op: &str) {}
    fn set_node_count(&self, _count: usize) {}
    fn set_pod_count(&self, _count: usize) {}
    fn set_deployment_count(&self, _count: usize) {}
    fn record_proxy_request(&self, _domain: &str, _status: u16, _duration_secs: f64) {}
    fn record_proxy_error(&self, _domain: &str, _error_type: &str) {}
    fn as_any(&self) -> &dyn Any { self }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_metrics_implements_trait() {
        let m: Box<dyn MetricsPort> = Box::new(NoOpMetrics);
        m.record_http_request("GET", "/health", 200, 0.001);
        m.record_container_event("started");
        m.record_schedule_decision("spread", 0.005);
        m.record_deployment_op("deploy");
        m.set_node_count(3);
        m.set_pod_count(10);
        m.set_deployment_count(5);
        m.record_proxy_request("api.example.com", 200, 0.05);
        m.record_proxy_error("api.example.com", "connection_refused");
    }
}
