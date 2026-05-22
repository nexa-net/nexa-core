use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::models::{DeploymentSpec, Node, NodeResources, NodeStatus, Pod};
use crate::error::Result;
use crate::ports::runtime::LogStream;

#[async_trait]
pub trait ClusterTransport: Send + Sync {
    async fn register_node(&self, node: &Node) -> Result<()>;
    async fn heartbeat(
        &self,
        node_id: &Uuid,
        status: &NodeStatus,
        resources: &NodeResources,
    ) -> Result<()>;
    async fn assign_pod(&self, node_id: &Uuid, pod: &Pod, spec: &DeploymentSpec) -> Result<()>;
    async fn stop_pod(&self, node_id: &Uuid, pod_id: &Uuid) -> Result<()>;
    async fn remove_pod(&self, node_id: &Uuid, pod_id: &Uuid) -> Result<()>;
    async fn stream_logs(
        &self,
        node_id: &Uuid,
        pod_id: &Uuid,
        tail: Option<u64>,
    ) -> Result<LogStream>;
}
