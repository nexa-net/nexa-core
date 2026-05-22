use std::collections::HashMap;
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone)]
pub struct ContainerConfig {
    pub name: String,
    pub image: String,
    pub env: HashMap<String, String>,
    pub ports: Vec<PortBinding>,
    pub volumes: Vec<VolumeBinding>,
    pub labels: HashMap<String, String>,
    pub network: Option<String>,
    pub dns: Vec<String>,
    pub dns_search: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PortBinding {
    pub container_port: u16,
    pub host_port: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct VolumeBinding {
    pub source: String,
    pub target: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: ContainerState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ContainerState {
    Created,
    Running,
    Paused,
    Restarting,
    Removing,
    Exited,
    Dead,
    Unknown,
}

pub type LogStream = Pin<Box<dyn Stream<Item = Result<String>> + Send>>;

pub type EventStream = Pin<Box<dyn Stream<Item = RuntimeEvent> + Send>>;

#[derive(Debug, Clone)]
pub enum RuntimeEvent {
    ContainerDied {
        container_id: String,
        exit_code: i64,
    },
    ContainerStarted {
        container_id: String,
    },
    ContainerOom {
        container_id: String,
    },
}

#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    fn runtime_name(&self) -> &'static str;
    async fn pull_image(&self, image: &str) -> Result<()>;
    async fn create_container(&self, config: &ContainerConfig) -> Result<String>;
    async fn start_container(&self, id: &str) -> Result<()>;
    async fn stop_container(&self, id: &str, timeout_secs: u64) -> Result<()>;
    async fn remove_container(&self, id: &str, force: bool) -> Result<()>;
    async fn inspect_container(&self, id: &str) -> Result<ContainerInfo>;
    async fn logs(&self, id: &str, tail: Option<u64>) -> Result<LogStream>;
    async fn container_exists(&self, name: &str) -> Result<bool>;
    async fn create_network(&self, name: &str) -> Result<String>;
    async fn remove_network(&self, name: &str) -> Result<()>;
    async fn connect_to_network(&self, container_id: &str, network: &str) -> Result<()>;
    async fn container_ip(&self, container_id: &str, network: &str) -> Result<String>;
    async fn events(&self) -> Result<EventStream>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_event_variants_exist() {
        let died = RuntimeEvent::ContainerDied {
            container_id: "abc123".into(),
            exit_code: 137,
        };
        let started = RuntimeEvent::ContainerStarted {
            container_id: "abc123".into(),
        };
        let oom = RuntimeEvent::ContainerOom {
            container_id: "abc123".into(),
        };
        let _ = format!("{died:?}");
        let _ = format!("{started:?}");
        let _ = format!("{oom:?}");
    }

    #[test]
    fn container_config_dns_fields_default_to_empty() {
        let config = ContainerConfig {
            name: "test".into(),
            image: "nginx".into(),
            env: HashMap::new(),
            ports: vec![],
            volumes: vec![],
            labels: HashMap::new(),
            network: None,
            dns: vec![],
            dns_search: vec![],
        };
        assert!(config.dns.is_empty());
        assert!(config.dns_search.is_empty());
    }
}
