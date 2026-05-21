use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: Uuid,
    pub name: String,
    pub address: String,
    pub role: NodeRole,
    pub status: NodeStatus,
    pub resources: NodeResources,
    pub last_heartbeat: DateTime<Utc>,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeRole {
    Master,
    Worker,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    Ready,
    NotReady,
    Draining,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResources {
    pub cpu_cores: f64,
    pub memory_bytes: u64,
    pub cpu_available: f64,
    pub memory_available: u64,
    pub running_pods: u32,
}

impl Node {
    pub fn new(name: String, address: String, role: NodeRole, resources: NodeResources) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            address,
            role,
            status: NodeStatus::Ready,
            resources,
            last_heartbeat: now,
            joined_at: now,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.status == NodeStatus::Ready
    }

    pub fn is_master(&self) -> bool {
        self.role == NodeRole::Master
    }
}

impl fmt::Display for NodeRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeRole::Master => write!(f, "Master"),
            NodeRole::Worker => write!(f, "Worker"),
        }
    }
}

impl fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeStatus::Ready => write!(f, "Ready"),
            NodeStatus::NotReady => write!(f, "NotReady"),
            NodeStatus::Draining => write!(f, "Draining"),
        }
    }
}

impl NodeResources {
    pub fn zero() -> Self {
        Self {
            cpu_cores: 0.0,
            memory_bytes: 0,
            cpu_available: 0.0,
            memory_available: 0,
            running_pods: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_resources() -> NodeResources {
        NodeResources {
            cpu_cores: 4.0,
            memory_bytes: 8_589_934_592,
            cpu_available: 3.5,
            memory_available: 7_000_000_000,
            running_pods: 2,
        }
    }

    #[test]
    fn create_master_node() {
        let node = Node::new(
            "master-1".into(),
            "192.168.1.1:9000".into(),
            NodeRole::Master,
            sample_resources(),
        );
        assert!(node.is_master());
        assert!(node.is_ready());
        assert_eq!(node.name, "master-1");
        assert_eq!(node.address, "192.168.1.1:9000");
    }

    #[test]
    fn create_worker_node() {
        let node = Node::new(
            "worker-1".into(),
            "192.168.1.2:9000".into(),
            NodeRole::Worker,
            sample_resources(),
        );
        assert!(!node.is_master());
        assert!(node.is_ready());
        assert_eq!(node.name, "worker-1");
    }

    #[test]
    fn node_status_display() {
        assert_eq!(NodeStatus::Ready.to_string(), "Ready");
        assert_eq!(NodeStatus::NotReady.to_string(), "NotReady");
        assert_eq!(NodeStatus::Draining.to_string(), "Draining");
    }

    #[test]
    fn node_role_display() {
        assert_eq!(NodeRole::Master.to_string(), "Master");
        assert_eq!(NodeRole::Worker.to_string(), "Worker");
    }

    #[test]
    fn node_serialization_roundtrip() {
        let node = Node::new(
            "test-node".into(),
            "10.0.0.1:9000".into(),
            NodeRole::Worker,
            sample_resources(),
        );
        let json = serde_json::to_string(&node).unwrap();
        let deserialized: Node = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test-node");
        assert_eq!(deserialized.role, NodeRole::Worker);
        assert_eq!(deserialized.status, NodeStatus::Ready);
        assert_eq!(deserialized.resources.cpu_cores, 4.0);
        assert_eq!(deserialized.resources.running_pods, 2);
    }
}
