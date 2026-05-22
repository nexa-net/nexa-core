use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::models::{Deployment, Node, Pod, Project, ProjectStatus};
use crate::error::Result;

#[async_trait]
pub trait StateStore: Send + Sync {
    async fn insert_project(&self, project: &Project) -> Result<()>;
    async fn get_project(&self, name: &str) -> Result<Option<Project>>;
    async fn list_projects(&self) -> Result<Vec<Project>>;
    async fn update_project_status(&self, name: &str, status: ProjectStatus) -> Result<()>;
    async fn delete_project(&self, name: &str) -> Result<()>;

    async fn insert_deployment(&self, deployment: &Deployment) -> Result<()>;
    async fn get_deployment(&self, project: &str, name: &str) -> Result<Option<Deployment>>;
    async fn list_deployments(&self, project: Option<&str>) -> Result<Vec<Deployment>>;
    async fn update_deployment(&self, deployment: &Deployment) -> Result<()>;
    async fn delete_deployment(&self, id: &Uuid) -> Result<()>;

    async fn insert_pod(&self, pod: &Pod) -> Result<()>;
    async fn list_pods(&self, project: Option<&str>) -> Result<Vec<Pod>>;
    async fn update_pod(&self, pod: &Pod) -> Result<()>;
    async fn delete_pod(&self, id: &Uuid) -> Result<()>;
    async fn pods_by_deployment(&self, deployment_id: &Uuid) -> Result<Vec<Pod>>;

    // Node operations
    async fn insert_node(&self, node: &Node) -> Result<()>;
    async fn get_node(&self, id: &Uuid) -> Result<Option<Node>>;
    async fn get_node_by_name(&self, name: &str) -> Result<Option<Node>>;
    async fn list_nodes(&self) -> Result<Vec<Node>>;
    async fn update_node(&self, node: &Node) -> Result<()>;
    async fn delete_node(&self, id: &Uuid) -> Result<()>;

    // Cluster config (key-value)
    async fn get_cluster_config(&self, key: &str) -> Result<Option<String>>;
    async fn set_cluster_config(&self, key: &str, value: &str) -> Result<()>;
}
